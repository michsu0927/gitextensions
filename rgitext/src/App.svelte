<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  import Sidebar from './components/Sidebar.svelte';
  import WorkspaceInspector from './components/WorkspaceInspector.svelte';
  import Header from './components/Header.svelte';
  import DashboardTab from './components/DashboardTab.svelte';
  import CommitHistoryTab from './components/CommitHistoryTab.svelte';
  import BranchesTab from './components/BranchesTab.svelte';
  import SettingsTab from './components/SettingsTab.svelte';
  import ConsoleLogDrawer from './components/ConsoleLogDrawer.svelte';

  import * as actions from './lib/actions';
  import {
    clearConsoleLogs, gitConsoleLogs, isConsoleLogDrawerExpanded, showConsoleLogDrawer, startGitLogListener,
  } from './stores/consoleLog';
  import {
    activeTab, customGitPath, gitError, gitVersion, isChecking, isSavingSettings, settingsError,
    settingsMessage, tempGitPath,
  } from './stores/app';
  import {
    currentRepoPath, gitStatus, isEditingPath, isLoadingStatus, pathError, statusError, tempRepoPath,
  } from './stores/repo';
  import { diffOptions, isLoadingCommits, selectedCommitFile, selectedHash } from './stores/history';
  import {
    commitFeedback, commitFeedbackError, commitMessage, expandedDirs, isCommitting, isLoadingWorkingFileDiff,
    isLoadingWorkingFiles, selectedStagedPaths, selectedUnstagedPaths, selectedWorkingFile,
    selectedWorkingFileDiff, workingFiles, workingFilesError,
  } from './stores/workdir';
  import {
    branchActionError, branches, branchesError, branchMessage, branchSearchQuery, isBranchesCollapsed,
    isExecutingBranchAction, isLoadingBranches, isLoadingRemoteBranches, isLoadingRemotes, isLoadingStashes,
    isLoadingTags, isSidebarBranchesExpanded, isSidebarRemotesExpanded, isSidebarStashesExpanded,
    isSidebarTagsExpanded, remoteBranches, remoteBranchesError, remotes, remotesError, selectedBranch,
    selectedSidebarBranch, selectedSidebarStash, selectedSidebarTag, sidebarBranchError, sidebarBranchMessage,
    stashes, stashesError, tags, tagsError,
  } from './stores/branches';

  // Load the files/diff of whatever is selected.
  $: actions.onSelectedHashChanged($selectedHash);
  $: void actions.onSelectedCommitFileChanged($selectedHash, $selectedCommitFile, $diffOptions);
  $: actions.onSelectedWorkingFileChanged($selectedWorkingFile, $diffOptions);

  let unlistenGitLog: UnlistenFn | undefined;

  onMount(async () => {
    // Listen to git command logs first so the initial load is captured too.
    unlistenGitLog = await startGitLogListener();
    await actions.initialize();
  });

  onDestroy(() => {
    unlistenGitLog?.();
  });
</script>

