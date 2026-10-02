// Application logic: loads data from the backend into the stores and implements the
// user actions. Components only call these functions and read the stores.

import { get } from 'svelte/store';
import { api } from './api';
import type { TabName } from './types';

import { activeTab, customGitPath, gitError, gitVersion, isChecking, isSavingSettings, saveGitPath, settingsError, settingsMessage, tempGitPath } from '../stores/app';
import { currentRepoPath, gitStatus, isEditingPath, isLoadingStatus, pathError, statusError, tempRepoPath } from '../stores/repo';
import {
  commitFiles, commitFilesError, commits, commitsError, isLoadingCommitFileDiff, isLoadingCommitFiles,
  isLoadingCommits, selectedCommit, selectedCommitFile, selectedCommitFileDiff,
} from '../stores/history';
import {
  commitFeedback, commitFeedbackError, commitMessage, expandedDirs, isCommitting, isLoadingWorkingFileDiff,
  isLoadingWorkingFiles, selectedStagedPaths, selectedUnstagedPaths, selectedWorkingFile,
  selectedWorkingFileDiff, workingFiles, workingFilesError,
} from '../stores/workdir';
import {
  branchActionError, branches, branchesError, branchMessage, isExecutingBranchAction, isLoadingBranches,
  isLoadingRemoteBranches, isLoadingRemotes, isLoadingStashes, isLoadingTags, remoteBranches,
  remoteBranchesError, remotes, remotesError, sidebarBranchError, sidebarBranchMessage, stashes,
  stashesError, tags, tagsError,
} from '../stores/branches';

const DIALOG_CANCELLED = 'Dialog cancelled by user';
const COMMIT_LOG_LIMIT = 100;

const errorText = (e: unknown): string => String(e);
const repo = (): string => get(currentRepoPath);

// ---------------------------------------------------------------------------
// Git executable
// ---------------------------------------------------------------------------

export async function checkGit(): Promise<void> {
  isChecking.set(true);
  gitError.set('');
  try {
    gitVersion.set(await api.getGitVersion());
  } catch (e) {
    gitVersion.set('');
    gitError.set(errorText(e));
  } finally {
    isChecking.set(false);
  }
}

/** Hands the saved git executable to the backend before any git command runs. */
export async function applySavedGitPath(): Promise<void> {
  const saved = get(customGitPath);
  if (!saved) return;
  try {
    await api.setGitPath(saved);
  } catch (e) {
    // An invalid saved path is ignored: the backend falls back to the git found on PATH.
    console.warn('Saved git path is not usable, falling back to PATH:', e);
  }
}

export async function saveSettings(): Promise<void> {
  isSavingSettings.set(true);
  settingsMessage.set('');
  settingsError.set('');
  try {
    // The backend validates the executable (runs `--version`) and stores it.
    const version = await api.setGitPath(get(tempGitPath) || null);
    customGitPath.set(get(tempGitPath));
    saveGitPath(get(customGitPath));

    gitVersion.set(version);
    gitError.set('');
    settingsMessage.set('Settings saved successfully! Connected to Git.');

    void loadGitStatus();
    reloadHistoryIfActive();
  } catch (e) {
    settingsError.set(`Failed to verify Git path: ${errorText(e)}`);
  } finally {
    isSavingSettings.set(false);
  }
}

export async function browseGitExecutable(): Promise<void> {
  settingsError.set('');
  try {
    const selected = await api.selectGitExecutable();
    if (selected) tempGitPath.set(selected);
  } catch (e) {
    if (errorText(e) !== DIALOG_CANCELLED) settingsError.set(errorText(e));
  }
}

// ---------------------------------------------------------------------------
// Repository / status
// ---------------------------------------------------------------------------

export async function loadGitStatus(): Promise<void> {
  isLoadingStatus.set(true);
  statusError.set('');
  try {
    gitStatus.set(await api.getGitStatus(repo()));
  } catch (e) {
    gitStatus.set(null);
    statusError.set(errorText(e));
  } finally {
    isLoadingStatus.set(false);
  }
}

