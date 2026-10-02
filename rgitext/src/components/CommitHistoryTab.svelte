<script lang="ts">
  import { onDestroy } from 'svelte';
  import * as actions from '../lib/actions';
  import { copyText, shortHash } from '../lib/format';
  import type { RevisionRow } from '../lib/types';
  import { currentRepoPath } from '../stores/repo';
  import { branches, remoteBranches, tags } from '../stores/branches';
  import {
    commitDetails, commitDiff, commitFiles, commitFilesError, commitsError, hasMoreRevisions,
    isLoadingCommitDetails, isLoadingCommitFileDiff, isLoadingCommitFiles, isLoadingCommits,
    isLoadingMoreCommits, revisionFilter, revisionRows, selectedCommitFile, selectedHash,
  } from '../stores/history';
  import RevisionGrid from './history/RevisionGrid.svelte';
  import CommitDetailsPanel from './history/CommitDetailsPanel.svelte';
  import DiffView from './history/DiffView.svelte';
  import TreeBrowser from './history/TreeBrowser.svelte';
  import BlameView from './history/BlameView.svelte';

  type DetailTab = 'diff' | 'tree' | 'blame';

  const SEARCH_DEBOUNCE_MS = 350;

  let tab: DetailTab = 'diff';
  let blamePath = '';
  let searchText = $revisionFilter.search;
  let authorText = $revisionFilter.author;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let authorTimer: ReturnType<typeof setTimeout> | undefined;

  // Context menu
  let menuRow: RevisionRow | null = null;
  let menuX = 0;
  let menuY = 0;

  $: blameTarget = blamePath || $selectedCommitFile;

  function onSearchInput(): void {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => actions.setRevisionFilter({ search: searchText.trim() }), SEARCH_DEBOUNCE_MS);
  }

  function onAuthorInput(): void {
    if (authorTimer) clearTimeout(authorTimer);
    authorTimer = setTimeout(() => actions.setRevisionFilter({ author: authorText.trim() }), SEARCH_DEBOUNCE_MS);
  }

  function clearFilters(): void {
    searchText = '';
    authorText = '';
    actions.resetRevisionFilter();
  }

  $: hasFilter =
    $revisionFilter.search !== '' ||
    $revisionFilter.author !== '' ||
    $revisionFilter.path !== '' ||
    $revisionFilter.scope !== 'current' ||
    $revisionFilter.firstParent;

  function openContextMenu(event: MouseEvent, row: RevisionRow): void {
    menuRow = row;
    menuX = Math.min(event.clientX, window.innerWidth - 260);
    menuY = Math.min(event.clientY, window.innerHeight - 140);
  }

  function closeMenu(): void {
    menuRow = null;
  }

  async function menuCopyHash(): Promise<void> {
    if (menuRow) await copyText(menuRow.hash);
    closeMenu();
  }

  function menuCheckout(): void {
    if (!menuRow) return;
    const hash = menuRow.hash;
    closeMenu();
    if (confirm(`Check out commit ${shortHash(hash)}? This leaves you in a detached HEAD state.`)) {
      void actions.performCheckout(hash);
    }
  }

  function menuReset(): void {
    if (!menuRow) return;
    const hash = menuRow.hash;
    closeMenu();
    const mode = prompt(
      `Reset current branch to ${shortHash(hash)}?\n\nEnter mode: 'hard', 'mixed', or 'soft':`,
      'mixed',
    );
    const normalized = mode?.toLowerCase().trim();
    if (normalized && ['hard', 'mixed', 'soft'].includes(normalized)) {
      void actions.performResetBranch(hash, normalized);
    } else if (mode) {
      alert("Invalid mode! Please enter 'hard', 'mixed', or 'soft'.");
    }
  }

  function showBlame(path: string): void {
    blamePath = path;
    tab = 'blame';
  }

  function statusClass(status: string): string {
    return `status-badge-${status.charAt(0).toLowerCase()}`;
  }

  function selectFile(path: string): void {
    selectedCommitFile.set(path);
    blamePath = '';
    tab = 'diff';
  }

  onDestroy(() => {
    if (searchTimer) clearTimeout(searchTimer);
    if (authorTimer) clearTimeout(authorTimer);
  });
</script>

