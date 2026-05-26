<script>
  import { invoke as tauriInvoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  // Import Console Log Drawer
  import ConsoleLogDrawer from './components/ConsoleLogDrawer.svelte';

  // Custom Git console logs state
  let gitConsoleLogs = [];
  let showConsoleLogDrawer = false;
  let isConsoleLogDrawerExpanded = false;

  function getRawGitCommand(cmd, args) {
    switch (cmd) {
      case 'get_git_version':
        return ['git --version'];
      case 'verify_git_repo':
        return [`git rev-parse --is-inside-work-tree` + (args.repoPath ? ` (dir: ${args.repoPath})` : '')];
      case 'get_git_status':
        return [
          'git rev-parse --is-inside-work-tree',
          'git status --porcelain=v1 -b',
          'git status'
        ];
      case 'get_commit_log':
        return ['git log --graph --oneline --decorate --all -n 100'];
      case 'get_git_branches':
        return ['git branch --format="%(refname:short)"'];
      case 'get_git_tags':
        return ['git tag'];
      case 'get_git_stashes':
        return ['git stash list'];
      case 'get_git_remotes':
        return ['git remote -v'];
      case 'configure_and_fetch_remote':
        return [
          `git remote add/set-url ${args.remoteName || 'origin'} ${args.remoteUrl || ''}`,
          `git fetch ${args.remoteName || 'origin'}`
        ];
      case 'checkout_branch':
        return [`git checkout ${args.branchName || ''}`];
      case 'merge_branch':
        return [`git merge ${args.branchName || ''}`];
      case 'rebase_branch':
        return [`git rebase ${args.branchName || ''}`];
      case 'create_branch':
        return [`git branch ${args.newBranchName || ''}`];
      case 'reset_current_branch':
        const mode = args.mode ? `--${args.mode}` : '--mixed';
        return [`git reset ${mode} ${args.target || 'HEAD'}`];
      case 'rename_branch':
        return [`git branch -m ${args.oldName || ''} ${args.newName || ''}`];
      case 'delete_branch':
        const flag = args.force ? '-D' : '-d';
        return [`git branch ${flag} ${args.branchName || ''}`];
      case 'pull_changes':
        return ['git pull'];
      case 'push_changes':
        return ['git push'];
      case 'stage_files':
        const filesToAdd = args.filePaths ? args.filePaths.map(f => `"${f}"`).join(' ') : '';
        return [`git add ${filesToAdd}`];
      case 'unstage_files':
        const filesToReset = args.filePaths ? args.filePaths.map(f => `"${f}"`).join(' ') : '';
        return [`git reset HEAD ${filesToReset}`];
      case 'commit_changes':
        return [`git commit -m "${args.message || ''}"`];
      case 'apply_stash':
        const idx = args.stashIndex !== undefined ? `stash@{${args.stashIndex}}` : '';
        return [`git stash apply ${idx}`];
      case 'get_commit_files':
        return [`git show --pretty="" --name-status ${args.commitHash || ''}`];
      case 'get_commit_file_diff':
        return [`git diff ${args.commitHash || ''}^! -- "${args.filePath || ''}"`];
      case 'get_working_dir_files':
        return ['git status --porcelain=v1'];
      case 'get_working_file_diff':
        const diffFlag = args.isStaged ? '--staged' : '';
        return [`git diff ${diffFlag} -- "${args.filePath || ''}"`];
      case 'list_directory':
        return [`[Tauri FS] List files in ${args.relativeDir || '.'}`];
      case 'select_directory':
        return ['[Tauri Dialog] Select directory'];
      case 'select_git_executable':
        return ['[Tauri Dialog] Select git executable'];
      default:
        return [];
    }
  }

  async function invoke(cmd, args = {}) {
    const logId = Math.random().toString(36).substring(2, 9);
    const now = new Date();
    const timestamp = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
    
    const logEntry = {
      id: logId,
      timestamp,
      command: cmd,
      args: args,
      gitCommands: getRawGitCommand(cmd, args),
      status: 'pending',
      result: null,
      error: null
    };

    gitConsoleLogs = [logEntry, ...gitConsoleLogs];

    try {
      const res = await tauriInvoke(cmd, args);
      gitConsoleLogs = gitConsoleLogs.map(log => {
        if (log.id === logId) {
          return { ...log, status: 'success', result: res };
        }
        return log;
      });
      return res;
    } catch (err) {
      gitConsoleLogs = gitConsoleLogs.map(log => {
        if (log.id === logId) {
          return { ...log, status: 'error', error: err.toString() };
        }
        return log;
      });
      throw err;
    }
  }

  function clearConsoleLogs() {
    gitConsoleLogs = [];
  }


  // Import split components
  import Sidebar from './components/Sidebar.svelte';
  import WorkspaceInspector from './components/WorkspaceInspector.svelte';
  import Header from './components/Header.svelte';
  import DashboardTab from './components/DashboardTab.svelte';
  import CommitHistoryTab from './components/CommitHistoryTab.svelte';
  import BranchesTab from './components/BranchesTab.svelte';
  import SettingsTab from './components/SettingsTab.svelte';

  let name = '';
  let greeting = '';
  let gitVersion = 'Checking...';
  let gitError = '';
  let activeTab = 'dashboard';
  let isChecking = false;

  // Custom Git executable path state
  let customGitPath = localStorage.getItem('custom_git_path') || '';
  let tempGitPath = customGitPath;
  let isSavingSettings = false;
  let settingsMessage = '';
  let settingsError = '';

  // Active Repository path state
  let currentRepoPath = 'e:\\gitextensions';
  let tempRepoPath = 'e:\\gitextensions';
  let isEditingPath = false;
  let pathError = '';

  // Git Status state
  let gitStatus = null;
  let isLoadingStatus = false;
  let statusError = '';

  // Commit history state
  let commits = [];
  let isLoadingCommits = false;
  let commitsError = '';
  let selectedCommit = null;

  // Selected commit files & diffs
  let commitFiles = [];
  let isLoadingCommitFiles = false;
  let selectedCommitFile = '';
  let selectedCommitFileDiff = '';
  let isLoadingCommitFileDiff = false;
  let commitFilesError = '';

  // Working directory staging and diffs
  let workingFiles = [];
  let isLoadingWorkingFiles = false;
  let selectedWorkingFile = null; // { path: string, is_staged: boolean }
  let selectedWorkingFileDiff = '';
  let isLoadingWorkingFileDiff = false;
  let workingFilesError = '';

  // Directory expand/collapse state: { [path]: DirEntry[] | 'loading' }
  let expandedDirs = {};

  // Multiselect checkboxes
  let selectedUnstagedPaths = [];
  let selectedStagedPaths = [];

  async function toggleDir(dirPath) {
    if (expandedDirs[dirPath]) {
      // Collapse
      const copy = { ...expandedDirs };
      // Remove this dir and all children
      for (const key of Object.keys(copy)) {
        if (key === dirPath || key.startsWith(dirPath + '/')) {
          delete copy[key];
        }
      }
      expandedDirs = copy;
    } else {
      // Expand — load listing from backend
      expandedDirs = { ...expandedDirs, [dirPath]: 'loading' };
      try {
        const entries = await invoke('list_directory', { repoPath: currentRepoPath, relativeDir: dirPath });
        expandedDirs = { ...expandedDirs, [dirPath]: entries };
      } catch (e) {
        expandedDirs = { ...expandedDirs, [dirPath]: [] };
        console.error('Failed to list directory:', e);
      }
    }
  }

  // Committing changes
  let commitMessage = '';
  let isCommitting = false;
  let commitFeedback = '';
  let commitFeedbackError = '';
  let lastLoadedCommitHash = '';

  async function loadCommitFiles(hash) {
    if (hash === lastLoadedCommitHash) return;
    lastLoadedCommitHash = hash;
    isLoadingCommitFiles = true;
    commitFilesError = '';
    selectedCommitFile = '';
    selectedCommitFileDiff = '';
    try {
      commitFiles = await invoke('get_commit_files', { repoPath: currentRepoPath, commitHash: hash, gitPath: customGitPath || null });
    } catch (e) {
      commitFiles = [];
      commitFilesError = e.toString();
    } finally {
      isLoadingCommitFiles = false;
    }
  }

  async function loadCommitFileDiff(hash, filePath) {
    isLoadingCommitFileDiff = true;
    try {
      selectedCommitFileDiff = await invoke('get_commit_file_diff', { repoPath: currentRepoPath, commitHash: hash, filePath, gitPath: customGitPath || null });
    } catch (e) {
      selectedCommitFileDiff = `Error loading diff: ${e.toString()}`;
    } finally {
      isLoadingCommitFileDiff = false;
    }
  }

  async function loadWorkingFiles() {
    isLoadingWorkingFiles = true;
    workingFilesError = '';
    selectedUnstagedPaths = [];
    selectedStagedPaths = [];
    try {
      workingFiles = await invoke('get_working_dir_files', { repoPath: currentRepoPath, gitPath: customGitPath || null });
      if (selectedWorkingFile) {
        const stillExists = workingFiles.some(f => f.path === selectedWorkingFile.path && f.is_staged === selectedWorkingFile.is_staged);
        if (!stillExists) {
          selectedWorkingFile = null;
          selectedWorkingFileDiff = '';
        }
      }
    } catch (e) {
      workingFiles = [];
      workingFilesError = e.toString();
    } finally {
      isLoadingWorkingFiles = false;
    }
  }

  async function loadWorkingFileDiff(filePath, isStaged) {
    isLoadingWorkingFileDiff = true;
    try {
      selectedWorkingFileDiff = await invoke('get_working_file_diff', { repoPath: currentRepoPath, filePath, isStaged, gitPath: customGitPath || null });
    } catch (e) {
      selectedWorkingFileDiff = `Error loading diff: ${e.toString()}`;
    } finally {
      isLoadingWorkingFileDiff = false;
    }
  }

  async function handleStageSelected() {
    if (selectedUnstagedPaths.length === 0) return;
    try {
      commitFeedback = '';
      commitFeedbackError = '';
      await invoke('stage_files', { repoPath: currentRepoPath, filePaths: selectedUnstagedPaths, gitPath: customGitPath || null });
      await loadWorkingFiles();
      await loadGitStatus();
    } catch (e) {
      commitFeedbackError = `Failed to stage files: ${e.toString()}`;
    }
  }

  async function handleUnstageSelected() {
    if (selectedStagedPaths.length === 0) return;
    try {
      commitFeedback = '';
      commitFeedbackError = '';
      await invoke('unstage_files', { repoPath: currentRepoPath, filePaths: selectedStagedPaths, gitPath: customGitPath || null });
      await loadWorkingFiles();
      await loadGitStatus();
    } catch (e) {
      commitFeedbackError = `Failed to unstage files: ${e.toString()}`;
    }
  }

  async function handleCommit() {
    if (!commitMessage.trim()) {
      commitFeedbackError = 'Please enter a commit message';
      return;
    }
    const stagedCount = workingFiles.filter(f => f.is_staged).length;
    if (stagedCount === 0) {
      commitFeedbackError = 'No staged files to commit. Stage some files first!';
      return;
    }
    isCommitting = true;
    commitFeedback = '';
    commitFeedbackError = '';
    try {
      const response = await invoke('commit_changes', { repoPath: currentRepoPath, message: commitMessage, gitPath: customGitPath || null });
      commitFeedback = `Committed successfully!\n${response}`;
      commitMessage = '';
      await loadWorkingFiles();
      await loadGitStatus();
      await loadBranches();
      if (activeTab === 'history') {
        await loadCommits();
      }
    } catch (e) {
      commitFeedbackError = e.toString();
    } finally {
      isCommitting = false;
    }
  }

  $: if (selectedCommit && selectedCommit.hash) {
    loadCommitFiles(selectedCommit.hash);
  } else {
    lastLoadedCommitHash = '';
    commitFiles = [];
    selectedCommitFile = '';
    selectedCommitFileDiff = '';
  }

  $: if (selectedCommit && selectedCommit.hash && selectedCommitFile) {
    loadCommitFileDiff(selectedCommit.hash, selectedCommitFile);
  } else {
    selectedCommitFileDiff = '';
  }

  $: if (selectedWorkingFile) {
    loadWorkingFileDiff(selectedWorkingFile.path, selectedWorkingFile.is_staged);
  } else {
    selectedWorkingFileDiff = '';
  }

  // Branch management state
  let branches = [];
  let isLoadingBranches = false;
  let branchesError = '';
  let selectedBranch = '';
  let branchMessage = '';
  let branchActionError = '';
  let isExecutingBranchAction = false;

  // Left branches sidebar state
  let isBranchesCollapsed = false;
  let branchSearchQuery = '';
  let selectedSidebarBranch = '';
  let sidebarBranchMessage = '';
  let sidebarBranchError = '';

  // Tags state
  let tags = [];
  let isLoadingTags = false;
  let tagsError = '';
  let selectedSidebarTag = '';

  // Stashes state
  let stashes = [];
  let isLoadingStashes = false;
  let stashesError = '';
  let selectedSidebarStash = null;

  // Remotes state
  let remotes = [];
  let isLoadingRemotes = false;
  let remotesError = '';

  // Remote branches state
  let remoteBranches = [];
  let isLoadingRemoteBranches = false;
  let remoteBranchesError = '';

  // Sidebar accordion expansion states
  let isSidebarBranchesExpanded = true;
  let isSidebarTagsExpanded = false;
  let isSidebarStashesExpanded = false;
  let isSidebarRemotesExpanded = false;

  async function refreshAll() {
    checkGit();
    loadGitStatus();
    loadBranches();
    loadTags();
    loadStashes();
    loadRemotes();
    loadRemoteBranches();
    loadWorkingFiles();
    if (activeTab === 'history') {
      loadCommits();
    }
  }

  async function callGreet() {
    if (!name.trim()) return;
    try {
      greeting = await invoke('greet', { name });
    } catch (e) {
      console.error(e);
    }
  }

  async function checkGit() {
    isChecking = true;
    gitError = '';
    try {
      gitVersion = await invoke('get_git_version', { gitPath: customGitPath || null });
    } catch (e) {
      gitVersion = '';
      gitError = e.toString();
    } finally {
      isChecking = false;
    }
  }

  async function loadGitStatus() {
    isLoadingStatus = true;
    statusError = '';
    try {
      gitStatus = await invoke('get_git_status', { repoPath: currentRepoPath, gitPath: customGitPath || null });
    } catch (e) {
      gitStatus = null;
      statusError = e.toString();
    } finally {
      isLoadingStatus = false;
    }
  }

  async function loadCommits() {
    isLoadingCommits = true;
    commitsError = '';
    try {
      commits = await invoke('get_commit_log', { repoPath: currentRepoPath, limit: 100, gitPath: customGitPath || null });
    } catch (e) {
      commitsError = e.toString();
      commits = [];
    } finally {
      isLoadingCommits = false;
    }
  }

  async function saveRepoPath() {
    if (!tempRepoPath.trim()) {
      pathError = 'Path cannot be empty';
      return;
    }
    pathError = '';
    try {
      // Validate the path as a valid Git repo in the backend
      await invoke('verify_git_repo', { repoPath: tempRepoPath, gitPath: customGitPath || null });
      currentRepoPath = tempRepoPath;
      isEditingPath = false;
      
      // Update environment and status checks
      checkGit();
      loadGitStatus();
      loadBranches();
      loadTags();
      loadStashes();
      loadRemotes();
      loadWorkingFiles();
      
      // Refresh state
      if (activeTab === 'history') {
        loadCommits();
      } else {
        commits = [];
        commitsError = '';
      }
    } catch (e) {
      pathError = e.toString();
    }
  }

  async function browseRepoPath() {
    pathError = '';
    try {
      const selected = await invoke('select_directory');
      if (selected) {
        tempRepoPath = selected;
        // Automatically save/open if chosen through browse
        await saveRepoPath();
      }
    } catch (e) {
      if (e.toString() !== 'Dialog cancelled by user') {
        pathError = e.toString();
      }
    }
  }

  async function browseRepoPathFromInput() {
    pathError = '';
    try {
      const selected = await invoke('select_directory');
      if (selected) {
        tempRepoPath = selected;
      }
    } catch (e) {
      if (e.toString() !== 'Dialog cancelled by user') {
        pathError = e.toString();
      }
    }
  }

  async function saveSettings() {
    isSavingSettings = true;
    settingsMessage = '';
    settingsError = '';
    try {
      // Verify path by testing git version first
      const testVersion = await invoke('get_git_version', { gitPath: tempGitPath || null });
      
      customGitPath = tempGitPath;
      localStorage.setItem('custom_git_path', customGitPath);
      
      gitVersion = testVersion;
      gitError = '';
      settingsMessage = 'Settings saved successfully! Connected to Git.';
      
      // Reload statuses
      loadGitStatus();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      settingsError = `Failed to verify Git path: ${e.toString()}`;
    } finally {
      isSavingSettings = false;
    }
  }

  async function browseGitExecutable() {
    settingsError = '';
    try {
      const selected = await invoke('select_git_executable');
      if (selected) {
        tempGitPath = selected;
      }
    } catch (e) {
      if (e.toString() !== 'Dialog cancelled by user') {
        settingsError = e.toString();
      }
    }
  }

  async function loadBranches() {
    isLoadingBranches = true;
    branchesError = '';
    branchMessage = '';
    branchActionError = '';
    try {
      branches = await invoke('get_git_branches', { repoPath: currentRepoPath, gitPath: customGitPath || null });
    } catch (e) {
      branches = [];
      branchesError = e.toString();
    } finally {
      isLoadingBranches = false;
    }
  }

  async function loadTags() {
    isLoadingTags = true;
    tagsError = '';
    try {
      tags = await invoke('get_git_tags', { repoPath: currentRepoPath, gitPath: customGitPath || null });
    } catch (e) {
      tags = [];
      tagsError = e.toString();
    } finally {
      isLoadingTags = false;
    }
  }

  async function loadStashes() {
    isLoadingStashes = true;
    stashesError = '';
    try {
      stashes = await invoke('get_git_stashes', { repoPath: currentRepoPath, gitPath: customGitPath || null });
    } catch (e) {
      stashes = [];
      stashesError = e.toString();
    } finally {
      isLoadingStashes = false;
    }
  }

  async function loadRemotes() {
    isLoadingRemotes = true;
    remotesError = '';
    try {
      remotes = await invoke('get_git_remotes', { repoPath: currentRepoPath, gitPath: customGitPath || null });
    } catch (e) {
      remotes = [];
      remotesError = e.toString();
    } finally {
      isLoadingRemotes = false;
    }
  }

  async function loadRemoteBranches() {
    isLoadingRemoteBranches = true;
    remoteBranchesError = '';
    try {
      remoteBranches = await invoke('get_git_remote_branches', { repoPath: currentRepoPath, gitPath: customGitPath || null });
    } catch (e) {
      remoteBranches = [];
      remoteBranchesError = e.toString();
    } finally {
      isLoadingRemoteBranches = false;
    }
  }

  async function checkoutTag(tagName) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    sidebarBranchMessage = '';
    sidebarBranchError = '';
    try {
      const response = await invoke('checkout_branch', { 
        repoPath: currentRepoPath, 
        branchName: tagName, 
        gitPath: customGitPath || null 
      });
      branchMessage = `Checked out tag successfully:\n${response}`;
      sidebarBranchMessage = `Checked out tag ${tagName} successfully.`;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      loadBranches();
      loadTags();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
      sidebarBranchError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performStashApply(stashIndex) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    sidebarBranchMessage = '';
    sidebarBranchError = '';
    try {
      const response = await invoke('apply_stash', { 
        repoPath: currentRepoPath, 
        stashIndex, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      sidebarBranchMessage = `Stash applied successfully!`;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      loadStashes();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
      sidebarBranchError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performCheckout(branchName) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    sidebarBranchMessage = '';
    sidebarBranchError = '';
    try {
      const response = await invoke('checkout_branch', { 
        repoPath: currentRepoPath, 
        branchName, 
        gitPath: customGitPath || null 
      });
      branchMessage = `Checked out successfully:\n${response}`;
      sidebarBranchMessage = `Checked out ${branchName} successfully.`;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      loadBranches();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
      sidebarBranchError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performCheckoutRemoteBranch(remoteBranch, localBranch) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    sidebarBranchMessage = '';
    sidebarBranchError = '';
    try {
      const response = await invoke('checkout_remote_branch', { 
        repoPath: currentRepoPath, 
        remoteBranch, 
        localBranch, 
        gitPath: customGitPath || null 
      });
      branchMessage = `Checked out remote branch successfully:\n${response}`;
      sidebarBranchMessage = `Checked out ${remoteBranch} as local ${localBranch} successfully.`;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      loadBranches();
      loadRemoteBranches();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
      sidebarBranchError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function configureAndFetchRemote(remoteName, remoteUrl) {
    try {
      const response = await invoke('configure_and_fetch_remote', {
        repoPath: currentRepoPath,
        remoteName,
        remoteUrl,
        gitPath: customGitPath || null
      });
      
      // Reload state
      await loadRemotes();
      await loadRemoteBranches();
      
      return response;
    } catch (e) {
      throw e;
    }
  }

  async function performMerge(branchName) {
    const currentBranchName = gitStatus?.branch || 'active';
    if (!confirm(`Are you sure you want to merge branch "${branchName}" into the current branch "${currentBranchName}"?`)) {
      return;
    }
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    sidebarBranchMessage = '';
    sidebarBranchError = '';
    try {
      const response = await invoke('merge_branch', { 
        repoPath: currentRepoPath, 
        branchName, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      sidebarBranchMessage = `Merged branch "${branchName}" into "${currentBranchName}" successfully.`;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      loadBranches();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
      sidebarBranchError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performRebase(branchName) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    try {
      const response = await invoke('rebase_branch', { 
        repoPath: currentRepoPath, 
        branchName, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performCreateBranch(newBranchName) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    try {
      const response = await invoke('create_branch', { 
        repoPath: currentRepoPath, 
        newBranchName, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      
      // Refresh environment and state
      loadBranches();
    } catch (e) {
      branchActionError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performResetBranch(target, mode) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    try {
      const response = await invoke('reset_current_branch', { 
        repoPath: currentRepoPath, 
        target, 
        mode, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      loadWorkingFiles();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performRenameBranch(oldName, newName) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    try {
      const response = await invoke('rename_branch', { 
        repoPath: currentRepoPath, 
        oldName, 
        newName, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      
      // Refresh environment and state
      loadBranches();
      loadGitStatus();
    } catch (e) {
      branchActionError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performDeleteBranch(branchName, force) {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    try {
      const response = await invoke('delete_branch', { 
        repoPath: currentRepoPath, 
        branchName, 
        force, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      
      // Refresh environment and state
      loadBranches();
      loadGitStatus();
    } catch (e) {
      branchActionError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performPull() {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    try {
      const response = await invoke('pull_changes', { 
        repoPath: currentRepoPath, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
      
      // Refresh environment and state
      checkGit();
      loadGitStatus();
      loadWorkingFiles();
      if (activeTab === 'history') {
        loadCommits();
      }
    } catch (e) {
      branchActionError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  async function performPush() {
    isExecutingBranchAction = true;
    branchMessage = '';
    branchActionError = '';
    try {
      const response = await invoke('push_changes', { 
        repoPath: currentRepoPath, 
        gitPath: customGitPath || null 
      });
      branchMessage = response;
    } catch (e) {
      branchActionError = e.toString();
    } finally {
      isExecutingBranchAction = false;
    }
  }

  function handlePathKeydown(event) {
    if (event.key === 'Enter') {
      saveRepoPath();
    } else if (event.key === 'Escape') {
      isEditingPath = false;
      pathError = '';
    }
  }

  function selectTab(tab) {
    activeTab = tab;
    if (tab === 'history') {
      loadCommits();
    } else if (tab === 'dashboard') {
      loadGitStatus();
    } else if (tab === 'branches') {
      loadBranches();
      loadRemoteBranches();
      loadWorkingFiles();
    }
  }

  let unlistenGitLog;

  onMount(async () => {
    // 1. Listen to Git command logs first so we capture initial load commands!
    try {
      unlistenGitLog = await listen('git-command-log', (event) => {
        const commandLine = event.payload;
        gitConsoleLogs = gitConsoleLogs.map((log, index) => {
          if (index === 0 || log.status === 'pending') {
            const existing = log.gitCommands || [];
            return { ...log, gitCommands: [...existing, commandLine] };
          }
          return log;
        });
      });
    } catch (err) {
      console.error("Failed to establish Tauri git log event listener:", err);
    }

    // 2. Load initial states
    checkGit();
    loadGitStatus();
    loadBranches();
    loadTags();
    loadStashes();
    loadRemotes();
    loadRemoteBranches();
    loadWorkingFiles();
  });

  onDestroy(() => {
    if (unlistenGitLog) unlistenGitLog();
  });
</script>

<div class="app-layout">
  <Sidebar 
    {activeTab} 
    {gitVersion} 
    {gitError} 
    {selectTab} 
    toggleConsoleLogDrawer={() => showConsoleLogDrawer = !showConsoleLogDrawer}
  />

  <WorkspaceInspector 
    bind:isBranchesCollapsed 
    {branches} 
    {isLoadingBranches} 
    {branchesError} 
    {tags} 
    {isLoadingTags} 
    {tagsError} 
    {stashes} 
    {isLoadingStashes} 
    {stashesError} 
    {remotes} 
    {isLoadingRemotes} 
    {remotesError} 
    {remoteBranches}
    {isLoadingRemoteBranches}
    {remoteBranchesError}
    {gitStatus} 
    {isExecutingBranchAction} 
    bind:sidebarBranchMessage 
    bind:sidebarBranchError 
    bind:branchSearchQuery
    bind:selectedSidebarBranch
    bind:selectedSidebarTag
    bind:selectedSidebarStash
    bind:isSidebarBranchesExpanded
    bind:isSidebarTagsExpanded
    bind:isSidebarStashesExpanded
    bind:isSidebarRemotesExpanded
    {refreshAll} 
    {performCheckout} 
    {performMerge} 
    {checkoutTag} 
    {performStashApply} 
    performCheckoutRemoteBranch={performCheckoutRemoteBranch}
  />

  <main class="main-content">
    <Header 
      bind:isBranchesCollapsed 
      bind:currentRepoPath 
      bind:tempRepoPath 
      bind:isEditingPath 
      bind:pathError 
      {isChecking} 
      {isLoadingCommits} 
      {isLoadingStatus} 
      {isLoadingBranches} 
      {refreshAll} 
      {browseRepoPath} 
      {browseRepoPathFromInput} 
      {saveRepoPath} 
      {handlePathKeydown} 
    />

    {#if pathError}
      <div class="path-error-banner">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
        <span class="path-error-text">{pathError}</span>
        <button class="close-error-btn" on:click={() => pathError = ''}>&times;</button>
      </div>
    {/if}

    <div class="content-panel">
      {#if activeTab === 'dashboard'}
        <DashboardTab 
          {gitVersion} 
          {gitError} 
          {isChecking} 
          {gitStatus} 
          {statusError} 
          {isLoadingStatus} 
          {checkGit} 
        />
      {:else if activeTab === 'history'}
        <CommitHistoryTab 
          {commits} 
          {isLoadingCommits} 
          {commitsError} 
          bind:selectedCommit 
          {commitFiles} 
          {isLoadingCommitFiles} 
          bind:selectedCommitFile 
          bind:selectedCommitFileDiff 
          {isLoadingCommitFileDiff} 
          {commitFilesError} 
          {performResetBranch}
        />
      {:else if activeTab === 'branches'}
        <BranchesTab 
          {branches} 
          {isLoadingBranches} 
          {branchesError} 
          bind:selectedBranch 
          bind:branchMessage 
          bind:branchActionError 
          {isExecutingBranchAction} 
          {gitStatus} 
          {workingFiles} 
          {isLoadingWorkingFiles} 
          {workingFilesError} 
          bind:selectedWorkingFile 
          bind:selectedWorkingFileDiff 
          {isLoadingWorkingFileDiff} 
          bind:expandedDirs 
          bind:selectedUnstagedPaths 
          bind:selectedStagedPaths 
          bind:commitMessage 
          {isCommitting} 
          bind:commitFeedback 
          bind:commitFeedbackError 
          {performCheckout} 
          {performMerge} 
          {performRebase}
          {performCreateBranch}
          {performRenameBranch}
          {performDeleteBranch}
          {performPull}
          {performPush}
          {toggleDir} 
          {handleStageSelected} 
          {handleUnstageSelected} 
          {handleCommit} 
          {loadWorkingFiles} 
        />
      {:else if activeTab === 'settings'}
        <SettingsTab 
          bind:tempGitPath 
          {isSavingSettings} 
          bind:settingsMessage 
          bind:settingsError 
          {customGitPath} 
          {browseGitExecutable} 
          {saveSettings} 
          {remoteBranches}
          {remotes}
          {configureAndFetchRemote}
          {performCheckoutRemoteBranch}
        />
      {/if}
    </div>

    <ConsoleLogDrawer 
      bind:show={showConsoleLogDrawer} 
      bind:expanded={isConsoleLogDrawerExpanded} 
      logs={gitConsoleLogs} 
      onClear={clearConsoleLogs}
    />
  </main>
</div>
