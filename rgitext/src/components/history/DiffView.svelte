<script lang="ts">
  import type { DiffFile, DiffHunk, DiffLine, HunkSelection } from '../../lib/types';
  import { copyText } from '../../lib/format';
  import { diffMode, diffOptions } from '../../stores/history';

  export let files: DiffFile[] = [];
  export let loading = false;
  /** Hide the per-file header when the caller already shows the file name. */
  export let hideFileHeader = false;
  /**
   * Actions that can be applied to single hunks or selected lines (stage, unstage, discard).
   * When given, the diff is shown inline with a checkbox in front of every changed line.
   */
  export let actions: DiffAction[] = [];

  interface DiffAction {
    label: string;
    danger?: boolean;
    run: (selections: HunkSelection[]) => void;
  }

  // Very large diffs are cut to keep the DOM responsive; the rest can be revealed on demand.
  const MAX_LINES = 3000;

  interface SplitRow {
    left: DiffLine | null;
    right: DiffLine | null;
  }

  let showAll = false;
  let copied = '';
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  // Checked lines, as `hunk:line` keys (indices into the diff of the first file).
  let selected = new Set<string>();

  $: if (files) {
    showAll = false;
    selected = new Set();
  }
  $: selectable = actions.length > 0;
  $: inline = $diffMode === 'inline' || selectable;

  const lineKey = (hunk: number, line: number): string => `${hunk}:${line}`;

  function changeIndices(hunk: DiffHunk): number[] {
    const out: number[] = [];
    hunk.lines.forEach((l, i) => {
      if (l.kind === 'add' || l.kind === 'del') out.push(i);
    });
    return out;
  }

  function toggleLine(hunk: number, line: number): void {
    const next = new Set(selected);
    const k = lineKey(hunk, line);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    selected = next;
  }

  function hunkFullySelected(hunk: number, h: DiffHunk, current: Set<string>): boolean {
    const indices = changeIndices(h);
    return indices.length > 0 && indices.every((i) => current.has(lineKey(hunk, i)));
  }

  function toggleHunk(hunk: number, h: DiffHunk): void {
    const all = hunkFullySelected(hunk, h, selected);
    const next = new Set(selected);
    for (const i of changeIndices(h)) {
      if (all) next.delete(lineKey(hunk, i));
      else next.add(lineKey(hunk, i));
    }
    selected = next;
  }

  function checkedSelections(): HunkSelection[] {
    const byHunk = new Map<number, number[]>();
    for (const key of selected) {
      const [h, l] = key.split(':').map(Number);
      byHunk.set(h, [...(byHunk.get(h) ?? []), l]);
    }
    return [...byHunk.entries()].map(([hunk, lines]) => ({ hunk, lines: lines.sort((a, b) => a - b) }));
  }
  $: totalLines = files.reduce((n, f) => n + f.hunks.reduce((m, h) => m + h.lines.length, 0), 0);
  $: view = showAll || totalLines <= MAX_LINES ? files : truncate(files, MAX_LINES);
  $: hiddenLines = showAll ? 0 : Math.max(0, totalLines - MAX_LINES);

  function truncate(source: DiffFile[], budget: number): DiffFile[] {
    const out: DiffFile[] = [];
    let left = budget;
    for (const file of source) {
      if (left <= 0) break;
      const hunks: DiffHunk[] = [];
      for (const hunk of file.hunks) {
        if (left <= 0) break;
        const lines = hunk.lines.slice(0, left);
        left -= lines.length;
        hunks.push({ ...hunk, lines });
      }
      out.push({ ...file, hunks });
    }
    return out;
  }

  /** Pairs removed and added lines side by side. */
  function toSplitRows(lines: DiffLine[]): SplitRow[] {
    const rows: SplitRow[] = [];
    let dels: DiffLine[] = [];
    let adds: DiffLine[] = [];
    const flush = () => {
      const n = Math.max(dels.length, adds.length);
      for (let i = 0; i < n; i++) rows.push({ left: dels[i] ?? null, right: adds[i] ?? null });
      dels = [];
      adds = [];
    };
    for (const line of lines) {
      if (line.kind === 'del') dels.push(line);
      else if (line.kind === 'add') adds.push(line);
      else {
        flush();
        if (line.kind === 'context') rows.push({ left: line, right: line });
      }
    }
    flush();
    return rows;
  }

  function fileTitle(file: DiffFile): string {
    if (file.change === 'renamed' || file.change === 'copied') {
      return `${file.old_path ?? ''} → ${file.new_path ?? ''}`;
    }
    return file.new_path ?? file.old_path ?? '';
  }

  const CHANGE_LABEL: Record<DiffFile['change'], string> = {
    modified: 'Modified',
    added: 'Added',
    deleted: 'Deleted',
    renamed: 'Renamed',
    copied: 'Copied',
  };

  async function copyCode(): Promise<void> {
    const text = files
      .flatMap((f) => f.hunks.flatMap((h) => h.lines.filter((l) => l.kind !== 'noeol' && l.kind !== 'del')))
      .map((l) => l.text)
      .join('\n');
    copied = (await copyText(text)) ? 'Copied!' : 'Failed';
    if (copiedTimer) clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied = ''), 1500);
  }
