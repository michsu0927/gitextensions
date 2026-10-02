<script lang="ts">
  import { api } from '../lib/api';
  import { confirmDialog, openDialog } from '../lib/dialog';
  import { runNetwork } from '../lib/network';
  import { runOperation } from '../lib/operations';
  import type { RemoteInfo } from '../lib/types';
  import { loadRemoteBranches, loadRemotes } from '../lib/actions';
  import { remotesDialogOpen } from '../stores/ops';
  import { currentRepoPath } from '../stores/repo';

  let list: RemoteInfo[] = [];
  let loading = false;
  let error = '';
  let newName = '';
  let newUrl = '';
  let wasOpen = false;

  $: if ($remotesDialogOpen !== wasOpen) {
    wasOpen = $remotesDialogOpen;
    if (wasOpen) void reload();
  }

  async function reload(): Promise<void> {
    loading = true;
    error = '';
    try {
      list = await api.listRemotes($currentRepoPath);
    } catch (e) {
      error = String(e);
      list = [];
    } finally {
      loading = false;
    }
  }

  async function changed(): Promise<void> {
    await reload();
    void loadRemotes();
    void loadRemoteBranches();
  }

  async function run(label: string, action: () => Promise<string>): Promise<void> {
    await runOperation(label, action);
    await changed();
  }

  function add(): void {
    const name = newName.trim();
    const url = newUrl.trim();
    if (!name || !url) {
      error = 'Both a name and a URL are required.';
      return;
    }
    newName = '';
    newUrl = '';
    void run('Remote added.', () => api.addRemote($currentRepoPath, name, url));
  }

  function editUrl(remote: RemoteInfo, push: boolean): void {
    openDialog({
      title: `${push ? 'Push URL' : 'URL'} of '${remote.name}'`,
      fields: [
        { name: 'url', label: 'URL', kind: 'text', value: push ? (remote.push_url ?? remote.url) : remote.url, required: true },
      ],
      submitLabel: 'Save',
      onSubmit: (v) =>
        void run('URL updated.', () => api.setRemoteUrl($currentRepoPath, remote.name, String(v.url), push)),
    });
  }

  function rename(remote: RemoteInfo): void {
    openDialog({
      title: `Rename '${remote.name}'`,
      fields: [{ name: 'name', label: 'New name', kind: 'text', value: remote.name, required: true }],
      submitLabel: 'Rename',
      onSubmit: (v) =>
        void run('Remote renamed.', () => api.renameRemote($currentRepoPath, remote.name, String(v.name).trim())),
    });
  }

  function remove(remote: RemoteInfo): void {
    confirmDialog(
      `Remove '${remote.name}'?`,
      'Its remote-tracking branches are removed from this repository. The remote itself is not touched.',
      'Remove',
      () => void run('Remote removed.', () => api.removeRemote($currentRepoPath, remote.name)),
      true,
    );
  }

  function fetchRemote(remote: RemoteInfo): void {
    $remotesDialogOpen = false;
    runNetwork(`Fetched ${remote.name}.`, (opId) =>
      api.fetchRemote($currentRepoPath, opId, { remote: remote.name, prune: false }),
    );
  }

  function close(): void {
    $remotesDialogOpen = false;
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && $remotesDialogOpen) close();
  }
</script>

<svelte:window on:keydown={onKeyDown} />

{#if $remotesDialogOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="rd-overlay" on:click|self={close}>
    <div class="rd" role="dialog" aria-modal="true" aria-label="Remotes">
      <h3>Remotes</h3>
      {#if loading}
        <div class="rd-state"><div class="loading-spinner-mini"></div>Loading…</div>
      {:else if list.length === 0}
        <div class="rd-state">This repository has no remotes yet.</div>
      {:else}
        <div class="rd-list">
          {#each list as remote (remote.name)}
            <div class="rd-row">
              <div class="rd-info">
                <strong>{remote.name}</strong>
                <span class="rd-url" title={remote.url}>{remote.url}</span>
                {#if remote.push_url}<span class="rd-url" title={remote.push_url}>push: {remote.push_url}</span>{/if}
              </div>
              <div class="rd-actions">
                <button on:click={() => fetchRemote(remote)}>Fetch</button>
                <button on:click={() => editUrl(remote, false)}>URL…</button>
                <button on:click={() => editUrl(remote, true)}>Push URL…</button>
                <button on:click={() => rename(remote)}>Rename…</button>
                <button on:click={() => run('Pruned.', () => api.pruneRemote($currentRepoPath, remote.name))}>Prune</button>
                <button class="danger" on:click={() => remove(remote)}>Remove</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

      <div class="rd-add">
        <input placeholder="Name (e.g. origin)" bind:value={newName} />
        <input class="rd-add-url" placeholder="URL" bind:value={newUrl} on:keydown={(e) => e.key === 'Enter' && add()} />
        <button on:click={add}>Add remote</button>
      </div>
      {#if error}<div class="rd-error">{error}</div>{/if}

      <div class="rd-footer"><button on:click={close}>Close</button></div>
    </div>
  </div>
{/if}

<style>
  .rd-overlay {
    position: fixed;
    inset: 0;
    z-index: 1900;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(2, 6, 23, 0.65);
  }
  .rd {
    width: min(760px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 20px 22px;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
    color: #e5e7eb;
  }
  .rd h3 {
    margin: 0;
    font-size: 1.05rem;
  }
  .rd-state {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #94a3b8;
    padding: 8px 0;
  }
  .rd-list {
    overflow-y: auto;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: #0b0f19;
  }
  .rd-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.82rem;
  }
  .rd-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .rd-url {
    color: #94a3b8;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono, monospace);
    font-size: 0.74rem;
  }
  .rd-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: flex-end;
    max-width: 330px;
  }
  .rd button {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    color: #cbd5e1;
    padding: 3px 10px;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .rd button:hover {
    background: rgba(255, 255, 255, 0.12);
  }
  .rd button.danger {
    color: #fca5a5;
    border-color: rgba(239, 68, 68, 0.4);
  }
  .rd-add {
    display: flex;
    gap: 8px;
  }
  .rd-add input {
    background: #0b0f19;
    color: #e5e7eb;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 0.82rem;
    outline: none;
    width: 170px;
  }
  .rd-add .rd-add-url {
    flex: 1;
    width: auto;
  }
  .rd-error {
    color: #fca5a5;
    font-size: 0.8rem;
  }
  .rd-footer {
    display: flex;
    justify-content: flex-end;
  }
</style>
