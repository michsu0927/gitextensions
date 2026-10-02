<script lang="ts">
  import * as actions from '../lib/actions';
  import { formatRelative } from '../lib/format';
  import type { HunkSelection, WorkingFile } from '../lib/types';
  import { gitStatus } from '../stores/repo';
  import { conflictsDialogOpen, repoState } from '../stores/ops';
  import { diffOptions } from '../stores/history';
  import {
    amendCommit, commitFeedback, commitFeedbackError, commitMessage, isCommitting, isLoadingWorkingFileDiff,
    isLoadingWorkingFiles, selectedStagedPaths, selectedUnstagedPaths, selectedWorkingFile,
    selectedWorkingFileDiff, signCommit, stashEntries, stashIncludeUntracked, stashMessage, workingFiles,
    workingFilesError,
  } from '../stores/workdir';
  import DiffView from './history/DiffView.svelte';

  let stashOpen = false;

  $: unstaged = $workingFiles.filter((f) => !f.is_staged);
  $: staged = $workingFiles.filter((f) => f.is_staged);
  $: current = $selectedWorkingFile;
  $: currentFile = current
    ? $workingFiles.find((f) => f.path === current.path && f.is_staged === current.is_staged)
    : undefined;
  $: diffActions = buildActions(currentFile);

  const STATUS_LABEL: Record<string, string> = { '??': 'N', M: 'M', A: 'A', D: 'D', R: 'R', C: 'C', U: 'U', T: 'T' };

  function statusLabel(file: WorkingFile): string {
    return STATUS_LABEL[file.status] ?? file.status;
  }

  function statusClass(file: WorkingFile): string {
    return file.status === '??' ? 'status-badge-untracked' : `status-badge-${file.status.toLowerCase()}`;
  }

  function splitPath(path: string): { dir: string; name: string } {
    const i = path.lastIndexOf('/');
    return i < 0 ? { dir: '', name: path } : { dir: path.slice(0, i + 1), name: path.slice(i + 1) };
  }

  function buildActions(file: WorkingFile | undefined) {
    if (!file) return [];
    const target = { path: file.path, untracked: file.status === '??' };
    if (file.is_staged) {
      return [
        {
          label: 'Unstage',
          run: (selections: HunkSelection[]) => void actions.applySelection('unstage', target, selections),
        },
      ];
    }
    const list = [
      {
        label: 'Stage',
        danger: false,
        run: (selections: HunkSelection[]) => void actions.applySelection('stage', target, selections),
      },
    ];
    // Untracked files have no committed content to go back to.
    if (!target.untracked) {
      list.push({
        label: 'Discard',
        danger: true,
        run: (selections: HunkSelection[]) => {
          if (confirm('Discard the selected changes in the working tree? This cannot be undone.')) {
            void actions.applySelection('discard', target, selections);
          }
        },
      });
    }
    return list;
  }

  function select(file: WorkingFile): void {
    selectedWorkingFile.set({ path: file.path, is_staged: file.is_staged });
  }

  function toggle(list: string[], path: string): string[] {
    return list.includes(path) ? list.filter((p) => p !== path) : [...list, path];
  }

  function toggleAll(files: WorkingFile[], chosen: string[]): string[] {
    return chosen.length === files.length ? [] : files.map((f) => f.path);
  }

  function discardSelected(): void {
    const targets = unstaged
      .filter((f) => $selectedUnstagedPaths.includes(f.path))
      .map((f) => ({ path: f.path, untracked: f.status === '??' }));
    if (targets.length === 0) return;
    const untracked = targets.filter((t) => t.untracked).length;
    const note = untracked > 0 ? `\n${untracked} untracked file(s) will be deleted.` : '';
    if (confirm(`Discard all changes of ${targets.length} file(s)? This cannot be undone.${note}`)) {
      void actions.discardFiles(targets);
    }
  }

  function dropStash(index: number, name: string): void {
    if (confirm(`Drop ${name}? The stashed changes will be lost.`)) void actions.dropStashEntry(index);
  }

  function onMessageKey(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      event.preventDefault();
      void actions.handleCommit();
    }
  }