</script>

<div class="dv">
  <div class="dv-toolbar">
    <div class="dv-seg" role="group" aria-label="Diff layout">
      <button class:active={$diffMode === 'inline'} on:click={() => diffMode.set('inline')}>Inline</button>
      <button class:active={$diffMode === 'split'} on:click={() => diffMode.set('split')}>Side by side</button>
    </div>
    <label class="dv-check">
      <input
        type="checkbox"
        checked={$diffOptions.ignoreWhitespace}
        on:change={(e) => diffOptions.update((o) => ({ ...o, ignoreWhitespace: e.currentTarget.checked }))}
      />
      Ignore whitespace
    </label>
    <label class="dv-check">
      Context
      <select
        value={$diffOptions.contextLines}
        on:change={(e) => diffOptions.update((o) => ({ ...o, contextLines: Number(e.currentTarget.value) }))}
      >
        {#each [0, 1, 3, 5, 10, 25, 100] as n}
          <option value={n}>{n}</option>
        {/each}
      </select>
    </label>
    {#if selectable}
      <span class="dv-selected">{selected.size} line{selected.size === 1 ? '' : 's'} selected</span>
      {#each actions as action}
        <button
          class="dv-action"
          class:danger={action.danger}
          disabled={selected.size === 0}
          on:click={() => action.run(checkedSelections())}
        >
          {action.label} selected
        </button>
      {/each}
    {/if}
    <span class="dv-spacer"></span>
    <button class="dv-copy" title="Copy the new version of the changed code (without +/-)" on:click={copyCode}>
      Copy {#if copied}<span class="dv-copied">{copied}</span>{/if}
    </button>
  </div>

  <div class="dv-body">
    {#if loading}
      <div class="dv-empty"><div class="loading-spinner-mini"></div><span>Fetching code changes…</span></div>
    {:else if files.length === 0}
      <div class="dv-empty">No changes.</div>
    {:else}
      {#each view as file}
        <div class="dv-file">
          {#if !hideFileHeader}
            <div class="dv-file-header">
              <span class="dv-change dv-change-{file.change}">{CHANGE_LABEL[file.change]}</span>
              <span class="dv-path" title={fileTitle(file)}>{fileTitle(file)}</span>
              <span class="dv-stat dv-stat-add">+{file.additions}</span>
              <span class="dv-stat dv-stat-del">−{file.deletions}</span>
            </div>
          {/if}
          {#if file.is_binary}
            <div class="dv-note">Binary file not shown.</div>
          {:else if file.hunks.length === 0}
            <div class="dv-note">No content changes (mode or rename only).</div>
          {/if}
          {#each file.hunks as hunk, hunkIndex}
            <div class="dv-hunk-header">
              {#if selectable}
                <input
                  type="checkbox"
                  title="Select all changed lines of this hunk"
                  checked={hunkFullySelected(hunkIndex, hunk, selected)}
                  on:change={() => toggleHunk(hunkIndex, hunk)}
                />
              {/if}
              <span class="dv-hunk-text">{hunk.header}</span>
              {#if selectable}
                <span class="dv-hunk-actions">
                  {#each actions as action}
                    <button
                      class="dv-action"
                      class:danger={action.danger}
                      on:click={() => action.run([{ hunk: hunkIndex }])}
                    >
                      {action.label} hunk
                    </button>
                  {/each}
                </span>
              {/if}
            </div>
            {#if inline}
              <table class="dv-table">
                <colgroup>
                  {#if selectable}<col class="dv-c-sel" />{/if}<col class="dv-c-no" /><col class="dv-c-no" /><col />
                </colgroup>
                <tbody>
                  {#each hunk.lines as line, lineIndex}
                    <tr class="dv-{line.kind}">
                      {#if selectable}
                        <td class="dv-sel">
                          {#if line.kind === 'add' || line.kind === 'del'}
                            <input
                              type="checkbox"
                              checked={selected.has(lineKey(hunkIndex, lineIndex))}
                              on:change={() => toggleLine(hunkIndex, lineIndex)}
                            />
                          {/if}
                        </td>
                      {/if}
                      <td class="dv-no">{line.old_no ?? ''}</td>
                      <td class="dv-no">{line.new_no ?? ''}</td>
                      <td class="dv-code"
                        ><span class="dv-mark">{line.kind === 'add' ? '+' : line.kind === 'del' ? '-' : ' '}</span
                        >{line.text}</td
                      >
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <table class="dv-table dv-split">
                <colgroup><col class="dv-c-no" /><col /><col class="dv-c-no" /><col /></colgroup>
                <tbody>
                  {#each toSplitRows(hunk.lines) as row}
                    <tr>
                      <td class="dv-no {row.left ? 'dv-' + row.left.kind : 'dv-empty-cell'}">{row.left?.old_no ?? ''}</td>
                      <td class="dv-code {row.left ? 'dv-' + row.left.kind : 'dv-empty-cell'}">{row.left?.text ?? ''}</td>
                      <td class="dv-no dv-sep {row.right ? 'dv-' + row.right.kind : 'dv-empty-cell'}">{row.right?.new_no ?? ''}</td>
                      <td class="dv-code {row.right ? 'dv-' + row.right.kind : 'dv-empty-cell'}">{row.right?.text ?? ''}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          {/each}
        </div>
      {/each}
      {#if hiddenLines > 0}
        <div class="dv-more">
          <button on:click={() => (showAll = true)}>Show {hiddenLines.toLocaleString()} more lines</button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .dv {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    background: #070a13;
  }
  .dv-toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 6px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    background: #0c101d;
    font-size: 0.78rem;
    color: #94a3b8;
    flex-shrink: 0;
  }
  .dv-spacer {
    flex: 1;
  }
  .dv-seg {
    display: inline-flex;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    overflow: hidden;
  }
  .dv-seg button,
  .dv-copy,
  .dv-more button {
    background: transparent;
    border: none;
    color: #cbd5e1;
    padding: 4px 10px;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .dv-seg button.active {
    background: rgba(234, 88, 12, 0.2);
    color: #fb923c;
  }
  .dv-copy {
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
  }
  .dv-copy:hover,
  .dv-more button:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .dv-copied {
    color: #34d399;
    margin-left: 4px;
  }
  .dv-check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dv-check select {
    background: #0b0f19;
    color: #cbd5e1;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 1px 4px;
  }
  .dv-body {
    flex: 1;
    overflow: auto;
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 0.78rem;
    line-height: 1.5;
  }
  .dv-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 40px;
    color: #64748b;
    font-family: inherit;
  }
  .dv-file {
    margin-bottom: 14px;
  }
  .dv-file-header {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px;
    background: #111827;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    font-family: var(--font-sans, 'Outfit', sans-serif);
    font-size: 0.8rem;
  }
  .dv-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #e5e7eb;
  }
  .dv-change {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.08);
    color: #94a3b8;
  }
  .dv-change-added { background: rgba(16, 185, 129, 0.15); color: #34d399; }
  .dv-change-deleted { background: rgba(239, 68, 68, 0.15); color: #f87171; }
  .dv-change-renamed,
  .dv-change-copied { background: rgba(59, 130, 246, 0.15); color: #60a5fa; }
  .dv-stat-add { color: #34d399; }
  .dv-stat-del { color: #f87171; }
  .dv-note {
    padding: 10px 14px;
    color: #64748b;
    font-family: var(--font-sans, 'Outfit', sans-serif);
  }
  .dv-selected {
    color: #94a3b8;
  }
  .dv-action {
    background: rgba(59, 130, 246, 0.15);
    border: 1px solid rgba(59, 130, 246, 0.35);
    border-radius: 6px;
    color: #93c5fd;
    padding: 3px 10px;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .dv-action.danger {
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.35);
    color: #fca5a5;
  }
  .dv-action:hover:not(:disabled) {
    filter: brightness(1.25);
  }
  .dv-action:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .dv-hunk-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 12px;
    background: rgba(59, 130, 246, 0.07);
    color: #60a5fa;
    white-space: pre;
  }
  .dv-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }
  .dv-c-no {
    width: 52px;
  }
  .dv-c-sel {
    width: 28px;
  }
  .dv-sel {
    text-align: center;
    vertical-align: middle;
    user-select: none;
  }
  .dv-hunk-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dv-hunk-actions {
    display: inline-flex;
    gap: 6px;
  }
  .dv-no {
    text-align: right;
    padding: 0 8px;
    color: #475569;
    user-select: none;
    vertical-align: top;
    border-right: 1px solid rgba(255, 255, 255, 0.04);
  }
  .dv-code {
    white-space: pre;
    padding: 0 8px;
    color: #cbd5e1;
    overflow: hidden;
    vertical-align: top;
  }
  .dv-mark {
    display: inline-block;
    width: 1.2ch;
    margin-right: 0.6ch;
    color: #64748b;
    user-select: none;
  }
  .dv-add { background: rgba(16, 185, 129, 0.1); }
  .dv-add .dv-code { color: #86efac; }
  .dv-add .dv-mark { color: #34d399; }
  .dv-del { background: rgba(239, 68, 68, 0.1); }
  .dv-del .dv-code { color: #fca5a5; }
  .dv-del .dv-mark { color: #f87171; }
  .dv-noeol { color: #64748b; font-style: italic; }
  td.dv-add { background: rgba(16, 185, 129, 0.1); }
  td.dv-del { background: rgba(239, 68, 68, 0.1); }
  td.dv-add.dv-code { color: #86efac; }
  td.dv-del.dv-code { color: #fca5a5; }
  td.dv-empty-cell { background: rgba(255, 255, 255, 0.02); }
  .dv-split .dv-sep {
    border-left: 1px solid rgba(255, 255, 255, 0.08);
  }
  .dv-more {
    display: flex;
    justify-content: center;
    padding: 12px;
  }
  .dv-more button {
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
  }
</style>
