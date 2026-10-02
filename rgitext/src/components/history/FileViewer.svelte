<script lang="ts">
  import type { FileContent } from '../../lib/types';
  import { formatSize } from '../../lib/format';

  export let content: FileContent | null = null;
  export let loading = false;
  export let error = '';
  export let path = '';

  const MAX_LINES = 5000;

  let showAll = false;

  $: if (content) showAll = false;
  $: lines = content && !content.is_binary ? content.text.split('\n') : [];
  // A trailing newline does not start another line.
  $: if (lines.length > 0 && lines[lines.length - 1] === '') lines = lines.slice(0, -1);
  $: shown = showAll ? lines : lines.slice(0, MAX_LINES);
  $: gutter = String(lines.length).length;
</script>

<div class="fv">
  {#if loading}
    <div class="fv-state"><div class="loading-spinner-mini"></div><span>Loading file…</span></div>
  {:else if error}
    <div class="fv-state fv-error">{error}</div>
  {:else if !content}
    <div class="fv-state">Select a file to view its content.</div>
  {:else if content.is_binary}
    <div class="fv-state">Binary file ({formatSize(content.size)}) — not shown.</div>
  {:else}
    {#if path}
      <div class="fv-title" title={path}>{path} <span class="fv-size">{formatSize(content.size)}</span></div>
    {/if}
    {#if content.truncated}
      <div class="fv-banner">File is larger than the viewer limit; only the beginning is shown.</div>
    {/if}
    <div class="fv-scroll">
      <table class="fv-table">
        <tbody>
          {#each shown as line, i}
            <tr>
              <td class="fv-no" style="min-width: {gutter + 2}ch">{i + 1}</td>
              <td class="fv-code">{line}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if !showAll && lines.length > MAX_LINES}
        <div class="fv-more">
          <button on:click={() => (showAll = true)}>Show {(lines.length - MAX_LINES).toLocaleString()} more lines</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .fv {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    background: #070a13;
  }
  .fv-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    flex: 1;
    color: #64748b;
    font-size: 0.85rem;
    padding: 24px;
    text-align: center;
  }
  .fv-error {
    color: #f87171;
  }
  .fv-title {
    padding: 6px 12px;
    background: #0c101d;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.8rem;
    color: #e5e7eb;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fv-size {
    color: #64748b;
    margin-left: 8px;
  }
  .fv-banner {
    padding: 4px 12px;
    background: rgba(234, 179, 8, 0.1);
    color: #facc15;
    font-size: 0.75rem;
  }
  .fv-scroll {
    flex: 1;
    overflow: auto;
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 0.78rem;
    line-height: 1.5;
  }
  .fv-table {
    border-collapse: collapse;
  }
  .fv-no {
    text-align: right;
    padding: 0 10px;
    color: #475569;
    user-select: none;
    vertical-align: top;
    border-right: 1px solid rgba(255, 255, 255, 0.05);
    position: sticky;
    left: 0;
    background: #070a13;
  }
  .fv-code {
    white-space: pre;
    padding: 0 12px;
    color: #cbd5e1;
  }
  .fv-more {
    display: flex;
    justify-content: center;
    padding: 12px;
  }
  .fv-more button {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    color: #cbd5e1;
    padding: 4px 10px;
    cursor: pointer;
  }
</style>