<div class="hist">
  <div class="hist-toolbar">
    <select
      class="hist-select"
      value={$revisionFilter.scope}
      on:change={(e) => actions.setRevisionFilter({ scope: e.currentTarget.value })}
      title="Which commits to show"
    >
      <option value="current">Current branch</option>
      <option value="all">All branches</option>
      {#if $branches.length > 0}
        <optgroup label="Branches">
          {#each $branches as name}<option value={name}>{name}</option>{/each}
        </optgroup>
      {/if}
      {#if $remoteBranches.length > 0}
        <optgroup label="Remotes">
          {#each $remoteBranches as name}<option value={name}>{name}</option>{/each}
        </optgroup>
      {/if}
      {#if $tags.length > 0}
        <optgroup label="Tags">
          {#each $tags as name}<option value={name}>{name}</option>{/each}
        </optgroup>
      {/if}
    </select>
    <input
      class="hist-input"
      type="search"
      placeholder="Search messages…"
      bind:value={searchText}
      on:input={onSearchInput}
    />
    <input
      class="hist-input hist-input-narrow"
      type="search"
      placeholder="Author…"
      bind:value={authorText}
      on:input={onAuthorInput}
    />
    <label class="hist-check" title="Follow only the first parent of merge commits">
      <input
        type="checkbox"
        checked={$revisionFilter.firstParent}
        on:change={(e) => actions.setRevisionFilter({ firstParent: e.currentTarget.checked })}
      />
      First parent
    </label>
    {#if $revisionFilter.path}
      <span class="hist-chip" title="Showing only commits that touch this file">
        {$revisionFilter.path}
        <button aria-label="Clear file filter" on:click={() => actions.setRevisionFilter({ path: '', follow: false })}>×</button>
      </span>
    {/if}
    {#if hasFilter}
      <button class="hist-link" on:click={clearFilters}>Clear filters</button>
    {/if}
    <span class="hist-spacer"></span>
    <span class="badge">{$revisionRows.length}{$hasMoreRevisions ? '+' : ''} commits</span>
  </div>

  {#if $commitsError}
    <div class="alert alert-error">
      <h3>Failed to load commits</h3>
      <p>{$commitsError}</p>
    </div>
  {/if}

  <div class="hist-main">
    <RevisionGrid
      rows={$revisionRows}
      selectedHash={$selectedHash}
      hasMore={$hasMoreRevisions}
      loading={$isLoadingCommits}
      loadingMore={$isLoadingMoreCommits}
      onSelect={actions.selectCommit}
      onLoadMore={actions.loadMoreCommits}
      onContextMenu={openContextMenu}
    />

    {#if $selectedHash}
      <div class="hist-detail">
        <div class="hist-detail-left">
          <CommitDetailsPanel
            details={$commitDetails}
            loading={$isLoadingCommitDetails}
            onSelectHash={actions.selectCommit}
          />
          <hr class="drawer-divider" />
          <span class="drawer-label">Changed files ({$commitFiles.length})</span>
          <div class="commit-files-list-wrapper">
            {#if $isLoadingCommitFiles}
              <div class="sidebar-loading-mini"><div class="loading-spinner-mini inline-spinner"></div>Loading files…</div>
            {:else if $commitFilesError}
              <div class="sidebar-error-mini">{$commitFilesError}</div>
            {:else if $commitFiles.length === 0}
              <div class="sidebar-empty-mini">No changed files.</div>
            {:else}
              {#each $commitFiles as file (file.path)}
                <div
                  class="commit-file-item hist-file {$selectedCommitFile === file.path ? 'selected' : ''}"
                  role="button"
                  tabindex="0"
                  on:click={() => selectFile(file.path)}
                  on:keydown={(e) => e.key === 'Enter' && selectFile(file.path)}
                >
                  <span class="file-item-status {statusClass(file.status)}">{file.status.charAt(0)}</span>
                  <span class="file-item-path" title={file.old_path ? `${file.old_path} → ${file.path}` : file.path}>
                    {file.path}
                  </span>
                  <span class="hist-file-actions">
                    <button title="Show only commits touching this file" on:click|stopPropagation={() => actions.showFileHistory(file.path)}>H</button>
                    <button title="Blame this file at this commit" on:click|stopPropagation={() => showBlame(file.path)}>B</button>
                  </span>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <div class="hist-detail-right">
          <div class="hist-tabs">
            <button class:active={tab === 'diff'} on:click={() => (tab = 'diff')}>Diff</button>
            <button class:active={tab === 'tree'} on:click={() => (tab = 'tree')}>File tree</button>
            <button class:active={tab === 'blame'} disabled={!blameTarget} on:click={() => (tab = 'blame')}>Blame</button>
            <span class="hist-spacer"></span>
            <button class="hist-close" aria-label="Close details" on:click={() => actions.selectCommit(null)}>×</button>
          </div>

          {#if tab === 'diff'}
            {#if $selectedCommitFile}
              <DiffView files={$commitDiff} loading={$isLoadingCommitFileDiff} hideFileHeader={false} />
            {:else}
              <div class="diff-viewer-empty">
                <span>{$isLoadingCommitFiles ? 'Loading…' : 'Select a file from the list to view its diff.'}</span>
              </div>
            {/if}
          {:else if tab === 'tree'}
            <TreeBrowser
              repoPath={$currentRepoPath}
              rev={$selectedHash}
              onShowHistory={actions.showFileHistory}
              onBlame={showBlame}
            />
          {:else if blameTarget}
            <BlameView
              repoPath={$currentRepoPath}
              path={blameTarget}
              rev={$selectedHash}
              onSelectHash={actions.selectCommit}
            />
          {/if}
        </div>
      </div>
    {/if}
  </div>

  {#if menuRow}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="hist-menu-overlay" on:click={closeMenu} on:contextmenu|preventDefault={closeMenu}></div>
    <div class="hist-menu shadow-premium" style="left: {menuX}px; top: {menuY}px">
      <div class="hist-menu-header">Commit {shortHash(menuRow.hash)}</div>
      <button class="dropdown-item" on:click={menuCopyHash}><span>Copy commit hash</span></button>
      <button class="dropdown-item" on:click={menuCheckout}><span>Check out this commit…</span></button>
      <button class="dropdown-item" on:click={menuReset}><span>Reset current branch to here…</span></button>
    </div>
  {/if}
</div>

<style>
  .hist {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: calc(100vh - 170px);
    min-height: 0;
  }
  .hist-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }
  .hist-select,
  .hist-input {
    background: #0b0f19;
    color: #e5e7eb;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 0.82rem;
    outline: none;
  }
  .hist-select {
    max-width: 220px;
  }
  .hist-input {
    width: 220px;
  }
  .hist-input-narrow {
    width: 140px;
  }
  .hist-select:focus,
  .hist-input:focus {
    border-color: rgba(234, 88, 12, 0.6);
  }
  .hist-check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: #94a3b8;
  }
  .hist-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 320px;
    padding: 3px 4px 3px 10px;
    border-radius: 9999px;
    background: rgba(59, 130, 246, 0.15);
    color: #93c5fd;
    font-size: 0.78rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hist-chip button,
  .hist-close {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    padding: 0 6px;
  }
  .hist-link {
    background: none;
    border: none;
    color: #fb923c;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .hist-link:hover {
    text-decoration: underline;
  }
  .hist-spacer {
    flex: 1;
  }
  .hist-main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .hist-main > :global(.rg-wrap) {
    flex: 1 1 45%;
  }
  .hist-detail {
    flex: 1 1 55%;
    min-height: 0;
    display: flex;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    overflow: hidden;
  }
  .hist-detail-left {
    width: 340px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-right: 1px solid rgba(255, 255, 255, 0.05);
    overflow-y: auto;
  }
  .hist-detail-right {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #070a13;
  }
  .hist-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 8px;
    background: #0c101d;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    flex-shrink: 0;
  }
  .hist-tabs button {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #94a3b8;
    padding: 9px 14px;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .hist-tabs button:hover:not(:disabled) {
    color: #e5e7eb;
  }
  .hist-tabs button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .hist-tabs button.active {
    color: #fb923c;
    border-bottom-color: #ea580c;
  }
  .hist-file {
    display: flex;
    align-items: center;
  }
  .hist-file-actions {
    display: none;
    gap: 2px;
    margin-left: auto;
  }
  .hist-file:hover .hist-file-actions,
  .hist-file.selected .hist-file-actions {
    display: inline-flex;
  }
  .hist-file-actions button {
    background: rgba(255, 255, 255, 0.06);
    border: none;
    border-radius: 4px;
    color: #cbd5e1;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 2px 5px;
    cursor: pointer;
  }
  .hist-file-actions button:hover {
    background: rgba(234, 88, 12, 0.3);
  }
  .hist-menu-overlay {
    position: fixed;
    inset: 0;
    z-index: 999;
  }
  .hist-menu {
    position: fixed;
    z-index: 1000;
    min-width: 240px;
    padding: 4px 0;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
  }
  .hist-menu-header {
    padding: 6px 14px;
    margin-bottom: 2px;
    font-size: 0.75rem;
    color: #94a3b8;
    font-family: var(--font-mono, monospace);
    border-bottom: 1px solid #334155;
  }
</style>
