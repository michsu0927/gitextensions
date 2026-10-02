// Application logic: loads data from the backend into the stores and implements the
// user actions. Components only call these functions and read the stores.

import { get } from 'svelte/store';
import { api } from './api';
import type { DiffOptions, DiscardTarget, HunkSelection, RevisionFilter, SelectionAction, TabName } from './types';

import { activeTab, customGitPath, gitError, gitVersion, isChecking, isSavingSettings, saveGitPath, settingsError, settingsMessage, tempGitPath } from '../stores/app';
import { repoState } from '../stores/ops';
import { currentRepoPath, gitStatus, isEditingPath, isLoadingStatus, pathError, statusError, tempRepoPath } from '../stores/repo';
import {
  commitDetails, commitDiff, commitFiles, commitFilesError, commitsError, DEFAULT_FILTER, diffOptions, graphState,
  hasMoreRevisions, isLoadingCommitDetails, isLoadingCommitFileDiff, isLoadingCommitFiles, isLoadingCommits,
  isLoadingMoreCommits, revisionFilter, revisionRows, selectedCommitFile, selectedHash,
} from '../stores/history';
import {
  amendCommit, commitFeedback, commitFeedbackError, commitMessage, expandedDirs, isCommitting,
  isLoadingStashEntries, isLoadingWorkingFileDiff, isLoadingWorkingFiles, selectedStagedPaths,
  selectedUnstagedPaths, selectedWorkingFile, selectedWorkingFileDiff, signCommit, stashEntries,
  stashIncludeUntracked, stashMessage, workingFiles, workingFilesError,
} from '../stores/workdir';
import {
  branchActionError, branches, branchesError, branchMessage, isExecutingBranchAction, isLoadingBranches,
  isLoadingRemoteBranches, isLoadingRemotes, isLoadingStashes, isLoadingTags, remoteBranches,
  remoteBranchesError, remotes, remotesError, sidebarBranchError, sidebarBranchMessage, stashes,
  stashesError, tags, tagsError,
} from '../stores/branches';

const DIALOG_CANCELLED = 'Dialog cancelled by user';

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
// In-progress operation (merge / rebase / cherry-pick / revert)
// ---------------------------------------------------------------------------