export async function saveRepoPath(): Promise<void> {
  const path = get(tempRepoPath);
  if (!path.trim()) {
    pathError.set('Path cannot be empty');
    return;
  }
  pathError.set('');
  try {
    // Validate the path as a git repository in the backend.
    await api.verifyGitRepo(path);
    currentRepoPath.set(path);
    isEditingPath.set(false);
    clearBranchFeedback();

    void checkGit();
    void loadGitStatus();
    void loadBranches();
    void loadTags();
    void loadStashes();
    void loadRemotes();
    void loadWorkingFiles();

    if (get(activeTab) === 'history') {
      void loadCommits();
    } else {
      commits.set([]);
      commitsError.set('');
    }
  } catch (e) {
    pathError.set(errorText(e));
  }
}

export async function browseRepoPath(): Promise<void> {
  pathError.set('');
  try {
    const selected = await api.selectDirectory();
    if (selected) {
      tempRepoPath.set(selected);
      // Open straight away when chosen through the browse button.
      await saveRepoPath();
    }
  } catch (e) {
    if (errorText(e) !== DIALOG_CANCELLED) pathError.set(errorText(e));
  }
}

export async function browseRepoPathFromInput(): Promise<void> {
  pathError.set('');
  try {
    const selected = await api.selectDirectory();
    if (selected) tempRepoPath.set(selected);
  } catch (e) {
    if (errorText(e) !== DIALOG_CANCELLED) pathError.set(errorText(e));
  }
}

export function handlePathKeydown(event: KeyboardEvent): void {
  if (event.key === 'Enter') {
    void saveRepoPath();
  } else if (event.key === 'Escape') {
    isEditingPath.set(false);
    pathError.set('');
  }
}

// ---------------------------------------------------------------------------
// Commit history
// ---------------------------------------------------------------------------

export async function loadCommits(): Promise<void> {
  isLoadingCommits.set(true);
  commitsError.set('');
  try {
    commits.set(await api.getCommitLog(repo(), COMMIT_LOG_LIMIT));
  } catch (e) {
    commitsError.set(errorText(e));
    commits.set([]);
  } finally {
    isLoadingCommits.set(false);
  }
}

function reloadHistoryIfActive(): void {
  if (get(activeTab) === 'history') void loadCommits();
}

let lastLoadedCommitHash = '';

async function loadCommitFiles(hash: string): Promise<void> {
  if (hash === lastLoadedCommitHash) return;
  lastLoadedCommitHash = hash;
  isLoadingCommitFiles.set(true);
  commitFilesError.set('');
  selectedCommitFile.set('');
  selectedCommitFileDiff.set('');
  try {
    commitFiles.set(await api.getCommitFiles(repo(), hash));
  } catch (e) {
    commitFiles.set([]);
    commitFilesError.set(errorText(e));
  } finally {
    isLoadingCommitFiles.set(false);
  }
}

async function loadCommitFileDiff(hash: string, filePath: string): Promise<void> {
  isLoadingCommitFileDiff.set(true);
  try {
    selectedCommitFileDiff.set(await api.getCommitFileDiff(repo(), hash, filePath));
  } catch (e) {
    selectedCommitFileDiff.set(`Error loading diff: ${errorText(e)}`);
  } finally {
    isLoadingCommitFileDiff.set(false);
  }
}

/** Reaction to the selected commit changing (wired up with `$:` in App.svelte). */
export function onSelectedCommitChanged(hash: string | undefined): void {
  if (hash) {
    void loadCommitFiles(hash);
  } else {
    lastLoadedCommitHash = '';
    commitFiles.set([]);
    selectedCommitFile.set('');
    selectedCommitFileDiff.set('');
  }
}

/** Reaction to the selected file of a commit changing. */
export function onSelectedCommitFileChanged(hash: string | undefined, filePath: string): void {
  if (hash && filePath) {
    void loadCommitFileDiff(hash, filePath);
  } else {
    selectedCommitFileDiff.set('');
  }
}

// ---------------------------------------------------------------------------
// Working directory / commit
// ---------------------------------------------------------------------------

