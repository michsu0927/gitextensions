# Git Extensions (C#) → rGitExt (Rust + Tauri) 遷移計畫

來源：`E:\gitextensions\src`（.NET + WinForms，約 1,668 個 .cs）。目標：`rgitext`（Tauri 2 + Svelte + Rust）。
策略：**維持呼叫 git CLI**（行為與原版一致），不改用 libgit2；UI 全部以 Web 技術重做；外掛與 i18n 在核心完成後再評估。

## 1. 原專案盤點

| 模組 | 規模 | 說明 | 遷移處理 |
|---|---|---|---|
| GitCommands | 201 檔 / 25.4K 行 | git 核心。`GitModule.cs` 4.1K 行上帝類別；Settings、Patches、Config、Submodules、RevisionReader | 移植為 Rust crate，拆成多個模組 |
| GitExtensions.Extensibility | 114 檔 / 6.1K | 資料模型（ObjectId、GitRevision、GitItemStatus…）與介面 | 資料模型轉 Rust struct；介面丟棄 |
| GitExtUtils / ResourceManager | 66+29 檔 | 多為 WinForms/DPI/翻譯工具 | 只取 `GitArgumentBuilder` 概念、`MruCache` 等；其餘丟棄 |
| GitUI | 約 94K 行、Form 約 97 個 | WinForms UI，含 RevisionGrid、FileViewer、FileStatusList、FormCommit | 以 Svelte 重做 |
| GitExtensions（進入點） | Program.cs 368 行 | 啟動流程、CLI 命令分派（`GitUICommands`） | Rust 端 CLI 解析 + 啟動流程 |
| BugReporter | 約 1.6K 行 | 獨立 exe | 改為前端錯誤頁 + panic hook |
| 外掛 | 約 16 個 | MEF + WinForms | 見第 5 節 |
| 原生 | GitExtensionsShellEx（C++）、GitExtSshAskPass | Explorer 右鍵、SSH 密碼提示 | 後期處理 |

## 2. 現有 rgitext 原型的問題（擴大功能前必須先修）

1. **阻塞**：所有 command 都是同步 `fn` + `.output()`，會卡主執行緒；`pull/push` 缺 `GIT_TERMINAL_PROMPT=0` 會永遠掛住；Windows 缺 `CREATE_NO_WINDOW`。
2. **參數注入**：branch/tag 等名稱未加 `--` 或驗證；`reset mode` 無白名單；`remote_url` 可用 `ext::`；前端可傳入任意 `git_path` 並被執行；`list_directory` 可路徑穿越。
3. **解析脆弱**：log 用 `│` 與自訂字串分隔、`--graph` ASCII；`status` 未用 `-z`；`from_utf8_lossy` 破壞非 UTF-8；未設定 `core.quotepath` 等編碼。
4. **結構**：錯誤皆 `String`；`println!` 偵錯；全域 `APP_HANDLE`；無 `GitService` 狀態；`git_branch.rs` 混雜 tag/stash/remote/pull/push；無測試/CI。
5. **前端**：`App.svelte` 1207 行、無 store、無 TypeScript、無 i18n。
6. **Tauri 安全**：`csp: null`、未收斂 capabilities。

## 3. 目標架構

```
rgitext/
├─ src-tauri/
│  ├─ crates/ (或 src/ 內模組)
│  │  ├─ git-core/   # executor、argument builder、parsers、models（可獨立測試）
│  │  ├─ settings/   # 分層設定、舊 XML 匯入
│  │  └─ graph/      # revision graph lane 佈局（增量、分頁）
│  └─ src/commands/  # 薄薄的 Tauri command 層，依功能拆檔
└─ src/ (Svelte + TypeScript)
   ├─ lib/api.ts, stores/, i18n/, commands registry（含快捷鍵）
   └─ components/ …
```

重點設計：
- `GitExecutor`：`tokio::process` + 逾時/取消 + 殺整個行程樹；參數用 `Vec<OsString>`；固定環境（`LC_ALL`、`GIT_TERMINAL_PROMPT=0`、`GIT_ASKPASS`/`GIT_SSH` 指向本程式）；stdout 串流以 Tauri event 回報長時間操作進度。
- `GitError`（`thiserror`）：`NotFound / NonZeroExit{code,stderr} / Io / Parse`，序列化給前端。
- `GitService`（Tauri `State`）：持有 git 路徑、目前 repo、設定，command 不再重複傳 `repoPath/gitPath`。
- 解析器：全部用 `-z` 或 `%x00/%x1f` 分隔，byte 層解析；以 C# 測試的樣本輸出作 golden test。
- 設定：新格式 JSON/TOML（serde）；`quick-xml` 寫一次性匯入器讀 `GitExtensions.settings` 與最近 repo XML。
- 指令系統：前端集中 command registry（id、預設鍵、scope）+ command palette，取代各 Form 的 `Command` enum 與 172 組預設快捷鍵。

建議 crate：`tokio`、`thiserror`、`tracing`、`gix-config`、`encoding_rs`+`chardetng`、`notify`、`keyring`、`portable-pty`、`which`、`dirs`、`winreg`（cfg windows）、`octocrab`/`reqwest`。

## 4. 功能對照與分階段

### Phase 0 — 地基（先做）
- 重構原型：`GitExecutor`、`GitError`、`GitService`、參數與路徑驗證、async 化、移除 `println!`。
- 解析改 `-z`、加 fixture 測試（臨時 repo）+ CI。
- 前端導入 TypeScript、store、`api.ts`；拆 `App.svelte`。
- Tauri CSP 與 capabilities；git 路徑僅後端保存。