</script>

<div class="ct">
  <div class="ct-left">
    {#if $repoState.conflicted.length > 0}
      <div class="ct-conflict">
        <span>{$repoState.conflicted.length} file{$repoState.conflicted.length === 1 ? '' : 's'} with merge conflicts</span>
        <button on:click={() => ($conflictsDialogOpen = true)}>Resolve…</button>
      </div>
    {/if}
    <section class="ct-section">
      <header class="ct-section-header">
        <input
          type="checkbox"
          aria-label="Select all unstaged files"
          checked={unstaged.length > 0 && $selectedUnstagedPaths.length === unstaged.length}
          on:change={() => selectedUnstagedPaths.set(toggleAll(unstaged, $selectedUnstagedPaths))}
        />
        <h3>Unstaged ({unstaged.length})</h3>
        <span class="ct-spacer"></span>
        <button disabled={$selectedUnstagedPaths.length === 0} on:click={actions.handleStageSelected}>Stage</button>
        <button disabled={unstaged.length === 0} on:click={actions.stageAll}>Stage all</button>
        <button class="danger" disabled={$selectedUnstagedPaths.length === 0} on:click={discardSelected}>Discard</button>
      </header>
      <div class="ct-list">
        {#if $isLoadingWorkingFiles && $workingFiles.length === 0}
          <div class="ct-empty"><div class="loading-spinner-mini"></div>Loading…</div>
        {:else if $workingFilesError}
          <div class="ct-empty ct-error">{$workingFilesError}</div>
        {:else if unstaged.length === 0}
          <div class="ct-empty">No unstaged changes.</div>
        {:else}
          {#each unstaged as file (file.path)}
            {@const parts = splitPath(file.path)}
            <div
              class="ct-file"
              class:selected={current?.path === file.path && !current.is_staged}
              role="button"
              tabindex="0"
              on:click={() => select(file)}
              on:keydown={(e) => e.key === 'Enter' && select(file)}
            >
              <input
                type="checkbox"
                checked={$selectedUnstagedPaths.includes(file.path)}
                on:click|stopPropagation
                on:change={() => selectedUnstagedPaths.update((l) => toggle(l, file.path))}
              />
              <span class="file-item-status {statusClass(file)}">{statusLabel(file)}</span>
              <span class="ct-path" title={file.path}><span class="ct-dir">{parts.dir}</span>{parts.name}</span>
              <button
                class="ct-row-action"
                title="Stage this file"
                on:click|stopPropagation={() => {
                  selectedUnstagedPaths.set([file.path]);
                  void actions.handleStageSelected();
                }}>+</button
              >
            </div>
          {/each}
        {/if}
      </div>
    </section>

    <section class="ct-section">
      <header class="ct-section-header">
        <input
          type="checkbox"
          aria-label="Select all staged files"
          checked={staged.length > 0 && $selectedStagedPaths.length === staged.length}
          on:change={() => selectedStagedPaths.set(toggleAll(staged, $selectedStagedPaths))}
        />
        <h3>Staged ({staged.length})</h3>
        <span class="ct-spacer"></span>
        <button disabled={$selectedStagedPaths.length === 0} on:click={actions.handleUnstageSelected}>Unstage</button>
        <button disabled={staged.length === 0} on:click={actions.unstageAll}>Unstage all</button>
      </header>
      <div class="ct-list">
        {#if staged.length === 0}
          <div class="ct-empty">Nothing staged.</div>
        {:else}
          {#each staged as file (file.path)}
            {@const parts = splitPath(file.path)}
            <div
              class="ct-file"
              class:selected={current?.path === file.path && current.is_staged}
              role="button"
              tabindex="0"
              on:click={() => select(file)}
              on:keydown={(e) => e.key === 'Enter' && select(file)}
            >
              <input
                type="checkbox"
                checked={$selectedStagedPaths.includes(file.path)}
                on:click|stopPropagation
                on:change={() => selectedStagedPaths.update((l) => toggle(l, file.path))}
              />
              <span class="file-item-status {statusClass(file)}">{statusLabel(file)}</span>
              <span class="ct-path" title={file.path}><span class="ct-dir">{parts.dir}</span>{parts.name}</span>
              <button
                class="ct-row-action"
                title="Unstage this file"
                on:click|stopPropagation={() => {
                  selectedStagedPaths.set([file.path]);
                  void actions.handleUnstageSelected();
                }}>−</button
              >
            </div>
          {/each}
        {/if}
      </div>
    </section>

    <section class="ct-commit">
      <textarea
        class="ct-message"
        placeholder="Commit message (Ctrl+Enter to commit)"
        rows="4"
        bind:value={$commitMessage}
        on:keydown={onMessageKey}
      ></textarea>
      <div class="ct-options">
        <label title="Replace the last commit instead of creating a new one">
          <input type="checkbox" checked={$amendCommit} on:change={(e) => actions.setAmend(e.currentTarget.checked)} />
          Amend
        </label>
        <label title="Sign the commit with GPG (-S)">
          <input type="checkbox" bind:checked={$signCommit} />
          Sign
        </label>
        <span class="ct-spacer"></span>
        <span class="ct-branch" title="Current branch">{$gitStatus?.branch ?? ''}</span>
      </div>
      <button class="btn-primary ct-commit-btn" disabled={$isCommitting} on:click={actions.handleCommit}>
        {#if $isCommitting}Committing…{:else if $amendCommit}Amend commit{:else}Commit{/if}
      </button>
      {#if $commitFeedback}
        <div class="ct-alert ct-ok">
          <pre>{$commitFeedback}</pre>
          <button aria-label="Dismiss" on:click={() => commitFeedback.set('')}>×</button>
        </div>
      {/if}
      {#if $commitFeedbackError}
        <div class="ct-alert ct-bad">
          <pre>{$commitFeedbackError}</pre>
          <button aria-label="Dismiss" on:click={() => commitFeedbackError.set('')}>×</button>
        </div>
      {/if}
    </section>

    <section class="ct-section ct-stash">
      <header class="ct-section-header">
        <button class="ct-toggle" on:click={() => (stashOpen = !stashOpen)}>{stashOpen ? '▾' : '▸'}</button>
        <h3>Stash ({$stashEntries.length})</h3>
      </header>
      {#if stashOpen}
        <div class="ct-stash-new">
          <input class="ct-input" placeholder="Stash message (optional)" bind:value={$stashMessage} />
          <label><input type="checkbox" bind:checked={$stashIncludeUntracked} /> Include untracked</label>
          <button disabled={$workingFiles.length === 0} on:click={actions.saveStash}>Stash changes</button>
        </div>
        {#each $stashEntries as entry (entry.name)}
          <div class="ct-stash-row">
            <div class="ct-stash-text" title={entry.message}>
              <span class="ct-stash-name">{entry.name}</span>
              <span>{entry.message}</span>
              <span class="ct-dir">{formatRelative(entry.timestamp)}</span>
            </div>
            <button on:click={() => actions.applyStashEntry(entry.index)}>Apply</button>
            <button on:click={() => actions.popStashEntry(entry.index)}>Pop</button>
            <button class="danger" on:click={() => dropStash(entry.index, entry.name)}>Drop</button>
          </div>
        {:else}
          <div class="ct-empty">No stashes.</div>
        {/each}
      {/if}
    </section>
  </div>

  <div class="ct-right">
    {#if current}
      <div class="ct-diff-title">
        <span class="ct-diff-path" title={current.path}>{current.path}</span>
        <span class="ct-badge {current.is_staged ? 'staged' : 'unstaged'}">{current.is_staged ? 'staged' : 'unstaged'}</span>
      </div>
      <DiffView
        files={$selectedWorkingFileDiff}
        loading={$isLoadingWorkingFileDiff}
        hideFileHeader={true}
        actions={diffActions}
      />
    {:else}
      <div class="ct-placeholder">
        <span>Select a file to see its changes. Tick single lines or whole hunks to stage them separately.</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .ct {
    display: flex;
    gap: 12px;
    height: calc(100vh - 170px);
    min-height: 0;
  }
  .ct-left {
    width: 400px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
    padding-right: 2px;
  }
  .ct-right {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #070a13;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    overflow: hidden;
  }
  .ct-conflict {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 10px;
    background: rgba(234, 179, 8, 0.12);
    border: 1px solid rgba(234, 179, 8, 0.35);
    color: #fde68a;
    font-size: 0.8rem;
    flex-shrink: 0;
  }
  .ct-section {
    display: flex;
    flex-direction: column;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    overflow: hidden;
    flex-shrink: 0;
  }
  .ct-section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: #0c101d;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }
  .ct-section-header h3 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: #e5e7eb;
  }
  .ct-spacer {
    flex: 1;
  }
  .ct button,
  .ct-section-header button {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    color: #cbd5e1;
    padding: 3px 9px;
    font-size: 0.74rem;
    cursor: pointer;
  }
  .ct button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }
  .ct button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .ct button.danger {
    color: #fca5a5;
    border-color: rgba(239, 68, 68, 0.35);
  }
  .ct-list {
    max-height: 240px;
    overflow-y: auto;
  }
  .ct-empty {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    color: #64748b;
    font-size: 0.8rem;
  }
  .ct-error {
    color: #f87171;
  }
  .ct-file {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    font-size: 0.8rem;
    color: #cbd5e1;
    cursor: default;
  }
  .ct-file:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .ct-file.selected {
    background: rgba(234, 88, 12, 0.16);
  }
  .ct-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ct-dir {
    color: #64748b;
  }
  .ct button.ct-row-action {
    display: none;
    padding: 0 7px;
    font-size: 0.9rem;
    line-height: 1.3;
  }
  .ct-file:hover .ct-row-action {
    display: inline-block;
  }
  .ct-commit {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    flex-shrink: 0;
  }
  .ct-message,
  .ct-input {
    width: 100%;
    box-sizing: border-box;
    background: #0b0f19;
    color: #e5e7eb;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 8px 10px;
    font-family: inherit;
    font-size: 0.82rem;
    outline: none;
    resize: vertical;
  }
  .ct-message:focus,
  .ct-input:focus {
    border-color: rgba(234, 88, 12, 0.6);
  }
  .ct-options {
    display: flex;
    align-items: center;
    gap: 14px;
    font-size: 0.78rem;
    color: #94a3b8;
  }
  .ct-options label,
  .ct-stash-new label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .ct-branch {
    color: #fb923c;
    font-family: var(--font-mono, monospace);
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ct-commit-btn {
    padding: 8px 12px !important;
    font-size: 0.85rem !important;
  }
  .ct-alert {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 0.78rem;
  }
  .ct-alert pre {
    flex: 1;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
  }
  .ct-alert button {
    background: none !important;
    border: none !important;
    color: inherit !important;
    font-size: 1rem !important;
    padding: 0 4px !important;
  }
  .ct-ok {
    background: rgba(16, 185, 129, 0.12);
    color: #6ee7b7;
  }
  .ct-bad {
    background: rgba(239, 68, 68, 0.12);
    color: #fca5a5;
  }
  .ct-stash-new {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    font-size: 0.78rem;
    color: #94a3b8;
  }
  .ct-stash-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    font-size: 0.78rem;
    color: #cbd5e1;
  }
  .ct-stash-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .ct-stash-text span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ct-stash-name {
    color: #fb923c;
    font-family: var(--font-mono, monospace);
    font-size: 0.72rem;
  }
  .ct-diff-title {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: #0c101d;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    flex-shrink: 0;
  }
  .ct-diff-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.82rem;
    color: #e5e7eb;
  }
  .ct-badge {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 7px;
    border-radius: 9999px;
  }
  .ct-badge.staged {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
  }
  .ct-badge.unstaged {
    background: rgba(234, 179, 8, 0.15);
    color: #facc15;
  }
  .ct-placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px;
    text-align: center;
    color: #475569;
    font-size: 0.85rem;
  }
</style>