export async function loadWorkingFiles(): Promise<void> {
  isLoadingWorkingFiles.set(true);
  workingFilesError.set('');
  selectedUnstagedPaths.set([]);
  selectedStagedPaths.set([]);
  try {
    const files = await api.getWorkingDirFiles(repo());
    workingFiles.set(files);
    const selected = get(selectedWorkingFile);
    if (selected) {
      const stillExists = files.some((f) => f.path === selected.path && f.is_staged === selected.is_staged);
      if (!stillExists) {
        selectedWorkingFile.set(null);
        selectedWorkingFileDiff.set('');
      }
    }
  } catch (e) {
    workingFiles.set([]);
    workingFilesError.set(errorText(e));
  } finally {
    isLoadingWorkingFiles.set(false);
  }
}

async function loadWorkingFileDiff(filePath: string, isStaged: boolean): Promise<void> {
  isLoadingWorkingFileDiff.set(true);
  try {
    selectedWorkingFileDiff.set(await api.getWorkingFileDiff(repo(), filePath, isStaged));
  } catch (e) {
    selectedWorkingFileDiff.set(`Error loading diff: ${errorText(e)}`);
  } finally {
    isLoadingWorkingFileDiff.set(false);
  }
}

/** Reaction to the selected working file changing. */
export function onSelectedWorkingFileChanged(file: { path: string; is_staged: boolean } | null): void {
  if (file) {
    void loadWorkingFileDiff(file.path, file.is_staged);
  } else {
    selectedWorkingFileDiff.set('');
  }
}

export async function toggleDir(dirPath: string): Promise<void> {
  if (get(expandedDirs)[dirPath]) {
    // Collapse this directory and everything below it.
    const copy = { ...get(expandedDirs) };
    for (const key of Object.keys(copy)) {
      if (key === dirPath || key.startsWith(dirPath + '/')) delete copy[key];
    }
    expandedDirs.set(copy);
    return;
  }
  expandedDirs.update((d) => ({ ...d, [dirPath]: 'loading' }));
  try {
    const entries = await api.listDirectory(repo(), dirPath);
    expandedDirs.update((d) => ({ ...d, [dirPath]: entries }));
  } catch (e) {
    expandedDirs.update((d) => ({ ...d, [dirPath]: [] }));
    console.error('Failed to list directory:', e);
  }
}

async function changeStaging(paths: string[], stage: boolean): Promise<void> {
  if (paths.length === 0) return;
  try {
    commitFeedback.set('');
    commitFeedbackError.set('');
    if (stage) {
      await api.stageFiles(repo(), paths);
    } else {
      await api.unstageFiles(repo(), paths);
    }
    await loadWorkingFiles();
    await loadGitStatus();
  } catch (e) {
    commitFeedbackError.set(`Failed to ${stage ? 'stage' : 'unstage'} files: ${errorText(e)}`);
  }
}

export const handleStageSelected = (): Promise<void> => changeStaging(get(selectedUnstagedPaths), true);
export const handleUnstageSelected = (): Promise<void> => changeStaging(get(selectedStagedPaths), false);

export async function handleCommit(): Promise<void> {
  const message = get(commitMessage);
  if (!message.trim()) {
    commitFeedbackError.set('Please enter a commit message');
    return;
  }
  if (get(workingFiles).every((f) => !f.is_staged)) {
    commitFeedbackError.set('No staged files to commit. Stage some files first!');
    return;
  }
  isCommitting.set(true);
  commitFeedback.set('');
  commitFeedbackError.set('');
  try {
    const response = await api.commitChanges(repo(), message);
    commitFeedback.set(`Committed successfully!\n${response}`);
    commitMessage.set('');
    await loadWorkingFiles();
    await loadGitStatus();
    await loadBranches();
    if (get(activeTab) === 'history') await loadCommits();
  } catch (e) {
    commitFeedbackError.set(errorText(e));
  } finally {
    isCommitting.set(false);
  }
}

// ---------------------------------------------------------------------------
// Branches, tags, stashes, remotes (loaders)
// ---------------------------------------------------------------------------

/** Clears the result banners of the branches tab. */
export function clearBranchFeedback(): void {
  branchMessage.set('');
  branchActionError.set('');
}

export async function loadBranches(): Promise<void> {
  isLoadingBranches.set(true);
  branchesError.set('');
  try {
    branches.set(await api.getBranches(repo()));
  } catch (e) {
    branches.set([]);
    branchesError.set(errorText(e));
  } finally {
    isLoadingBranches.set(false);
  }
}

