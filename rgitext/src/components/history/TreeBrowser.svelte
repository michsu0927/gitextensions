<script lang="ts">
  import { api } from '../../lib/api';
  import { formatSize } from '../../lib/format';
  import type { FileContent, TreeEntry } from '../../lib/types';
  import FileViewer from './FileViewer.svelte';

  export let repoPath: string;
  /** Commit (or any revision) whose tree is browsed. */
  export let rev: string;
  export let onShowHistory: (path: string) => void;
  export let onBlame: (path: string) => void;

  interface FlatEntry {
    entry: TreeEntry;
    depth: number;
  }

  let dirs: Record<string, TreeEntry[] | 'loading'> = {};
  let loadedRev = '';
  let rootError = '';
  let selectedPath = '';
  let content: FileContent | null = null;
  let contentLoading = false;
  let contentError = '';

  $: if (rev && repoPath && rev !== loadedRev) reset(rev);
  $: flat = flatten(dirs);

  function reset(newRev: string): void {
    loadedRev = newRev;
    dirs = {};
    rootError = '';
    selectedPath = '';
    content = null;
    contentError = '';
    void loadDir('');
  }

  async function loadDir(dir: string): Promise<void> {
    const forRev = loadedRev;
    dirs = { ...dirs, [dir]: 'loading' };
    try {
      const entries = await api.getTree(repoPath, forRev, dir);
      if (forRev !== loadedRev) return;
      dirs = { ...dirs, [dir]: entries };
    } catch (e) {
      if (forRev !== loadedRev) return;
      const { [dir]: _removed, ...rest } = dirs;
      dirs = rest;
      if (dir === '') rootError = String(e);
    }
  }

  function flatten(source: Record<string, TreeEntry[] | 'loading'>): FlatEntry[] {
    const out: FlatEntry[] = [];
    const walk = (dir: string, depth: number) => {
      const entries = source[dir];
      if (!Array.isArray(entries)) return;
      for (const entry of entries) {
        out.push({ entry, depth });
        if (entry.kind === 'tree' && source[entry.path]) walk(entry.path, depth + 1);
      }
    };
    walk('', 0);
    return out;
  }

  function toggleDir(entry: TreeEntry): void {
    if (dirs[entry.path]) {
      const copy = { ...dirs };
      for (const key of Object.keys(copy)) {
        if (key === entry.path || key.startsWith(entry.path + '/')) delete copy[key];
      }
      dirs = copy;
    } else {
      void loadDir(entry.path);
    }
  }

  async function openFile(entry: TreeEntry): Promise<void> {
    selectedPath = entry.path;
    content = null;
    contentError = '';
    contentLoading = true;
    const forPath = entry.path;
    const forRev = loadedRev;
    try {
      const result = await api.getFileContent(repoPath, entry.path, forRev);
      if (selectedPath === forPath && forRev === loadedRev) content = result;
    } catch (e) {
      if (selectedPath === forPath) contentError = String(e);
    } finally {
      if (selectedPath === forPath) contentLoading = false;
    }
  }

  function onClick(entry: TreeEntry): void {
    if (entry.kind === 'tree') toggleDir(entry);
    else if (entry.kind === 'blob') void openFile(entry);
  }

  function icon(entry: TreeEntry): string {
    if (entry.kind === 'tree') return dirs[entry.path] ? '▾' : '▸';
    return entry.kind === 'commit' ? '◈' : '·';
  }
</script>

<div class="tb">
  <div class="tb-tree">
    {#if rootError}
      <div class="tb-state tb-error">{rootError}</div>
    {:else if dirs[''] === 'loading' && flat.length === 0}
      <div class="tb-state"><div class="loading-spinner-mini"></div><span>Loading…</span></div>
    {:else}
      {#each flat as { entry, depth } (entry.path)}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <div
          class="tb-row"
          class:selected={entry.path === selectedPath}
          class:dir={entry.kind === 'tree'}
          role="treeitem"
          aria-selected={entry.path === selectedPath}
          tabindex="-1"
          style="padding-left: {8 + depth * 14}px"
          on:click={() => onClick(entry)}
        >
          <span class="tb-icon">{icon(entry)}</span>
          <span class="tb-name" title={entry.path}>{entry.name}</span>
          {#if dirs[entry.path] === 'loading'}<span class="tb-size">…</span>{/if}
          {#if entry.kind === 'blob'}<span class="tb-size">{formatSize(entry.size)}</span>{/if}
        </div>
      {/each}
    {/if}
  </div>

  <div class="tb-view">
    {#if selectedPath}
      <div class="tb-actions">
        <button on:click={() => onShowHistory(selectedPath)}>File history</button>
        <button on:click={() => onBlame(selectedPath)}>Blame</button>
      </div>
    {/if}
    <FileViewer {content} loading={contentLoading} error={contentError} path={selectedPath} />
  </div>
</div>

<style>
  .tb {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .tb-tree {
    width: 280px;
    flex-shrink: 0;
    overflow: auto;
    border-right: 1px solid rgba(255, 255, 255, 0.05);
    padding: 4px 0;
    font-size: 0.8rem;
  }
  .tb-state {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px;
    color: #64748b;
  }
  .tb-error {
    color: #f87171;
  }
  .tb-row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding-right: 8px;
    cursor: default;
    color: #cbd5e1;
    white-space: nowrap;
  }
  .tb-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .tb-row.selected {
    background: rgba(234, 88, 12, 0.16);
  }
  .tb-row.dir {
    color: #e2e8f0;
    font-weight: 500;
  }
  .tb-icon {
    width: 12px;
    text-align: center;
    color: #64748b;
  }
  .tb-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tb-size {
    color: #475569;
    font-size: 0.7rem;
  }
  .tb-view {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .tb-actions {
    display: flex;
    gap: 8px;
    padding: 6px 12px;
    background: #0c101d;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }
  .tb-actions button {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    color: #cbd5e1;
    padding: 3px 10px;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .tb-actions button:hover {
    background: rgba(255, 255, 255, 0.06);
  }
</style>
