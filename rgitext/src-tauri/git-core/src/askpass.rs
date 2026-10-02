//! Credential prompts for git (`GIT_ASKPASS` / `SSH_ASKPASS`).
//!
//! git (and ssh) run the askpass program with the prompt as the only argument and read the
//! answer from its stdout. rGitExt uses its own executable as that program: it forwards the
//! prompt over a loopback socket to the running app, which asks the user and sends the answer
//! back. Requests carry a random token so that other local processes cannot use the socket.

use std::future::Future;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use tokio::net::TcpListener;

/// Environment variables that tell the askpass client where to connect.
pub const ENV_ADDR: &str = "RGITEXT_ASKPASS_ADDR";
pub const ENV_TOKEN: &str = "RGITEXT_ASKPASS_TOKEN";

const MAX_REQUEST_BYTES: usize = 64 * 1024;
/// The user may take a while to find the password.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    token: String,
    prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    /// `None` when the user cancelled or the request was refused.
    secret: Option<String>,
}

/// Answers a prompt (`None` = cancelled).
pub type Handler = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> + Send + Sync>;

/// A random hex token for authenticating askpass requests.
pub fn new_token() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::getrandom(&mut bytes).is_err() {
        // Extremely unlikely; fall back to something unguessable enough for a loopback socket.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        bytes.copy_from_slice(&nanos.to_le_bytes());
    }
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn same(a: &str, b: &str) -> bool {
    // Compare without an early exit so the token cannot be probed byte by byte.
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Binds a loopback listener on a free port.
pub async fn bind() -> std::io::Result<(TcpListener, SocketAddr)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    Ok((listener, addr))
}

/// Accepts askpass connections forever.
pub async fn serve(listener: TcpListener, token: String, handler: Handler) {
    loop {
        let Ok((stream, _)) = listener.accept().await else { continue };
        let token = token.clone();
        let handler = handler.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(stream, &token, handler).await {
                tracing::debug!("askpass connection failed: {}", e);
            }
        });
    }
}

async fn handle(stream: tokio::net::TcpStream, token: &str, handler: Handler) -> std::io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = AsyncBufReader::new(read_half.take(MAX_REQUEST_BYTES as u64));
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let secret = match serde_json::from_str::<Request>(&line) {
        Ok(req) if same(&req.token, token) => handler(req.prompt).await,
        _ => None,
    };

    let mut reply = serde_json::to_string(&Response { secret }).unwrap_or_else(|_| "{\"secret\":null}".into());
    reply.push('\n');
    write_half.write_all(reply.as_bytes()).await?;
    write_half.flush().await
}

/// Blocking client: sends the prompt and returns the answer.
pub fn client_request(addr: &str, token: &str, prompt: &str) -> std::io::Result<Option<String>> {
    let stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(CLIENT_TIMEOUT))?;
    let mut writer = stream.try_clone()?;
    let mut request = serde_json::to_string(&Request { token: token.to_string(), prompt: prompt.to_string() })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    request.push('\n');
    writer.write_all(request.as_bytes())?;
    writer.flush()?;

    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let response: Response =
        serde_json::from_str(&line).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(response.secret)
}

/// Entry point of the askpass program: `rgitext <prompt>` with the environment set by the app.
/// Prints the answer on stdout and returns the process exit code.
pub fn run_client(args: &[String]) -> i32 {
    let (Some(addr), Some(token)) = (std::env::var_os(ENV_ADDR), std::env::var_os(ENV_TOKEN)) else {
        return 2;
    };
    let prompt = args.get(1).cloned().unwrap_or_default();
    match client_request(&addr.to_string_lossy(), &token.to_string_lossy(), &prompt) {
        Ok(Some(secret)) => {
            let mut out = std::io::stdout();
            let ok = out.write_all(secret.as_bytes()).is_ok() && out.write_all(b"\n").is_ok() && out.flush().is_ok();
            if ok {
                0
            } else {
                1
            }
        }
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> Handler {
        Arc::new(|prompt: String| {
            Box::pin(async move {
                if prompt.contains("Password") {
                    Some("s3cret".to_string())
                } else {
                    None
                }
            })
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn prompt_roundtrip_and_token_check() {
        let (listener, addr) = bind().await.unwrap();
        let token = new_token();
        tokio::spawn(serve(listener, token.clone(), handler()));
        let addr = addr.to_string();

        let t = token.clone();
        let a = addr.clone();
        let answer = tokio::task::spawn_blocking(move || client_request(&a, &t, "Password for 'https://x': "))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(answer.as_deref(), Some("s3cret"));

        // The handler can decline (user pressed cancel).
        let t = token.clone();
        let a = addr.clone();
        let declined = tokio::task::spawn_blocking(move || client_request(&a, &t, "Username: "))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(declined, None);

        // A wrong token never reaches the handler.
        let a = addr.clone();
        let refused = tokio::task::spawn_blocking(move || client_request(&a, "wrong", "Password: "))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(refused, None);
    }

    #[test]
    fn tokens_are_random_hex() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        assert!(same("abc", "abc") && !same("abc", "abd") && !same("abc", "ab"));
    }

    #[test]
    fn client_without_environment_fails() {
        std::env::remove_var(ENV_ADDR);
        assert_eq!(run_client(&["askpass".to_string(), "Password: ".to_string()]), 2);
    }
}