export async function loadTags(): Promise<void> {
  isLoadingTags.set(true);
  tagsError.set('');
  try {
    tags.set(await api.getTags(repo()));
  } catch (e) {
    tags.set([]);
    tagsError.set(errorText(e));
  } finally {
    isLoadingTags.set(false);
  }
}

export async function loadStashes(): Promise<void> {
  isLoadingStashes.set(true);
  stashesError.set('');
  try {
    stashes.set(await api.getStashes(repo()));
  } catch (e) {
    stashes.set([]);
    stashesError.set(errorText(e));
  } finally {
    isLoadingStashes.set(false);
  }
}

export async function loadRemotes(): Promise<void> {
  isLoadingRemotes.set(true);
  remotesError.set('');
  try {
    remotes.set(await api.getRemotes(repo()));
  } catch (e) {
    remotes.set([]);
    remotesError.set(errorText(e));
  } finally {
    isLoadingRemotes.set(false);
  }
}

export async function loadRemoteBranches(): Promise<void> {
  isLoadingRemoteBranches.set(true);
  remoteBranchesError.set('');
  try {
    remoteBranches.set(await api.getRemoteBranches(repo()));
  } catch (e) {
    remoteBranches.set([]);
    remoteBranchesError.set(errorText(e));
  } finally {
    isLoadingRemoteBranches.set(false);
  }
}

export function refreshAll(): void {
  clearBranchFeedback();
  void checkGit();
  void loadGitStatus();
  void loadBranches();
  void loadTags();
  void loadStashes();
  void loadRemotes();
  void loadRemoteBranches();
  void loadWorkingFiles();
  reloadHistoryIfActive();
}

export function selectTab(tab: TabName): void {
  activeTab.set(tab);
  if (tab === 'history') {
    void loadCommits();
  } else if (tab === 'dashboard') {
    void loadGitStatus();
  } else if (tab === 'branches') {
    clearBranchFeedback();
    void loadBranches();
    void loadRemoteBranches();
    void loadWorkingFiles();
  }
}

// ---------------------------------------------------------------------------
// Branch / tag / stash / remote actions
// ---------------------------------------------------------------------------

interface BranchActionOptions {
  /** Also report the outcome in the workspace inspector (left sidebar). */
  sidebar: boolean;
  onSuccess: (response: string) => void;
  refresh: () => void;
}

async function runBranchAction(run: () => Promise<string>, opts: BranchActionOptions): Promise<void> {
  isExecutingBranchAction.set(true);
  clearBranchFeedback();
  if (opts.sidebar) {
    sidebarBranchMessage.set('');
    sidebarBranchError.set('');
  }
  try {
    const response = await run();
    opts.onSuccess(response);
    opts.refresh();
  } catch (e) {
    branchActionError.set(errorText(e));
    if (opts.sidebar) sidebarBranchError.set(errorText(e));
  } finally {
    isExecutingBranchAction.set(false);
  }
}

export function checkoutTag(tagName: string): Promise<void> {
  return runBranchAction(() => api.checkoutBranch(repo(), tagName), {
    sidebar: true,
    onSuccess: (response) => {
      branchMessage.set(`Checked out tag successfully:\n${response}`);
      sidebarBranchMessage.set(`Checked out tag ${tagName} successfully.`);
    },
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      void loadBranches();
      void loadTags();
      reloadHistoryIfActive();
    },
  });
}

export function performStashApply(stashIndex: number): Promise<void> {
  return runBranchAction(() => api.applyStash(repo(), stashIndex), {
    sidebar: true,
    onSuccess: (response) => {
      branchMessage.set(response);
      sidebarBranchMessage.set('Stash applied successfully!');
    },
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      void loadStashes();
      reloadHistoryIfActive();
    },
  });
}

export function performCheckout(branchName: string): Promise<void> {
  return runBranchAction(() => api.checkoutBranch(repo(), branchName), {
    sidebar: true,
    onSuccess: (response) => {
      branchMessage.set(`Checked out successfully:\n${response}`);
      sidebarBranchMessage.set(`Checked out ${branchName} successfully.`);
    },
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      void loadBranches();
      reloadHistoryIfActive();
    },
  });
}

