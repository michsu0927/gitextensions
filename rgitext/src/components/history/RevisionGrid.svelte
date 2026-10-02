<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { RefInfo, RevisionRow } from '../../lib/types';
  import { formatRelative, shortHash } from '../../lib/format';

  export let rows: RevisionRow[] = [];
  export let selectedHash: string | null = null;
  export let hasMore = false;
  /** True while the first page is loading. */
  export let loading = false;
  export let loadingMore = false;
  export let onSelect: (hash: string) => void;
  export let onLoadMore: () => void;
  export let onContextMenu: (event: MouseEvent, row: RevisionRow) => void;

  const ROW_H = 26;
  const LANE_W = 14;
  const PAD = 8;
  const MAX_LANES = 14;
  const OVERSCAN = 6;
  const LOAD_MORE_MARGIN = 40;
  const PALETTE = ['#f97316', '#3b82f6', '#10b981', '#a855f7', '#ec4899', '#eab308', '#14b8a6', '#ef4444'];
  const BG = '#0b0f19';

  let viewport: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let scrollTop = 0;
  let viewportH = 400;
  let resizeObserver: ResizeObserver | undefined;
  let frame = 0;

  $: maxLanes = Math.min(MAX_LANES, Math.max(1, rows.reduce((m, r) => Math.max(m, r.graph.width), 1)));
  $: graphW = PAD * 2 + maxLanes * LANE_W;
  $: first = Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN);
  $: last = Math.min(rows.length, Math.ceil((scrollTop + viewportH) / ROW_H) + OVERSCAN);
  $: visible = rows.slice(first, last);
  $: if (hasMore && !loading && !loadingMore && rows.length > 0 && last >= rows.length - LOAD_MORE_MARGIN) {
    onLoadMore();
  }
  $: scheduleDraw(rows, first, last, scrollTop, viewportH, graphW, selectedHash);
  $: if (selectedHash) ensureVisible(selectedHash);

  function color(index: number): string {
    return PALETTE[index % PALETTE.length];
  }

  function laneX(lane: number): number {
    return PAD + Math.min(lane, MAX_LANES - 1) * LANE_W + LANE_W / 2;
  }

  // The graph is drawn on one canvas that stays glued to the top of the scroll viewport and
  // only paints the visible rows.
  function scheduleDraw(..._deps: unknown[]): void {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      drawGraph();
    });
  }

  function drawGraph(): void {
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = graphW;
    const h = viewportH;
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
    }
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    ctx.lineWidth = 2;
    ctx.lineCap = 'round';

    for (let i = first; i < last; i++) {
      const row = rows[i];
      const y0 = i * ROW_H - scrollTop;
      const yc = y0 + ROW_H / 2;
      const y1 = y0 + ROW_H;
      const nodeX = laneX(row.graph.node_lane);

      for (const line of row.graph.lines) {
        ctx.strokeStyle = color(line.color);
        ctx.beginPath();
        if (line.kind === 'pass') {
          ctx.moveTo(laneX(line.from), y0);
          ctx.lineTo(laneX(line.from), y1);
        } else if (line.kind === 'in') {
          ctx.moveTo(laneX(line.from), y0);
          if (line.from === line.to) ctx.lineTo(nodeX, yc);
          else ctx.bezierCurveTo(laneX(line.from), yc, nodeX, y0, nodeX, yc);
        } else {
          ctx.moveTo(nodeX, yc);
          if (line.from === line.to) ctx.lineTo(laneX(line.to), y1);
          else ctx.bezierCurveTo(nodeX, y1, laneX(line.to), yc, laneX(line.to), y1);
        }
        ctx.stroke();
      }

      // Commit node: filled circle, hollow for merges, ringed when selected.
      const isMerge = row.parents.length > 1;
      const isSelected = row.hash === selectedHash;
      ctx.beginPath();
      ctx.arc(nodeX, yc, isMerge ? 4 : 4.5, 0, Math.PI * 2);
      ctx.fillStyle = isMerge ? BG : color(row.graph.color);
      ctx.fill();
      ctx.strokeStyle = isSelected ? '#ffffff' : color(row.graph.color);
      ctx.lineWidth = isSelected ? 2.5 : 2;
      ctx.stroke();
      ctx.lineWidth = 2;
    }
  }

  function onScroll(): void {
    scrollTop = viewport.scrollTop;
  }

  function ensureVisible(hash: string): void {
    if (!viewport) return;
    const index = rows.findIndex((r) => r.hash === hash);
    if (index < 0) return;
    const top = index * ROW_H;
    if (top < viewport.scrollTop) viewport.scrollTop = top;
    else if (top + ROW_H > viewport.scrollTop + viewport.clientHeight) {
      viewport.scrollTop = top + ROW_H - viewport.clientHeight;
    }
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (rows.length === 0) return;
    const current = selectedHash ? rows.findIndex((r) => r.hash === selectedHash) : -1;
    const page = Math.max(1, Math.floor(viewportH / ROW_H) - 1);
    let next = current;
    switch (event.key) {
      case 'ArrowDown': next = current + 1; break;
      case 'ArrowUp': next = current - 1; break;
      case 'PageDown': next = current + page; break;
      case 'PageUp': next = current - page; break;
      case 'Home': next = 0; break;
      case 'End': next = rows.length - 1; break;
      default: return;
    }
    event.preventDefault();
    next = Math.max(0, Math.min(rows.length - 1, next));
    if (next !== current) onSelect(rows[next].hash);
  }

  function refClass(ref: RefInfo): string {
    if (ref.is_head && ref.kind === 'head') return 'rg-ref rg-ref-current';
    return `rg-ref rg-ref-${ref.kind}`;
  }

  onMount(() => {
    viewportH = viewport.clientHeight || viewportH;
    resizeObserver = new ResizeObserver(() => {
      viewportH = viewport.clientHeight || viewportH;
    });
    resizeObserver.observe(viewport);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    if (frame) cancelAnimationFrame(frame);
  });