export async function loadRepoState(): Promise<void> {
  try {
    repoState.set(await api.getRepoState(repo()));
  } catch {
    // Not a repository (yet): nothing is in progress.
    repoState.set({ operation: null, conflicted: [] });
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
  // Almost every operation ends with a status reload: keep the "in progress" banner in sync.
  void loadRepoState();
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
      revisionRows.set([]);
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

const PAGE_SIZE = 200;
// Incremented for every reload so that slow, outdated responses can be discarded.
let revisionRequestId = 0;

export async function loadCommits(): Promise<void> {
  const requestId = ++revisionRequestId;
  isLoadingCommits.set(true);
  commitsError.set('');
  try {
    const page = await api.getRevisions(repo(), get(revisionFilter), 0, PAGE_SIZE, null);
    if (requestId !== revisionRequestId) return;
    revisionRows.set(page.rows);
    graphState.set(page.graph_state);
    hasMoreRevisions.set(page.has_more);
  } catch (e) {
    if (requestId !== revisionRequestId) return;
    commitsError.set(errorText(e));
    revisionRows.set([]);
    graphState.set(null);
    hasMoreRevisions.set(false);
  } finally {
    if (requestId === revisionRequestId) isLoadingCommits.set(false);
  }
}

/** Appends the next page; the graph continues from the state of the previous page. */
export async function loadMoreCommits(): Promise<void> {
  if (get(isLoadingCommits) || get(isLoadingMoreCommits) || !get(hasMoreRevisions)) return;
  const requestId = revisionRequestId;
  isLoadingMoreCommits.set(true);
  try {
    const rows = get(revisionRows);
    const page = await api.getRevisions(repo(), get(revisionFilter), rows.length, PAGE_SIZE, get(graphState));
    if (requestId !== revisionRequestId) return;
    revisionRows.set([...rows, ...page.rows]);
    graphState.set(page.graph_state);
    hasMoreRevisions.set(page.has_more);
  } catch (e) {
    if (requestId === revisionRequestId) commitsError.set(errorText(e));
  } finally {
    isLoadingMoreCommits.set(false);
  }
}

function reloadHistoryIfActive(): void {
  if (get(activeTab) === 'history') void loadCommits();
}

export function setRevisionFilter(patch: Partial<RevisionFilter>): void {
  revisionFilter.update((f) => ({ ...f, ...patch }));
  void loadCommits();
}

export function resetRevisionFilter(): void {
  revisionFilter.set({ ...DEFAULT_FILTER });
  void loadCommits();
}

/** Restricts the history to the commits touching one file (following renames). */
export function showFileHistory(path: string): void {
  setRevisionFilter({ path, follow: true });
}

export function selectCommit(hash: string | null): void {
  selectedHash.set(hash);
}

let lastLoadedHash = '';

async function loadCommitDetails(hash: string): Promise<void> {
  isLoadingCommitDetails.set(true);
  try {
    const details = await api.getCommitDetails(repo(), hash);
    if (get(selectedHash) === hash) commitDetails.set(details);
  } catch (e) {
    if (get(selectedHash) === hash) commitDetails.set(null);
    console.error('Failed to load commit details:', e);
  } finally {
    if (get(selectedHash) === hash) isLoadingCommitDetails.set(false);
  }
}

async function loadCommitFiles(hash: string): Promise<void> {
  isLoadingCommitFiles.set(true);
  commitFilesError.set('');
  try {
    const files = await api.getCommitFiles(repo(), hash);
    if (get(selectedHash) !== hash) return;
    commitFiles.set(files);
    // Like Git Extensions: show the first file right away.
    if (files.length > 0 && !get(selectedCommitFile)) selectedCommitFile.set(files[0].path);
  } catch (e) {
    if (get(selectedHash) !== hash) return;
    commitFiles.set([]);
    commitFilesError.set(errorText(e));
  } finally {
    if (get(selectedHash) === hash) isLoadingCommitFiles.set(false);
  }
}

/** Reaction to the selected commit changing (wired up with `$:` in App.svelte). */
export function onSelectedHashChanged(hash: string | null): void {
  if (!hash) {
    lastLoadedHash = '';
    commitDetails.set(null);
    commitFiles.set([]);
    selectedCommitFile.set('');
    commitDiff.set([]);
    return;
  }
  if (hash === lastLoadedHash) return;
  lastLoadedHash = hash;
  selectedCommitFile.set('');
  commitDiff.set([]);
  commitFiles.set([]);
  void loadCommitDetails(hash);
  void loadCommitFiles(hash);
}

/** Loads the diff of the selected file; also runs when the diff options change. */
export async function onSelectedCommitFileChanged(
  hash: string | null,
  filePath: string,
  options: DiffOptions,
): Promise<void> {
  if (!hash || !filePath) {
    commitDiff.set([]);
    return;
  }
  isLoadingCommitFileDiff.set(true);
  try {
    const diff = await api.getCommitDiff(repo(), hash, filePath, options);
    if (get(selectedHash) === hash && get(selectedCommitFile) === filePath) commitDiff.set(diff);
  } catch (e) {
    commitDiff.set([]);
    commitFilesError.set(`Error loading diff: ${errorText(e)}`);
  } finally {
    isLoadingCommitFileDiff.set(false);
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
        selectedWorkingFileDiff.set([]);
      }
    }
  } catch (e) {
    workingFiles.set([]);
    workingFilesError.set(errorText(e));
  } finally {
    isLoadingWorkingFiles.set(false);
  }
}

async function loadWorkingFileDiff(filePath: string, isStaged: boolean, options: DiffOptions): Promise<void> {
  isLoadingWorkingFileDiff.set(true);
  try {
    selectedWorkingFileDiff.set(await api.getWorkingDiff(repo(), filePath, isStaged, options));
  } catch (e) {
    selectedWorkingFileDiff.set([]);
    workingFilesError.set(`Error loading diff: ${errorText(e)}`);
  } finally {
    isLoadingWorkingFileDiff.set(false);
  }
}