export function performCheckoutRemoteBranch(remoteBranch: string, localBranch: string): Promise<void> {
  return runBranchAction(() => api.checkoutRemoteBranch(repo(), remoteBranch, localBranch), {
    sidebar: true,
    onSuccess: (response) => {
      branchMessage.set(`Checked out remote branch successfully:\n${response}`);
      sidebarBranchMessage.set(`Checked out ${remoteBranch} as local ${localBranch} successfully.`);
    },
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      void loadBranches();
      void loadRemoteBranches();
      reloadHistoryIfActive();
    },
  });
}

/** Adds/updates a remote and fetches it. Rejects on failure so the caller can show the error. */
export async function configureAndFetchRemote(remoteName: string, remoteUrl: string): Promise<string> {
  const response = await api.configureAndFetchRemote(repo(), remoteName, remoteUrl);
  await loadRemotes();
  await loadRemoteBranches();
  return response;
}

export async function performMerge(branchName: string): Promise<void> {
  const current = get(gitStatus)?.branch || 'active';
  if (!confirm(`Are you sure you want to merge branch "${branchName}" into the current branch "${current}"?`)) {
    return;
  }
  await runBranchAction(() => api.mergeBranch(repo(), branchName), {
    sidebar: true,
    onSuccess: (response) => {
      branchMessage.set(response);
      sidebarBranchMessage.set(`Merged branch "${branchName}" into "${current}" successfully.`);
    },
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      void loadBranches();
      reloadHistoryIfActive();
    },
  });
}

export function performRebase(branchName: string): Promise<void> {
  return runBranchAction(() => api.rebaseBranch(repo(), branchName), {
    sidebar: false,
    onSuccess: (response) => branchMessage.set(response),
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      reloadHistoryIfActive();
    },
  });
}

export function performCreateBranch(newBranchName: string): Promise<void> {
  return runBranchAction(() => api.createBranch(repo(), newBranchName), {
    sidebar: false,
    onSuccess: (response) => branchMessage.set(response),
    refresh: () => void loadBranches(),
  });
}

export function performResetBranch(target: string, mode: string): Promise<void> {
  return runBranchAction(() => api.resetCurrentBranch(repo(), target, mode), {
    sidebar: false,
    onSuccess: (response) => branchMessage.set(response),
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      void loadWorkingFiles();
      reloadHistoryIfActive();
    },
  });
}

export function performRenameBranch(oldName: string, newName: string): Promise<void> {
  return runBranchAction(() => api.renameBranch(repo(), oldName, newName), {
    sidebar: false,
    onSuccess: (response) => branchMessage.set(response),
    refresh: () => {
      void loadBranches();
      void loadGitStatus();
    },
  });
}

export function performDeleteBranch(branchName: string, force: boolean): Promise<void> {
  return runBranchAction(() => api.deleteBranch(repo(), branchName, force), {
    sidebar: false,
    onSuccess: (response) => branchMessage.set(response),
    refresh: () => {
      void loadBranches();
      void loadGitStatus();
    },
  });
}

export function performPull(): Promise<void> {
  return runBranchAction(() => api.pullChanges(repo()), {
    sidebar: false,
    onSuccess: (response) => branchMessage.set(response),
    refresh: () => {
      void checkGit();
      void loadGitStatus();
      void loadWorkingFiles();
      reloadHistoryIfActive();
    },
  });
}

export function performPush(): Promise<void> {
  return runBranchAction(() => api.pushChanges(repo()), {
    sidebar: false,
    onSuccess: (response) => branchMessage.set(response),
    refresh: () => {},
  });
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

/** Resolves the working directory and loads the initial data. Call once on mount. */
export async function initialize(): Promise<void> {
  try {
    const cwd = await api.getCurrentWorkingDir();
    if (cwd) {
      currentRepoPath.set(cwd);
      tempRepoPath.set(cwd);
    }
  } catch (e) {
    console.error('Failed to resolve current working directory:', e);
  }

  await applySavedGitPath();

  void checkGit();
  void loadGitStatus();
  void loadBranches();
  void loadTags();
  void loadStashes();
  void loadRemotes();
  void loadRemoteBranches();
  void loadWorkingFiles();
}
