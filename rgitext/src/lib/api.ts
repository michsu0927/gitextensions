import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { beginLog, finishLog } from '../stores/consoleLog';
import type {
  BlameLine, BranchInfo, CommitDetails, CommitFile, CommitOptions, ControlAction, LocalChangesMode, MergeOptions,
  PickOptions, RepoState, TagInfo, TodoCandidate, TodoItem, DiffFile, DiffOptions, DirEntry, DiscardTarget, FileContent,
  GitStatus, GraphState, HunkSelection, RevisionFilter, RevisionPage, SelectionAction, StashEntry, TreeEntry,
  WorkingFile,
} from './types';

type Args = Record<string, unknown>;

/** Calls a backend command and records it in the console drawer. */
async function call<T>(command: string, args: Args = {}): Promise<T> {
  const logId = beginLog(command, args);
  try {
    const result = await tauriInvoke<T>(command, args);
    finishLog(logId, { result });
    return result;
  } catch (err) {
    finishLog(logId, { error: String(err) });
    throw err;
  }
}

// One typed function per backend command. The git executable path is stored in the
// backend (`setGitPath`) and is never sent with individual calls.
export const api = {
  // system
  getGitVersion: () => call<string>('get_git_version'),
  setGitPath: (gitPath: string | null) => call<string>('set_git_path', { gitPath }),
  verifyGitRepo: (repoPath: string) => call<string>('verify_git_repo', { repoPath }),
  selectDirectory: () => call<string>('select_directory'),
  selectGitExecutable: () => call<string>('select_git_executable'),
  getCurrentWorkingDir: () => call<string>('get_current_working_dir'),

  // repository
  getGitStatus: (repoPath: string) => call<GitStatus>('get_git_status', { repoPath }),
  scanForRepos: (basePath: string) => call<string[]>('scan_for_repos', { basePath }),
  listDirectory: (repoPath: string, relativeDir: string) =>
    call<DirEntry[]>('list_directory', { repoPath, relativeDir }),

  // history
  getRevisions: (
    repoPath: string,
    filter: RevisionFilter,
    skip: number,
    limit: number,
    graphState: GraphState | null,
  ) =>
    call<RevisionPage>('get_revisions', {
      repoPath,
      query: { ...filter, skip, limit },
      graphState,
    }),
  getCommitDetails: (repoPath: string, hash: string) =>
    call<CommitDetails>('get_commit_details', { repoPath, hash }),
  getCommitFiles: (repoPath: string, commitHash: string) =>
    call<CommitFile[]>('get_commit_files', { repoPath, commitHash }),
  getCommitDiff: (repoPath: string, hash: string, filePath: string | null, options: DiffOptions) =>
    call<DiffFile[]>('get_commit_diff', { repoPath, hash, filePath, options }),
  getBlame: (repoPath: string, filePath: string, rev: string | null, ignoreWhitespace: boolean) =>
    call<BlameLine[]>('get_blame', { repoPath, filePath, rev, ignoreWhitespace }),
  getTree: (repoPath: string, rev: string, dir: string) =>
    call<TreeEntry[]>('get_tree', { repoPath, rev, dir }),
  getFileContent: (repoPath: string, path: string, rev: string | null) =>
    call<FileContent>('get_file_content', { repoPath, path, rev }),

  // working directory
  getWorkingDirFiles: (repoPath: string) => call<WorkingFile[]>('get_working_dir_files', { repoPath }),
  getWorkingDiff: (repoPath: string, filePath: string, isStaged: boolean, options: DiffOptions) =>
    call<DiffFile[]>('get_working_diff', { repoPath, filePath, isStaged, options }),
  stageFiles: (repoPath: string, filePaths: string[]) =>
    call<string>('stage_files', { repoPath, filePaths }),
  unstageFiles: (repoPath: string, filePaths: string[]) =>
    call<string>('unstage_files', { repoPath, filePaths }),
  commitChanges: (repoPath: string, message: string, options: Partial<CommitOptions>) =>
    call<string>('commit_changes', { repoPath, message, options }),
  getCommitTemplate: (repoPath: string) => call<string | null>('get_commit_template', { repoPath }),
  getLastCommitMessage: (repoPath: string) => call<string>('get_last_commit_message', { repoPath }),
  stageAll: (repoPath: string) => call<string>('stage_all', { repoPath }),
  unstageAll: (repoPath: string) => call<string>('unstage_all', { repoPath }),
  discardChanges: (repoPath: string, targets: DiscardTarget[]) =>
    call<string>('discard_changes', { repoPath, targets }),
  applySelection: (
    repoPath: string,
    filePath: string,
    action: SelectionAction,
    selections: HunkSelection[],
    untracked: boolean,
    options: DiffOptions,
  ) => call<string>('apply_selection', { repoPath, filePath, action, selections, untracked, options }),

  // branches
  getBranches: (repoPath: string) => call<string[]>('get_git_branches', { repoPath }),
  listBranches: (repoPath: string) => call<BranchInfo[]>('list_branches', { repoPath }),
  checkoutBranch: (repoPath: string, branchName: string, localChanges?: LocalChangesMode) =>
    call<string>('checkout_branch', { repoPath, branchName, options: { localChanges } }),
  mergeBranch: (repoPath: string, branchName: string, options?: MergeOptions) =>
    call<string>('merge_branch', { repoPath, branchName, options }),
  rebaseBranch: (repoPath: string, branchName: string, autostash = false) =>
    call<string>('rebase_branch', { repoPath, branchName, options: { autostash } }),
  createBranch: (repoPath: string, newBranchName: string, startPoint?: string, checkout = false) =>
    call<string>('create_branch', { repoPath, newBranchName, startPoint, checkout }),
  resetCurrentBranch: (repoPath: string, target: string, mode: string) =>
    call<string>('reset_current_branch', { repoPath, target, mode }),
  renameBranch: (repoPath: string, oldName: string, newName: string) =>
    call<string>('rename_branch', { repoPath, oldName, newName }),
  deleteBranch: (repoPath: string, branchName: string, force: boolean) =>
    call<string>('delete_branch', { repoPath, branchName, force }),
  setUpstream: (repoPath: string, branchName: string, upstream: string | null) =>
    call<string>('set_upstream', { repoPath, branchName, upstream }),

  // in-progress operations, cherry-pick, revert, interactive rebase
  getRepoState: (repoPath: string) => call<RepoState>('get_repo_state', { repoPath }),
  operationControl: (repoPath: string, action: ControlAction) =>
    call<string>('operation_control', { repoPath, action }),
  cherryPick: (repoPath: string, commits: string[], options?: PickOptions) =>
    call<string>('cherry_pick', { repoPath, commits, options }),
  revertCommits: (repoPath: string, commits: string[], options?: PickOptions) =>
    call<string>('revert_commits', { repoPath, commits, options }),
  getRebaseTodo: (repoPath: string, onto: string) =>
    call<TodoCandidate[]>('get_rebase_todo', { repoPath, onto }),
  rebaseInteractive: (repoPath: string, onto: string, items: TodoItem[], autostash = false) =>
    call<string>('rebase_interactive', { repoPath, onto, items, autostash }),

  // tags, stashes
  getTags: (repoPath: string) => call<string[]>('get_git_tags', { repoPath }),
  listTags: (repoPath: string) => call<TagInfo[]>('list_tags', { repoPath }),
  createTag: (
    repoPath: string,
    name: string,
    target?: string,
    message?: string,
    sign = false,
    force = false,
  ) => call<string>('create_tag', { repoPath, name, target, message, sign, force }),
  deleteTag: (repoPath: string, name: string) => call<string>('delete_tag', { repoPath, name }),
  deleteRemoteRef: (repoPath: string, remote: string, kind: 'branch' | 'tag', name: string) =>
    call<string>('delete_remote_ref', { repoPath, remote, kind, name }),
  getStashes: (repoPath: string) => call<string[]>('get_git_stashes', { repoPath }),
  listStashes: (repoPath: string) => call<StashEntry[]>('list_stashes', { repoPath }),
  stashSave: (repoPath: string, message: string | null, includeUntracked: boolean, keepIndex: boolean) =>
    call<string>('stash_save', { repoPath, message, includeUntracked, keepIndex }),
  applyStash: (repoPath: string, stashIndex: number) =>
    call<string>('apply_stash', { repoPath, stashIndex }),
  popStash: (repoPath: string, stashIndex: number) => call<string>('stash_pop', { repoPath, stashIndex }),
  dropStash: (repoPath: string, stashIndex: number) => call<string>('stash_drop', { repoPath, stashIndex }),

  // remotes
  getRemotes: (repoPath: string) => call<string[]>('get_git_remotes', { repoPath }),
  getRemoteBranches: (repoPath: string) => call<string[]>('get_git_remote_branches', { repoPath }),
  checkoutRemoteBranch: (repoPath: string, remoteBranch: string, localBranch: string) =>
    call<string>('checkout_remote_branch', { repoPath, remoteBranch, localBranch }),
  configureAndFetchRemote: (repoPath: string, remoteName: string, remoteUrl: string) =>
    call<string>('configure_and_fetch_remote', { repoPath, remoteName, remoteUrl }),
  pullChanges: (repoPath: string) => call<string>('pull_changes', { repoPath }),
  pushChanges: (repoPath: string) => call<string>('push_changes', { repoPath }),
};