/** Reaction to the selected working file (or the diff options) changing. */
export function onSelectedWorkingFileChanged(
  file: { path: string; is_staged: boolean } | null,
  options: DiffOptions,
): void {
  if (file) {
    void loadWorkingFileDiff(file.path, file.is_staged, options);
  } else {
    selectedWorkingFileDiff.set([]);
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
  const amend = get(amendCommit);
  if (!message.trim()) {
    commitFeedbackError.set('Please enter a commit message');
    return;
  }
  // Amending may only change the message, everything else needs something staged.
  if (!amend && get(workingFiles).every((f) => !f.is_staged)) {
    commitFeedbackError.set('No staged files to commit. Stage some files first!');
    return;
  }
  isCommitting.set(true);
  commitFeedback.set('');
  commitFeedbackError.set('');
  try {
    const response = await api.commitChanges(repo(), message, { amend, sign: get(signCommit) });
    commitFeedback.set(`${amend ? 'Amended' : 'Committed'} successfully!\n${response}`);
    commitMessage.set('');
    amendCommit.set(false);
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

/** Reloads everything that changes when files are staged, unstaged or discarded. */
async function refreshWorkingState(): Promise<void> {
  await loadWorkingFiles();
  // The selected file may still be there with different content (e.g. after staging one hunk).
  const selected = get(selectedWorkingFile);
  if (selected) onSelectedWorkingFileChanged(selected, get(diffOptions));
  await loadGitStatus();
}

export async function stageAll(): Promise<void> {
  await runWorkdirAction(() => api.stageAll(repo()), 'Failed to stage files');
}

export async function unstageAll(): Promise<void> {
  await runWorkdirAction(() => api.unstageAll(repo()), 'Failed to unstage files');
}

export async function discardFiles(targets: DiscardTarget[]): Promise<void> {
  if (targets.length === 0) return;
  await runWorkdirAction(() => api.discardChanges(repo(), targets), 'Failed to discard changes');
}

async function runWorkdirAction(run: () => Promise<unknown>, failure: string): Promise<void> {
  commitFeedback.set('');
  commitFeedbackError.set('');
  try {
    await run();
  } catch (e) {
    commitFeedbackError.set(`${failure}: ${errorText(e)}`);
  }
  await refreshWorkingState();
}

/** Stages, unstages or discards selected hunks/lines of the file shown in the diff. */
export async function applySelection(
  action: SelectionAction,
  file: { path: string; untracked: boolean },
  selections: HunkSelection[],
): Promise<void> {
  if (selections.length === 0) return;
  await runWorkdirAction(
    () => api.applySelection(repo(), file.path, action, selections, file.untracked, get(diffOptions)),
    `Failed to ${action} the selection`,
  );
}

/** Turns "amend" on/off; turning it on pre-fills the message of the last commit. */
export async function setAmend(on: boolean): Promise<void> {
  amendCommit.set(on);
  if (on && !get(commitMessage).trim()) {
    try {
      commitMessage.set(await api.getLastCommitMessage(repo()));
    } catch (e) {
      console.warn('Could not read the last commit message:', e);
    }
  }
}

/** Pre-fills the message from `commit.template` when the message box is empty. */
export async function applyCommitTemplate(): Promise<void> {
  if (get(commitMessage).trim()) return;
  try {
    const template = await api.getCommitTemplate(repo());
    if (template && !get(commitMessage).trim()) commitMessage.set(template);
  } catch (e) {
    console.warn('Could not read the commit template:', e);
  }
}

// ---------------------------------------------------------------------------
// Stashes
// ---------------------------------------------------------------------------

export async function loadStashEntries(): Promise<void> {
  isLoadingStashEntries.set(true);
  try {
    stashEntries.set(await api.listStashes(repo()));
  } catch (e) {
    stashEntries.set([]);
    console.error('Failed to list stashes:', e);
  } finally {
    isLoadingStashEntries.set(false);
  }
}

async function runStashAction(run: () => Promise<string>, failure: string): Promise<void> {
  commitFeedback.set('');
  commitFeedbackError.set('');
  try {
    commitFeedback.set(await run());
  } catch (e) {
    commitFeedbackError.set(`${failure}: ${errorText(e)}`);
  }
  await Promise.all([refreshWorkingState(), loadStashEntries(), loadStashes()]);
}

export async function saveStash(): Promise<void> {
  const message = get(stashMessage).trim();
  await runStashAction(
    () => api.stashSave(repo(), message || null, get(stashIncludeUntracked), false),
    'Failed to stash',
  );
  stashMessage.set('');
}

export const applyStashEntry = (index: number): Promise<void> =>
  runStashAction(() => api.applyStash(repo(), index), 'Failed to apply the stash');
export const popStashEntry = (index: number): Promise<void> =>
  runStashAction(() => api.popStash(repo(), index), 'Failed to pop the stash');
export const dropStashEntry = (index: number): Promise<void> =>
  runStashAction(() => api.dropStash(repo(), index), 'Failed to drop the stash');

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
  void loadStashEntries();
  reloadHistoryIfActive();
}

export function selectTab(tab: TabName): void {
  activeTab.set(tab);
  if (tab === 'history') {
    void loadCommits();
  } else if (tab === 'dashboard') {
    void loadGitStatus();
  } else if (tab === 'commit') {
    void loadWorkingFiles();
    void loadGitStatus();
    void loadStashEntries();
    void applyCommitTemplate();
  } else if (tab === 'branches') {
    clearBranchFeedback();
    void loadBranches();
    void loadRemoteBranches();
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
  void loadStashEntries();
}

// ---------------------------------------------------------------------------
// Window focus
// ---------------------------------------------------------------------------

let lastFocusRefresh = 0;
const FOCUS_REFRESH_MIN_INTERVAL_MS = 2000;

/** Picks up changes made outside of the app (editor, terminal) when the window regains focus. */
export function refreshOnFocus(): void {
  const now = Date.now();
  if (!repo() || now - lastFocusRefresh < FOCUS_REFRESH_MIN_INTERVAL_MS) return;
  lastFocusRefresh = now;
  void loadGitStatus();
  const tab = get(activeTab);
  if (tab === 'commit') void loadWorkingFiles();
  else if (tab === 'history') void loadCommits();
}
