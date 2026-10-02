<script lang="ts">
  import { cancelNetwork, cloneDialog, fetchAll, pullDialog, pushDialog } from '../lib/network';
  import { progress, remotesDialogOpen } from '../stores/ops';
  import { currentRepoPath, gitStatus } from '../stores/repo';

  $: busy = $progress !== null;
  $: lastLine = $progress?.lines[$progress.lines.length - 1] ?? '';
  $: hasRepo = $currentRepoPath !== '';
</script>

<div class="sb">
  <div class="sb-buttons">
    <button disabled={busy || !hasRepo} on:click={fetchAll} title="Fetch from all remotes">Fetch</button>
    <button disabled={busy || !hasRepo} on:click={pullDialog} title="Fetch and integrate remote changes">Pull…</button>
    <button disabled={busy || !hasRepo} on:click={pushDialog} title="Push commits to a remote">Push…</button>
    <span class="sb-sep"></span>
    <button disabled={busy} on:click={cloneDialog} title="Clone a repository">Clone…</button>
    <button disabled={!hasRepo} on:click={() => remotesDialogOpen.set(true)} title="Manage remotes">Remotes…</button>
    <span class="sb-spacer"></span>
    {#if $gitStatus?.branch}
      <span class="sb-branch" title="Current branch">⎇ {$gitStatus.branch}</span>
    {/if}
  </div>

  {#if $progress}
    <div class="sb-progress" role="status">
      <div class="loading-spinner-mini"></div>
      <div class="sb-progress-text">
        <strong>{$progress.title}</strong>
        <span title={lastLine}>{lastLine || 'Starting…'}</span>
      </div>
      <button class="sb-cancel" on:click={cancelNetwork}>Cancel</button>
    </div>
  {/if}
</div>

<style>
  .sb {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 10px;
  }
  .sb-buttons {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .sb-buttons button {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #cbd5e1;
    padding: 5px 14px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .sb-buttons button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }
  .sb-buttons button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .sb-sep {
    width: 1px;
    height: 18px;
    background: rgba(255, 255, 255, 0.1);
  }
  .sb-spacer {
    flex: 1;
  }
  .sb-branch {
    color: #fb923c;
    font-family: var(--font-mono, monospace);
    font-size: 0.78rem;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sb-progress {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    border-radius: 10px;
    background: rgba(59, 130, 246, 0.12);
    border: 1px solid rgba(59, 130, 246, 0.3);
    color: #bfdbfe;
    font-size: 0.8rem;
  }
  .sb-progress-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .sb-progress-text span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #93c5fd;
    font-family: var(--font-mono, monospace);
    font-size: 0.74rem;
  }
  .sb-cancel {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: 6px;
    color: #fca5a5;
    padding: 4px 12px;
    font-size: 0.78rem;
    cursor: pointer;
  }
</style>