### Phase 1 — 核心唯讀
- `RevisionReader` 等價：log（過濾、搜尋、分頁、encoding）、refs、describe、reflog。
- Revision graph：Rust 端 lane 佈局 + 前端 Canvas 虛擬捲動（參考 GitUI `RevisionGraph`/`Lane`）。
- Diff/檔案檢視：CodeMirror 6 + `@codemirror/merge`；commit 詳情、檔案樹、blame（`--porcelain`）、file history。
- 左側樹：本地/遠端分支、tag、stash、submodule、worktree。

### Phase 2 — 寫入操作（FormCommit 為重點）
- status/stage/unstage、hunk 與逐行 stage（移植 `PatchManager`，大量單測）、commit/amend/template/GPG、stash 完整操作、reset/clean。
- checkout/create/rename/delete branch、tag、merge、rebase（含互動式、continue/skip/abort）、cherry-pick、revert。
- pull/push/fetch/clone/init/remote CRUD：進度串流、認證（askpass 以 Tauri 對話框）、SSH/plink 偵測。
- 衝突解決（三向）、mergetool/difftool。
- 約 60 個「收集參數→執行 git→顯示輸出」的對話框用通用 Process 元件 + 表單 schema 模板化。

### Phase 3 — 外圍與相容
- 設定視窗（約 30 頁）、分層設定與舊設定匯入、主題（沿用 CSS 子集 → CSS 變數）、快捷鍵自訂。
- Dashboard / 最近 repo / 分類。
- CLI 相容：`browse/commit/pull/push/clone/blame/filehistory/fileeditor/mergetool/difftool…`（以 `GitUICommands.RunCommandBasedOnArgument` 清單為驗收）；`fileeditor`、`mergetool`、`difftool` 會被 git 當工具呼叫，必須保留。
- 內嵌終端（xterm.js + portable-pty）、submodule、worktree、bisect、gc、archive、grep、patch（format/apply/am）。
- 單一實例：原版無，可選 `tauri-plugin-single-instance`。

### Phase 4 — 評估項目
- 外掛、i18n、Shell 整合（Explorer 右鍵、JumpList）、安裝程式（WiX → Tauri bundler / NSIS）、自動更新、telemetry。

## 5. 外掛（Phase 4 評估）

放棄 .NET 二進位相容，改為內建 Rust 模組 + Svelte 面板；若要第三方擴充，另定 JSON-RPC/子行程協定。

| 難度 | 外掛 |
|---|---|
| easy | AutoCompileSubmodules、BackgroundFetch（tokio 計時器）、CreateLocalBranches、ProxySwitcher |
| med | FindLargeFiles、DeleteUnusedBranches、ReleaseNotesGenerator、Gource、GitHub3（octocrab）、Statistics（圖表改前端） |
| med–hard | BuildServerIntegration（AppVeyor/AzureDevOps/GitHubActions/Gitlab/Jenkins/TeamCity，六家 API + 認證） |
| hard | GitUIPluginInterfaces（MEF 契約，需整個重設計） |

## 6. i18n

- 現況：XLIFF（`src/app/GitUI/Translation/*.xlf`），主程式 + `.Plugins.xlf`，目前有 .xlf 的約 13 種語言（含 Traditional/Simplified Chinese、Japanese、Korean 等），其餘語言在 Transifex。
- key 為 `FormName.control.Text` 形式；計畫寫一次性腳本轉 JSON，前端用 i18next 或 paraglide。UI 從一開始就不要寫死字串。

## 7. 風險與注意事項

- 工作量：GitUI 約 94K 行；最難四塊為 RevisionGrid（含 graph）、FileViewer/diff、FileStatusList、FormCommit，約占 UI 邏輯一半以上。
- `GitModule` 4K 行不要照搬，依功能拆為 trait/模組。
- 編碼（`.gitattributes working-tree-encoding`、BOM、`i18n.commitEncoding`）、Windows 路徑（UNC、8.3、WSL、msys）、`index.lock`、依 git 版本判斷可用參數。
- 設定相容：Distributed（`.gitext`）、Local、Global、Effective 四層 + git config。
- 驗收：以 C# 專案的測試樣本做 golden test；CLI 命令清單作相容性檢查。

## 8. 進度

### Phase 0 — 完成
`git-core` crate（executor / error / validate / parse）、Tauri command 拆檔、前端 TypeScript + stores + actions。

### Phase 1 — 唯讀核心（進行中）
已完成：
- `graph.rs`：增量 lane 佈局，狀態可序列化，分頁時由前端回傳 `graph_state`，後端無狀態。
- `revisions.rs`：分頁 `git log`（topo-order、`--decorate=full`）、scope / 搜尋 / 作者 / 路徑（可 `--follow`）/ first-parent 篩選、commit details。
- `diff.rs`：unified diff → 結構化（hunk、行號、rename、binary、no-EOL）。
- `blame.rs`、`tree.rs`：`blame --porcelain`、`ls-tree`、檔案內容（二進位偵測、2 MB 上限）。
- 前端：虛擬捲動 `RevisionGrid`（Canvas 繪製圖形）、`DiffView`（inline / 並排、忽略空白、context 行數）、`CommitDetailsPanel`、`TreeBrowser` + `FileViewer`、`BlameView`、檔案歷史篩選。

與原計畫的差異：
- Diff 檢視器先用自製渲染（git 已經給出 patch，只需呈現），沒有引入 CodeMirror；CodeMirror 留到需要「編輯 / 大檔語法高亮」時（FormEditor、commit message）再導入。

未完成：
- 非 UTF-8 編碼處理（`i18n.logOutputEncoding`、`working-tree-encoding`）。
- 左側 repo 樹點選分支 / tag 後跳到對應 commit。
- 圖形欄寬度可調整、超過 14 條 lane 的縮排顯示。
- Combined diff（`--cc`）顯示；目前合併 commit 以第一個 parent 為基準。
