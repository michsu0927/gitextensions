fn main() {
    // A Windows program gets a 1 MB main-thread stack by default (Linux: 8 MB). Tauri dispatches
    // every command of the app from there, so reserve more room.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
    tauri_build::build()
}
