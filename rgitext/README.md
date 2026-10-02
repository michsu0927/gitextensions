# rGitExt

A desktop Git client written in **Rust + Tauri 2** with a **Svelte + TypeScript** UI. It is a
from-scratch re-implementation of the workflow of [Git Extensions](https://github.com/gitextensions/gitextensions)
(a C# / WinForms application), using the `git` command line as its engine.

> **Status: work in progress.** Phases 0–2 (foundation, read-only core, write operations) are
> implemented and covered by tests, but the UI has so far only been verified by compiling and
> type-checking, not by hands-on use. See [TODO.md](TODO.md) for what is left.

## Features

**Browse**
- Commit history with a canvas-drawn branch graph, virtual scrolling and paging
- Filter by branch/tag, message, author and file (with rename following)
- Commit details, changed files, diff viewer (inline or side by side, ignore whitespace, context size)
- File tree at any commit, blame, file history

**Commit**
- Stage / unstage / discard whole files, single hunks, or single lines
- Amend, GPG signing, `commit.template`
- Stash (save, apply, pop, drop)

**Branches and history operations**
- Create, rename, delete branches and tags; checkout with local-change handling (including auto-stash)
- Merge (strategies), rebase, **interactive rebase** (reorder, reword, squash, fixup, drop, edit)
- Cherry-pick, revert, reset
- In-progress operation banner with continue / skip / abort
- **Conflict resolver**: per-hunk ours / theirs / both (diff3 aware), manual edit, external merge tool

**Remotes**
- Fetch, pull, push (force-with-lease), clone, init, remote management
- Live progress and cancel; credential prompts via an askpass bridge (`GIT_ASKPASS` / `SSH_ASKPASS`)

## Architecture

```
rgitext/
├─ src/                    Svelte + TypeScript UI
│  ├─ lib/                 api.ts (typed backend calls), actions.ts, operations.ts, network.ts, ...
│  ├─ stores/              Svelte stores per area (repo, history, workdir, branches, ops, ...)
│  └─ components/          Tabs, dialogs, graph / diff / blame / tree views
└─ src-tauri/
   ├─ git-core/            Tauri-independent library: executor, parsers, graph layout,
   │                       patch builder, validation, askpass protocol  (unit + integration tests)
   └─ src/commands/        Thin Tauri command layer, one file per feature area
```

Design decisions:

- **Git CLI, not libgit2.** Behaviour stays identical to what users get on the command line.
  Arguments are passed as an argv vector (never through a shell) and user-supplied values
  (revisions, branch names, URLs, paths) are validated first.
- **Machine-readable output only.** `-z` and explicit separators are used everywhere, so file
  names and commit subjects cannot break parsing.
- **Stateless graph paging.** The graph lane layout is incremental and its small state is passed
  back and forth, so the backend keeps no per-view state.
- **Never block on a terminal.** `GIT_TERMINAL_PROMPT=0`, no console window on Windows, timeouts,
  and cancellation that also stops the processes git started.

## Requirements

- [Git](https://git-scm.com/) on `PATH` (or set the path in Settings)
- To build: Rust (stable), Node.js 20+, and the
  [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform
  (on Windows: WebView2 and the MSVC build tools)

## Build and run

### Local toolchain

```sh
npm install
npm run tauri dev        # development (hot reload, incremental builds)
npm run tauri build      # release build for the current platform
```

### Containers (no local Rust / Node needed)

Podman or Docker builds the app inside an Ubuntu image (`Dockerfile`, driven by `build.sh`) and
writes the results to `dist/`. Run the commands from the `rgitext/` folder. Replace `podman` with
`docker` if you use Docker.

**Windows** (cross-compiled from Linux with `cargo-xwin`; produces `dist/rgitext.exe` and
`dist/nsis/rgitext_*_x64-setup.exe`, unsigned):

```sh
podman compose build build-windows
podman compose run --rm build-windows
```

**Linux** (native build; produces `dist/rgitext` plus the bundles Tauri creates, e.g.
`dist/deb/*.deb`, `dist/appimage/*.AppImage`):

```sh
podman compose build build-linux
podman compose run --rm build-linux
```

**macOS** (experimental cross-compilation with `cargo-zigbuild`; cannot sign or notarize, use a
macOS runner for real releases):

```sh
podman compose build build-macos
podman compose run --rm build-macos
```

Equivalent without compose, for example for Windows:

```sh
podman build --build-arg BUILD_PLATFORM=windows -t rgitext-build-windows .
podman run --rm -e BUILD_PLATFORM=windows -v ./dist:/app/dist rgitext-build-windows
```

Notes:

- The first build is slow (10–25 minutes): the Rust dependencies and, for Windows, the MSVC CRT /
  Windows SDK are downloaded and compiled from scratch, and `run --rm` throws the build cache away
  afterwards. For day-to-day work use `npm run tauri dev` with a local toolchain, and
  `cargo check` / `cargo test` for quick verification.
- A Linux build cannot be run on Windows, and the other way around: build for the platform you
  want to test on.

## Tests and checks

```sh
cd src-tauri/git-core && cargo test      # unit + integration tests against real temporary repositories
cd src-tauri && cargo check              # the Tauri crate
npm run check                            # svelte-check (TypeScript + Svelte)
npm run build                            # production frontend build
```

The integration tests run the real `git` binary, so `git` must be installed.

## Relationship to Git Extensions

rGitExt is an independent project and **is not affiliated with or endorsed by** the Git Extensions
project. Git Extensions served as the functional reference: its feature set, dialogs and
command-line interface define what rGitExt aims to cover, and it is the source of the migration
plan in [MIGRATION_PLAN.md](MIGRATION_PLAN.md).

- Git Extensions: <https://github.com/gitextensions/gitextensions> (GPL-3.0)
- No Git Extensions source code, translations, icons or themes are included in rGitExt; the code
  here was written from scratch against the behaviour of git itself.

> **Licensing note for contributors:** Git Extensions is licensed under the **GPL-3.0**. If you port
> code, translation files (`*.xlf`), icons, or theme files from it, that material stays GPL-3.0 and
> would conflict with this project's MIT license. Re-implement from the behaviour instead, or ask
> first.

## License

[MIT](LICENSE) © 2026 rGitExt contributors

## Troubleshooting

- **The app closes right after starting.** The release build has no console window. It writes a
  log to `%LOCALAPPDATA%\rGitExt\rgitext.log` (Linux: `~/.local/share/rGitExt/rgitext.log`) with
  the startup steps and any panic. When started from a terminal (`.\rgitext.exe`) the output
  also appears in that terminal. If no log file is created at all, the process died before
  `main`, which usually means a missing runtime such as the WebView2 runtime or the Visual C++
  redistributable.