<div class="app-layout">
  <Sidebar
    activeTab={$activeTab}
    gitVersion={$gitVersion}
    gitError={$gitError}
    selectTab={actions.selectTab}
    toggleConsoleLogDrawer={() => showConsoleLogDrawer.update((v) => !v)}
  />

  <WorkspaceInspector
    bind:isBranchesCollapsed={$isBranchesCollapsed}
    branches={$branches}
    isLoadingBranches={$isLoadingBranches}
    branchesError={$branchesError}
    tags={$tags}
    isLoadingTags={$isLoadingTags}
    tagsError={$tagsError}
    stashes={$stashes}
    isLoadingStashes={$isLoadingStashes}
    stashesError={$stashesError}
    remotes={$remotes}
    isLoadingRemotes={$isLoadingRemotes}
    remotesError={$remotesError}
    remoteBranches={$remoteBranches}
    isLoadingRemoteBranches={$isLoadingRemoteBranches}
    remoteBranchesError={$remoteBranchesError}
    gitStatus={$gitStatus}
    isExecutingBranchAction={$isExecutingBranchAction}
    bind:sidebarBranchMessage={$sidebarBranchMessage}
    bind:sidebarBranchError={$sidebarBranchError}
    bind:branchSearchQuery={$branchSearchQuery}
    bind:selectedSidebarBranch={$selectedSidebarBranch}
    bind:selectedSidebarTag={$selectedSidebarTag}
    bind:selectedSidebarStash={$selectedSidebarStash}
    bind:isSidebarBranchesExpanded={$isSidebarBranchesExpanded}
    bind:isSidebarTagsExpanded={$isSidebarTagsExpanded}
    bind:isSidebarStashesExpanded={$isSidebarStashesExpanded}
    bind:isSidebarRemotesExpanded={$isSidebarRemotesExpanded}
    refreshAll={actions.refreshAll}
    performCheckout={actions.performCheckout}
    performMerge={actions.performMerge}
    checkoutTag={actions.checkoutTag}
    performStashApply={actions.performStashApply}
    performCheckoutRemoteBranch={actions.performCheckoutRemoteBranch}
  />

  <main class="main-content">
    <Header
      bind:isBranchesCollapsed={$isBranchesCollapsed}
      bind:currentRepoPath={$currentRepoPath}
      bind:tempRepoPath={$tempRepoPath}
      bind:isEditingPath={$isEditingPath}
      bind:pathError={$pathError}
      isChecking={$isChecking}
      isLoadingCommits={$isLoadingCommits}
      isLoadingStatus={$isLoadingStatus}
      isLoadingBranches={$isLoadingBranches}
      refreshAll={actions.refreshAll}
      browseRepoPath={actions.browseRepoPath}
      browseRepoPathFromInput={actions.browseRepoPathFromInput}
      saveRepoPath={actions.saveRepoPath}
      handlePathKeydown={actions.handlePathKeydown}
    />

    {#if $pathError}
      <div class="path-error-banner">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
        <span class="path-error-text">{$pathError}</span>
        <button class="close-error-btn" on:click={() => pathError.set('')}>&times;</button>
      </div>
    {/if}

    <div class="content-panel">
      {#if $activeTab === 'dashboard'}
        <DashboardTab
          gitVersion={$gitVersion}
          gitError={$gitError}
          isChecking={$isChecking}
          gitStatus={$gitStatus}
          statusError={$statusError}
          isLoadingStatus={$isLoadingStatus}
          checkGit={actions.checkGit}
        />
      {:else if $activeTab === 'history'}
        <CommitHistoryTab />
      {:else if $activeTab === 'branches'}
        <BranchesTab
          branches={$branches}
          isLoadingBranches={$isLoadingBranches}
          branchesError={$branchesError}
          bind:selectedBranch={$selectedBranch}
          bind:branchMessage={$branchMessage}
          bind:branchActionError={$branchActionError}
          isExecutingBranchAction={$isExecutingBranchAction}
          gitStatus={$gitStatus}
          workingFiles={$workingFiles}
          isLoadingWorkingFiles={$isLoadingWorkingFiles}
          workingFilesError={$workingFilesError}
          bind:selectedWorkingFile={$selectedWorkingFile}
          bind:selectedWorkingFileDiff={$selectedWorkingFileDiff}
          isLoadingWorkingFileDiff={$isLoadingWorkingFileDiff}
          bind:expandedDirs={$expandedDirs}
          bind:selectedUnstagedPaths={$selectedUnstagedPaths}
          bind:selectedStagedPaths={$selectedStagedPaths}
          bind:commitMessage={$commitMessage}
          isCommitting={$isCommitting}
          bind:commitFeedback={$commitFeedback}
          bind:commitFeedbackError={$commitFeedbackError}
          performCheckout={actions.performCheckout}
          performMerge={actions.performMerge}
          performRebase={actions.performRebase}
          performCreateBranch={actions.performCreateBranch}
          performRenameBranch={actions.performRenameBranch}
          performDeleteBranch={actions.performDeleteBranch}
          performPull={actions.performPull}
          performPush={actions.performPush}
          toggleDir={actions.toggleDir}
          handleStageSelected={actions.handleStageSelected}
          handleUnstageSelected={actions.handleUnstageSelected}
          handleCommit={actions.handleCommit}
          loadWorkingFiles={actions.loadWorkingFiles}
        />
      {:else if $activeTab === 'settings'}
        <SettingsTab
          bind:tempGitPath={$tempGitPath}
          isSavingSettings={$isSavingSettings}
          bind:settingsMessage={$settingsMessage}
          bind:settingsError={$settingsError}
          customGitPath={$customGitPath}
          browseGitExecutable={actions.browseGitExecutable}
          saveSettings={actions.saveSettings}
          remoteBranches={$remoteBranches}
          remotes={$remotes}
          configureAndFetchRemote={actions.configureAndFetchRemote}
          performCheckoutRemoteBranch={actions.performCheckoutRemoteBranch}
        />
      {/if}
    </div>

    <ConsoleLogDrawer
      bind:show={$showConsoleLogDrawer}
      bind:expanded={$isConsoleLogDrawerExpanded}
      logs={$gitConsoleLogs}
      onClear={clearConsoleLogs}
    />
  </main>
</div>
