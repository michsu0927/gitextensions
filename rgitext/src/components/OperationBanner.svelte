<script lang="ts">
  import { controlOperation, dismissOperationResult } from '../lib/operations';
  import type { OperationKind } from '../lib/types';
  import { conflictsDialogOpen, opBusy, opError, opMessage, repoState } from '../stores/ops';

  const TITLES: Record<OperationKind, string> = {
    merge: 'Merge in progress',
    rebase: 'Rebase in progress',
    'cherry-pick': 'Cherry-pick in progress',
    revert: 'Revert in progress',
    bisect: 'Bisect in progress',
  };

  $: op = $repoState.operation;
  $: conflicts = $repoState.conflicted.length;
  $: canSkip = op === 'rebase' || op === 'cherry-pick' || op === 'revert';
  $: canContinue = op !== null && op !== 'bisect';
</script>

{#if op}
  <div class="ob ob-progress" role="status">
    <div class="ob-text">
      <strong>{TITLES[op]}</strong>
      {#if conflicts > 0}
        <span>
          · {conflicts} file{conflicts === 1 ? '' : 's'} with conflicts. Resolve them, then continue.
        </span>
      {:else if canContinue}
        <span>· No conflicts left. Continue to finish.</span>
      {/if}
    </div>
    <div class="ob-actions">
      {#if conflicts > 0}
        <button class="primary" on:click={() => ($conflictsDialogOpen = true)}>Resolve conflicts…</button>
      {/if}
      {#if canContinue}
        <button disabled={$opBusy} on:click={() => controlOperation('continue')}>Continue</button>
      {/if}
      {#if canSkip}
        <button disabled={$opBusy} on:click={() => controlOperation('skip')}>Skip</button>
      {/if}
      <button class="danger" disabled={$opBusy} on:click={() => controlOperation('abort')}>
        {op === 'bisect' ? 'Reset bisect' : 'Abort'}
      </button>
    </div>
  </div>
{/if}

{#if $opBusy}
  <div class="ob ob-info" role="status"><div class="loading-spinner-mini"></div><span>Working…</span></div>
{/if}
{#if $opError}
  <div class="ob ob-error" role="alert">
    <pre>{$opError}</pre>
    <button aria-label="Dismiss" on:click={dismissOperationResult}>×</button>
  </div>
{:else if $opMessage}
  <div class="ob ob-ok" role="status">
    <pre>{$opMessage}</pre>
    <button aria-label="Dismiss" on:click={dismissOperationResult}>×</button>
  </div>
{/if}

<style>
  .ob {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 0 10px;
    padding: 8px 14px;
    border-radius: 10px;
    font-size: 0.82rem;
  }
  .ob pre {
    flex: 1;
    margin: 0;
    max-height: 140px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
  }
  .ob-progress {
    justify-content: space-between;
    background: rgba(234, 179, 8, 0.12);
    border: 1px solid rgba(234, 179, 8, 0.35);
    color: #fde68a;
  }
  .ob-text {
    flex: 1;
    min-width: 0;
  }
  .ob-actions {
    display: flex;
    gap: 8px;
  }
  .ob-actions button {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 6px;
    color: inherit;
    padding: 4px 12px;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .ob-actions button.primary {
    background: #ea580c;
    border-color: #ea580c;
    color: #fff;
    font-weight: 600;
  }
  .ob-actions button.danger {
    border-color: rgba(239, 68, 68, 0.5);
    color: #fca5a5;
  }
  .ob-actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .ob-info {
    background: rgba(59, 130, 246, 0.12);
    color: #93c5fd;
  }
  .ob-ok {
    background: rgba(16, 185, 129, 0.12);
    color: #6ee7b7;
  }
  .ob-error {
    background: rgba(239, 68, 68, 0.12);
    color: #fca5a5;
  }
  .ob-ok button,
  .ob-error button {
    background: none;
    border: none;
    color: inherit;
    font-size: 1.1rem;
    cursor: pointer;
  }
</style>
