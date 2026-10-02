# TODO

Reference for the feature set: [Git Extensions](https://github.com/gitextensions/gitextensions)
(see [MIGRATION_PLAN.md](MIGRATION_PLAN.md) for the full inventory and architecture notes).

Phases 0–2 are done (see the progress section of the plan). Everything below is open.

## Verify first (before building more)

- [ ] Run `dist\rgitext.exe` on Windows and click through Phases 0–2: history graph, diff / blame / tree,
      line-level staging, commit / amend, branch and tag operations, merge / rebase / cherry-pick,
      conflict resolver, fetch / pull / push with progress and cancel.
- [ ] Check the CSP against the real app (Google Fonts are allowed; confirm nothing else is blocked).
- [ ] Credentials on Windows: askpass was only tested at protocol level on Linux. Verify that git can read the
      answer from the GUI-subsystem executable's stdout, with HTTPS (with and without Git Credential Manager)
      and with an SSH key that has a passphrase.
- [ ] Interactive rebase `reword` relies on `cp` / `sh` from Git for Windows: verify on Windows.
- [ ] Large repositories: history paging, graph drawing and blame performance.

## Phase 3 — Surroundings and compatibility

**Settings**
- [ ] Settings dialog (Git Extensions has about 30 pages: general, appearance, colors, hotkeys, diff / blame
      viewer, commit dialog, confirmations, SSH, scripts, ...).
- [ ] Layered settings: distributed (`.gitext` in the repo), local, global, effective; plus git config editing.
- [ ] One-off importer for the old `GitExtensions.settings` XML and the recent-repositories XML.
- [ ] Persist UI state (window size, selected tab, diff mode, column widths) with serde JSON/TOML.

**Look and feel**
- [ ] Themes: reuse the Git Extensions CSS subset format mapped to CSS variables; light / dark / follow OS.
- [ ] Central command registry with customizable hotkeys (defaults from Git Extensions' ~170 shortcuts) and a
      command palette.
- [ ] Resizable graph column and panels; graph with more than 14 lanes.

**Navigation**
- [ ] Dashboard: recent repositories, categories / favourites, "open", "clone", "init".
- [ ] Multiple repositories (tabs) and quick switching.
- [ ] Left repository tree (local / remote branches, tags, stashes, submodules, worktrees) that jumps to the
      commit on click.
- [ ] Go to commit (by hash / ref), commit search, revision filters by date range.

**Command-line compatibility** (acceptance list: Git Extensions' `GitUICommands.RunCommandBasedOnArgument`)
- [ ] `browse`, `commit`, `pull`, `push`, `clone`, `init`, `merge`, `rebase`, `checkout`, `cherry`, `revert`,
      `reset`, `blame`, `filehistory`, `viewdiff`, `viewpatch`, `applypatch`, `formatpatch`, `stash`, `remotes`,
      `settings`, `searchfile`, ...
- [ ] Tools invoked **by git**: `fileeditor` (as `core.editor` for commit messages and rebase todo),
      `mergetool`, `difftool` — keep exit codes and comment-character handling compatible.
- [ ] Optional single-instance behaviour (`tauri-plugin-single-instance`); the original has none.

**More git features**
- [ ] Embedded terminal (xterm.js + `portable-pty`).
- [ ] Submodules (add, update, sync, status), worktrees (list, create, remove).
- [ ] Bisect, gc, clean, archive, grep / pickaxe search, notes, reflog browser.
- [ ] Patches: `format-patch`, `apply`, `am`, view patch.
- [ ] Difftool for working tree / commit files; combined diff (`--cc`) for merge commits.
- [ ] `.gitignore` / `.gitattributes` / `.mailmap` editors, sparse checkout, LFS awareness.
- [ ] Multi-select commits for cherry-pick / revert; compare two arbitrary commits.
- [ ] Non-UTF-8 content: `i18n.logOutputEncoding`, `working-tree-encoding`, BOM / `gui.encoding`.
- [ ] Syntax highlighting and editing for file viewer / commit message (CodeMirror 6).
- [ ] Better branch UI: ahead / behind and upstream shown in the Branches tab (`list_branches` already
      returns them), set-upstream and remote-branch deletion from the UI.
- [ ] Tag UI: tag list with annotated / lightweight info, push tag from the UI (`push_tag` exists).
- [ ] Spell checking for commit messages; Conventional Commit helpers.

## Phase 4 — Evaluate later

**Plugins** (the original uses MEF + WinForms; binary compatibility is dropped)
- [ ] Decide the extension model: built-in Rust modules + Svelte panels, optionally a JSON-RPC / subprocess
      protocol for third-party plugins.
- [ ] Easy: BackgroundFetch, ProxySwitcher, CreateLocalBranches, AutoCompileSubmodules.
- [ ] Medium: FindLargeFiles, DeleteUnusedBranches, ReleaseNotesGenerator, Gource launcher, GitHub
      (pull requests, fork and clone), statistics / impact charts.
- [ ] Medium–hard: build server integration (AppVeyor, Azure DevOps, GitHub Actions, GitLab, Jenkins,
      TeamCity) showing CI status in the history.

**Internationalization**
- [ ] i18n framework in the UI (i18next or paraglide); no hard-coded strings in new code.
- [ ] One-off converter from the Git Extensions XLIFF files — **mind the license**: those translations are
      GPL-3.0; either get agreement from the translators / project or translate afresh.
- [ ] Languages to start with: English, Traditional Chinese.

**Platform integration and distribution**
- [ ] Windows Explorer context menu (shell extension) and JumpList.
- [ ] Installer: Tauri bundler / NSIS (currently produced by the container build, unsigned); code signing.
- [ ] Auto-update, crash reporting (replaces BugReporter), optional telemetry (opt-in only).
- [ ] macOS and Linux builds in CI (cross-compiling macOS from Linux cannot sign / notarize).

## Engineering

- [ ] CI: `cargo test`, `cargo clippy`, `svelte-check`, frontend build on every push.
- [ ] Frontend unit tests (e.g. vitest) for pure logic such as `lib/conflicts.ts`; end-to-end tests with
      `tauri-driver`.
- [ ] Accessibility pass (keyboard navigation in dialogs and lists, focus management, contrast).
- [ ] Split `lib/actions.ts` further by area; reduce prop-drilling in the older components
      (`Sidebar`, `WorkspaceInspector`, `Header`, `SettingsTab`) by moving them to the stores.
- [ ] Remove leftovers of the prototype: unused `greet`, `list_directory`, `scan_for_repos` commands
      if they stay unused.
- [ ] Add a `.gitattributes` to normalise line endings (the repository currently warns about LF → CRLF).