</script>

<div class="rg-wrap">
  <div class="rg-header" style="padding-left: {graphW}px">
    <span class="rg-col-message">Message</span>
    <span class="rg-col-author">Author</span>
    <span class="rg-col-date">Date</span>
    <span class="rg-col-hash">Commit</span>
  </div>

  <!-- svelte-ignore a11y-no-noninteractive-tabindex -->
  <div class="rg-viewport" bind:this={viewport} on:scroll={onScroll} on:keydown={onKeyDown} tabindex="0" role="listbox">
    <canvas class="rg-canvas" bind:this={canvas} style="height: {viewportH}px; margin-bottom: -{viewportH}px"></canvas>
    <div class="rg-inner" style="height: {rows.length * ROW_H}px">
      {#each visible as row, i (row.hash)}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <div
          class="rg-row"
          class:selected={row.hash === selectedHash}
          role="option"
          tabindex="-1"
          aria-selected={row.hash === selectedHash}
          style="top: {(first + i) * ROW_H}px; height: {ROW_H}px; padding-left: {graphW}px"
          on:click={() => onSelect(row.hash)}
          on:contextmenu|preventDefault={(e) => onContextMenu(e, row)}
        >
          <span class="rg-col-message" title={row.subject}>
            {#each row.refs as ref}
              <span class={refClass(ref)}>{ref.name}</span>
            {/each}
            <span class="rg-subject">{row.subject}</span>
          </span>
          <span class="rg-col-author" title="{row.author} <{row.email}>">{row.author}</span>
          <span class="rg-col-date">{formatRelative(row.author_date)}</span>
          <span class="rg-col-hash monospaced">{shortHash(row.hash)}</span>
        </div>
      {/each}
    </div>
    {#if loading}
      <div class="rg-overlay"><div class="loading-spinner-mini"></div><span>Reading history…</span></div>
    {:else if rows.length === 0}
      <div class="rg-overlay"><span>No commits to show.</span></div>
    {:else if loadingMore}
      <div class="rg-more"><div class="loading-spinner-mini"></div><span>Loading more…</span></div>
    {/if}
  </div>
</div>

<style>
  .rg-wrap {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    background: #0b0f19;
    overflow: hidden;
  }
  .rg-header {
    display: flex;
    align-items: center;
    height: 28px;
    flex-shrink: 0;
    background: #0f172a;
    color: #94a3b8;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    box-sizing: border-box;
    padding-right: 12px;
  }
  .rg-viewport {
    position: relative;
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    outline: none;
  }
  .rg-canvas {
    position: sticky;
    top: 0;
    display: block;
    z-index: 1;
    pointer-events: none;
  }
  .rg-inner {
    position: relative;
  }
  .rg-row {
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    box-sizing: border-box;
    padding-right: 12px;
    font-size: 0.82rem;
    color: #cbd5e1;
    cursor: default;
    white-space: nowrap;
  }
  .rg-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }
  .rg-row.selected {
    background: rgba(234, 88, 12, 0.14);
  }
  .rg-viewport:focus-visible .rg-row.selected {
    background: rgba(234, 88, 12, 0.22);
  }
  .rg-col-message {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rg-col-author {
    width: 150px;
    flex-shrink: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: #94a3b8;
    padding-left: 8px;
  }
  .rg-col-date {
    width: 110px;
    flex-shrink: 0;
    color: #64748b;
    padding-left: 8px;
  }
  .rg-col-hash {
    width: 66px;
    flex-shrink: 0;
    color: #64748b;
    padding-left: 8px;
  }
  .rg-header .rg-col-message,
  .rg-header .rg-col-author,
  .rg-header .rg-col-date,
  .rg-header .rg-col-hash {
    color: inherit;
  }
  .rg-subject {
    vertical-align: middle;
  }
  .rg-ref {
    display: inline-block;
    font-size: 0.68rem;
    font-weight: 600;
    line-height: 1;
    padding: 3px 6px;
    border-radius: 4px;
    margin-right: 5px;
    vertical-align: middle;
    color: #fff;
  }
  .rg-ref-current { background: #ea580c; }
  .rg-ref-head { background: #2563eb; }
  .rg-ref-remote { background: #059669; }
  .rg-ref-tag { background: #ca8a04; }
  .rg-ref-stash { background: #64748b; }
  .rg-ref-other { background: #475569; }
  .rg-overlay,
  .rg-more {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 10px;
    color: #94a3b8;
    font-size: 0.85rem;
  }
  .rg-overlay {
    inset: 0;
    justify-content: center;
    background: rgba(11, 15, 25, 0.6);
    z-index: 2;
  }
  .rg-more {
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 9999px;
    padding: 4px 12px;
    z-index: 2;
  }
</style>
