import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { beginLog, finishLog } from '../stores/consoleLog';
import type { CommitFile, CommitItem, DirEntry, GitStatus, WorkingFile } from './types';

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
  getCommitLog: (repoPath: string, limit: number) =>
    call<CommitItem[]>('get_commit_log', { repoPath, limit }),
  getCommitFiles: (repoPath: string, commitHash: string) =>
    call<CommitFile[]>('get_commit_files', { repoPath, commitHash }),
  getCommitFileDiff: (repoPath: string, commitHash: string, filePath: string) =>
    call<string>('get_commit_file_diff', { repoPath, commitHash, filePath }),

  // working directory
  getWorkingDirFiles: (repoPath: string) => call<WorkingFile[]>('get_working_dir_files', { repoPath }),
  getWorkingFileDiff: (repoPath: string, filePath: string, isStaged: boolean) =>
    call<string>('get_working_file_diff', { repoPath, filePath, isStaged }),
  stageFiles: (repoPath: string, filePaths: string[]) =>
    call<string>('stage_files', { repoPath, filePaths }),
  unstageFiles: (repoPath: string, filePaths: string[]) =>
    call<string>('unstage_files', { repoPath, filePaths }),
  commitChanges: (repoPath: string, message: string) =>
    call<string>('commit_changes', { repoPath, message }),

  // branches
  getBranches: (repoPath: string) => call<string[]>('get_git_branches', { repoPath }),
  checkoutBranch: (repoPath: string, branchName: string) =>
    call<string>('checkout_branch', { repoPath, branchName }),
  mergeBranch: (repoPath: string, branchName: string) =>
    call<string>('merge_branch', { repoPath, branchName }),
  rebaseBranch: (repoPath: string, branchName: string) =>
    call<string>('rebase_branch', { repoPath, branchName }),
  createBranch: (repoPath: string, newBranchName: string) =>
    call<string>('create_branch', { repoPath, newBranchName }),
  resetCurrentBranch: (repoPath: string, target: string, mode: string) =>
    call<string>('reset_current_branch', { repoPath, target, mode }),
  renameBranch: (repoPath: string, oldName: string, newName: string) =>
    call<string>('rename_branch', { repoPath, oldName, newName }),
  deleteBranch: (repoPath: string, branchName: string, force: boolean) =>
    call<string>('delete_branch', { repoPath, branchName, force }),

  // tags, stashes
  getTags: (repoPath: string) => call<string[]>('get_git_tags', { repoPath }),
  getStashes: (repoPath: string) => call<string[]>('get_git_stashes', { repoPath }),
  applyStash: (repoPath: string, stashIndex: number) =>
    call<string>('apply_stash', { repoPath, stashIndex }),

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
