<script lang="ts">
  import { api } from '../../lib/api';
  import { formatRelative, shortHash } from '../../lib/format';
  import type { BlameLine } from '../../lib/types';

  export let repoPath: string;
  export let path: string;
  /** Revision to blame; null blames the working tree (HEAD plus uncommitted changes). */
  export let rev: string | null = null;
  export let onSelectHash: (hash: string) => void;

  const MAX_LINES = 5000;

  interface Row {
    line: BlameLine;
    startsGroup: boolean;
    shade: boolean;
  }

  let lines: BlameLine[] = [];
  let loading = false;
  let error = '';
  let ignoreWhitespace = false;
  let showAll = false;
  let requestId = 0;

  $: if (repoPath && path) void load(repoPath, path, rev, ignoreWhitespace);
  $: rows = toRows(showAll ? lines : lines.slice(0, MAX_LINES));

  async function load(repo: string, file: string, revision: string | null, ignoreWs: boolean): Promise<void> {
    const id = ++requestId;
    loading = true;
    error = '';
    showAll = false;
    try {
      const result = await api.getBlame(repo, file, revision, ignoreWs);
      if (id === requestId) lines = result;
    } catch (e) {
      if (id === requestId) {
        lines = [];
        error = String(e);
      }
    } finally {
      if (id === requestId) loading = false;
    }
  }

  // Consecutive lines of the same commit form one group; the info is shown once per group
  // and groups alternate in shade.
  function toRows(source: BlameLine[]): Row[] {
    const out: Row[] = [];
    let shade = false;
    let previous = '';
    for (const line of source) {
      const startsGroup = line.hash !== previous;
      if (startsGroup) shade = !shade;
      previous = line.hash;
      out.push({ line, startsGroup, shade });
    }
    return out;
  }

  function isUncommitted(hash: string): boolean {
    return /^0+$/.test(hash);
  }
</script>

<div class="bv">
  <div class="bv-toolbar">
    <span class="bv-path" title={path}>{path}{rev ? ` @ ${shortHash(rev)}` : ''}</span>
    <label><input type="checkbox" bind:checked={ignoreWhitespace} /> Ignore whitespace</label>
  </div>

  {#if loading}
    <div class="bv-state"><div class="loading-spinner-mini"></div><span>Running git blame…</span></div>
  {:else if error}
    <div class="bv-state bv-error">{error}</div>
  {:else}
    <div class="bv-scroll">
      <table class="bv-table">
        <tbody>
          {#each rows as { line, startsGroup, shade } (line.line_no)}
            <tr class:shade class:group-start={startsGroup}>
              <td class="bv-info">
                {#if startsGroup}
                  {#if isUncommitted(line.hash)}
                    <span class="bv-uncommitted">Not committed yet</span>
                  {:else}
                    <button class="bv-hash monospaced" title={line.summary} on:click={() => onSelectHash(line.hash)}>
                      {shortHash(line.hash)}
                    </button>
                    <span class="bv-author" title="{line.author} <{line.email}>">{line.author}</span>
                    <span class="bv-date">{formatRelative(line.author_date)}</span>
                  {/if}
                {/if}
              </td>
              <td class="bv-no">{line.line_no}</td>
              <td class="bv-code">{line.text}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if !showAll && lines.length > MAX_LINES}
        <div class="bv-more">
          <button on:click={() => (showAll = true)}>Show {(lines.length - MAX_LINES).toLocaleString()} more lines</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .bv {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    background: #070a13;
  }
  .bv-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 12px;
    background: #0c101d;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.78rem;
    color: #94a3b8;
  }
  .bv-path {
    color: #e5e7eb;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bv-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    flex: 1;
    color: #64748b;
    padding: 24px;
  }
  .bv-error {
    color: #f87171;
  }
  .bv-scroll {
    flex: 1;
    overflow: auto;
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 0.78rem;
    line-height: 1.5;
  }
  .bv-table {
    border-collapse: collapse;
    width: 100%;
  }
  .bv-table tr.shade {
    background: rgba(255, 255, 255, 0.025);
  }
  .bv-table tr.group-start td {
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }
  .bv-info {
    width: 330px;
    min-width: 330px;
    max-width: 330px;
    padding: 0 8px;
    white-space: nowrap;
    overflow: hidden;
    color: #94a3b8;
    font-family: var(--font-sans, 'Outfit', sans-serif);
    vertical-align: top;
  }
  .bv-hash {
    background: none;
    border: none;
    color: #fb923c;
    cursor: pointer;
    padding: 0;
    margin-right: 8px;
  }
  .bv-hash:hover {
    text-decoration: underline;
  }
  .bv-author {
    display: inline-block;
    max-width: 130px;
    overflow: hidden;
    text-overflow: ellipsis;
    vertical-align: bottom;
    margin-right: 8px;
  }
  .bv-date {
    color: #64748b;
  }
  .bv-uncommitted {
    color: #facc15;
  }
  .bv-no {
    text-align: right;
    padding: 0 10px;
    color: #475569;
    user-select: none;
    vertical-align: top;
    border-right: 1px solid rgba(255, 255, 255, 0.05);
    min-width: 4ch;
  }
  .bv-code {
    white-space: pre;
    padding: 0 12px;
    color: #cbd5e1;
  }
  .bv-more {
    display: flex;
    justify-content: center;
    padding: 12px;
  }
  .bv-more button {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    color: #cbd5e1;
    padding: 4px 10px;
    cursor: pointer;
  }
</style>
