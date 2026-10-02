<script lang="ts">
  import { api } from '../lib/api';
  import { shortHash } from '../lib/format';
  import { runOperation } from '../lib/operations';
  import type { RebaseAction, TodoItem } from '../lib/types';
  import { currentRepoPath } from '../stores/repo';
  import { rebaseDialogOnto } from '../stores/ops';

  interface Row {
    hash: string;
    subject: string;
    action: RebaseAction;
    message: string;
  }

  const ACTIONS: { value: RebaseAction; label: string }[] = [
    { value: 'pick', label: 'pick' },
    { value: 'reword', label: 'reword' },
    { value: 'edit', label: 'edit (stop to amend)' },
    { value: 'squash', label: 'squash (into previous)' },
    { value: 'fixup', label: 'fixup (into previous, drop message)' },
    { value: 'drop', label: 'drop' },
  ];

  let rows: Row[] = [];
  let loading = false;
  let error = '';
  let autostash = true;
  let loadedFor: string | null = null;

  $: onto = $rebaseDialogOnto;
  $: if (onto !== loadedFor) {
    loadedFor = onto;
    rows = [];
    error = '';
    if (onto) void load(onto);
  }
  $: invalid = firstProblem(rows);

  async function load(target: string): Promise<void> {
    loading = true;
    try {
      const todo = await api.getRebaseTodo($currentRepoPath, target);
      if ($rebaseDialogOnto !== target) return;
      rows = todo.map((t) => ({ ...t, action: 'pick' as RebaseAction, message: t.subject }));
      if (rows.length === 0) error = 'There are no commits to rebase: the current branch has nothing on top of the target.';
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  /** The first line that is not squashed into something cannot be squash/fixup. */
  function firstProblem(list: Row[]): string {
    const first = list.find((r) => r.action !== 'drop');
    if (first && (first.action === 'squash' || first.action === 'fixup')) {
      return 'The first commit cannot be squashed or fixed up.';
    }
    if (list.some((r) => r.action === 'reword' && !r.message.trim())) return 'A reworded commit needs a message.';
    if (list.length > 0 && list.every((r) => r.action === 'drop')) return 'All commits are dropped.';
    return '';
  }

  function move(index: number, delta: number): void {
    const target = index + delta;
    if (target < 0 || target >= rows.length) return;
    const next = [...rows];
    [next[index], next[target]] = [next[target], next[index]];
    rows = next;
  }

  function setAction(index: number, value: string): void {
    const action = value as RebaseAction;
    rows = rows.map((r, i) => (i === index ? { ...r, action } : r));
  }

  function setMessage(index: number, message: string): void {
    rows = rows.map((r, i) => (i === index ? { ...r, message } : r));
  }

  function close(): void {
    rebaseDialogOnto.set(null);
  }

  function start(): void {
    const target = onto;
    if (!target || invalid) return;
    const items: TodoItem[] = rows.map((r) => ({
      action: r.action,
      hash: r.hash,
      message: r.action === 'reword' ? r.message.trim() : undefined,
    }));
    close();
    void runOperation('Rebase finished.', () =>
      api.rebaseInteractive($currentRepoPath, target, items, autostash),
    );
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === 'Escape') close();
  }
</script>

<svelte:window on:keydown={onKeyDown} />

{#if onto}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="rt-overlay" on:click|self={close}>
    <div class="rt" role="dialog" aria-modal="true" aria-label="Interactive rebase">
      <h3>Interactive rebase onto {shortHash(onto)}</h3>
      <p class="rt-help">
        Commits are listed oldest first, in the order they will be applied. Reorder them, change what happens to each one, or drop them.
      </p>

      {#if loading}
        <div class="rt-state"><div class="loading-spinner-mini"></div>Loading commits…</div>
      {:else if error}
        <div class="rt-state rt-error">{error}</div>
      {:else}
        <div class="rt-list">
          {#each rows as row, i (row.hash)}
            <div class="rt-row" class:dropped={row.action === 'drop'}>
              <div class="rt-move">
                <button aria-label="Move up" disabled={i === 0} on:click={() => move(i, -1)}>▲</button>
                <button aria-label="Move down" disabled={i === rows.length - 1} on:click={() => move(i, 1)}>▼</button>
              </div>
              <select value={row.action} on:change={(e) => setAction(i, e.currentTarget.value)}>
                {#each ACTIONS as a}<option value={a.value}>{a.label}</option>{/each}
              </select>
              <span class="rt-hash monospaced">{shortHash(row.hash)}</span>
              {#if row.action === 'reword'}
                <input
                  class="rt-message"
                  value={row.message}
                  on:input={(e) => setMessage(i, e.currentTarget.value)}
                  aria-label="New message"
                />
              {:else}
                <span class="rt-subject" title={row.subject}>{row.subject}</span>
              {/if}
            </div>
          {/each}
        </div>
        <label class="rt-check">
          <input type="checkbox" bind:checked={autostash} />
          Stash local changes and re-apply them afterwards
        </label>
        {#if invalid}<div class="rt-error">{invalid}</div>{/if}
      {/if}

      <div class="rt-actions">
        <button on:click={close}>Cancel</button>
        <button class="primary" disabled={loading || !!error || !!invalid || rows.length === 0} on:click={start}>
          Start rebase
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .rt-overlay {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(2, 6, 23, 0.65);
  }
  .rt {
    width: min(760px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 20px 22px;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
    color: #e5e7eb;
  }
  .rt h3 {
    margin: 0;
    font-size: 1.05rem;
  }
  .rt-help {
    margin: 0;
    color: #94a3b8;
    font-size: 0.82rem;
  }
  .rt-state {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 0;
    color: #94a3b8;
  }
  .rt-error {
    color: #fca5a5;
    font-size: 0.82rem;
  }
  .rt-list {
    overflow-y: auto;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: #0b0f19;
  }
  .rt-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    font-size: 0.82rem;
  }
  .rt-row.dropped {
    opacity: 0.45;
    text-decoration: line-through;
  }
  .rt-move {
    display: flex;
    flex-direction: column;
  }
  .rt-move button {
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 0.6rem;
    line-height: 1.1;
    cursor: pointer;
    padding: 0 4px;
  }
  .rt-move button:disabled {
    opacity: 0.25;
    cursor: default;
  }
  .rt select,
  .rt-message {
    background: #0f172a;
    color: #e5e7eb;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 0.8rem;
  }
  .rt select {
    width: 190px;
  }
  .rt-hash {
    color: #fb923c;
    font-size: 0.75rem;
  }
  .rt-subject {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rt-message {
    flex: 1;
    min-width: 0;
  }
  .rt-check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 0.82rem;
    color: #cbd5e1;
  }
  .rt-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .rt-actions button {
    border-radius: 8px;
    padding: 7px 16px;
    font-size: 0.85rem;
    cursor: pointer;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: transparent;
    color: #cbd5e1;
  }
  .rt-actions .primary {
    background: #ea580c;
    border-color: #ea580c;
    color: #fff;
    font-weight: 600;
  }
  .rt-actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
