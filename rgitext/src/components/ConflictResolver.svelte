<script lang="ts">
  import { api } from '../lib/api';
  import { refreshAll } from '../lib/actions';
  import {
    buildMerged, conflictCount, parseConflicts, unresolvedCount, type Choice, type ConflictSegment, type Segment,
  } from '../lib/conflicts';
  import type { ConflictFile, ConflictKind, ConflictVersions, OperationKind, Resolution } from '../lib/types';
  import { conflictsDialogOpen, opMessage } from '../stores/ops';
  import { currentRepoPath } from '../stores/repo';

  const KIND_LABEL: Record<ConflictKind, string> = {
    'both-modified': 'Both modified',
    'both-added': 'Both added',
    'both-deleted': 'Both deleted',
    'added-by-us': 'Added by us',
    'added-by-them': 'Added by them',
    'deleted-by-us': 'Deleted by us',
    'deleted-by-them': 'Deleted by them',
  };

  let files: ConflictFile[] = [];
  let selected = '';
  let versions: ConflictVersions | null = null;
  let segments: Segment[] = [];
  let manualText = '';
  let manual = false;
  let loadingList = false;
  let loadingFile = false;
  let busy = false;
  let error = '';
  let wasOpen = false;

  $: open = $conflictsDialogOpen;
  $: if (open !== wasOpen) {
    wasOpen = open;
    if (open) void loadList();
  }
  $: current = files.find((f) => f.path === selected);
  $: textual = !!versions && !versions.is_binary && versions.merged !== null;
  $: hasHunks = conflictCount(segments) > 0;
  $: unresolved = unresolvedCount(segments);
  $: sides = sideLabels(versions?.operation ?? null);

  function sideLabels(op: OperationKind | null): { ours: string; theirs: string } {
    return op === 'rebase'
      ? { ours: 'Ours (branch being rebased onto)', theirs: 'Theirs (your commit being replayed)' }
      : { ours: 'Ours (current branch)', theirs: 'Theirs (incoming)' };
  }

  async function loadList(keep?: string): Promise<void> {
    loadingList = true;
    error = '';
    try {
      files = await api.getConflicts($currentRepoPath);
      if (files.length === 0) {
        finish();
        return;
      }
      const next = files.find((f) => f.path === keep)?.path ?? files[0].path;
      await select(next);
    } catch (e) {
      error = String(e);
      files = [];
    } finally {
      loadingList = false;
    }
  }

  async function select(path: string): Promise<void> {
    selected = path;
    versions = null;
    segments = [];
    manual = false;
    manualText = '';
    loadingFile = true;
    error = '';
    try {
      const v = await api.getConflictVersions($currentRepoPath, path);
      if (selected !== path) return;
      versions = v;
      if (!v.is_binary && v.merged !== null) {
        segments = parseConflicts(v.merged);
        manualText = v.merged;
      }
    } catch (e) {
      if (selected === path) error = String(e);
    } finally {
      if (selected === path) loadingFile = false;
    }
  }

  function choose(segment: ConflictSegment, choice: Choice | null): void {
    segment.choice = choice;
    segments = segments;
    manualText = buildMerged(segments);
  }

  function startManual(): void {
    manualText = buildMerged(segments);
    manual = true;
  }

  async function resolve(resolution: Resolution, content?: string): Promise<void> {
    if (!selected || busy) return;
    busy = true;
    error = '';
    const path = selected;
    try {
      await api.resolveConflict($currentRepoPath, path, resolution, content);
      refreshAll();
      await loadList();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function saveMerged(): void {
    const content = manual ? manualText : buildMerged(segments);
    void resolve('merged', content);
  }

  async function mergetool(): Promise<void> {
    if (!selected || busy) return;
    busy = true;
    error = '';
    try {
      await api.runMergetool($currentRepoPath, selected);
      refreshAll();
      await loadList(selected);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function finish(): void {
    $conflictsDialogOpen = false;
    opMessage.set('All conflicts are resolved. Use "Continue" in the banner above to finish the operation.');
    refreshAll();
  }

  function close(): void {
    $conflictsDialogOpen = false;
    refreshAll();
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) close();
  }
</script>

<svelte:window on:keydown={onKeyDown} />

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="cr-overlay" on:click|self={close}>
    <div class="cr" role="dialog" aria-modal="true" aria-label="Resolve conflicts">
      <aside class="cr-list">
        <h3>Conflicts ({files.length})</h3>
        {#if loadingList && files.length === 0}
          <div class="cr-state"><div class="loading-spinner-mini"></div>Loading…</div>
        {/if}
        {#each files as file (file.path)}
          <button class="cr-file" class:selected={file.path === selected} on:click={() => select(file.path)}>
            <span class="cr-path" title={file.path}>{file.path}</span>
            <span class="cr-kind">{KIND_LABEL[file.kind]}</span>
          </button>
        {/each}
        <div class="cr-spacer"></div>
        <button class="cr-close" on:click={close}>Close</button>
      </aside>

      <section class="cr-main">
        {#if !current}
          <div class="cr-state">Select a file.</div>
        {:else}
          <header class="cr-header">
            <strong title={current.path}>{current.path}</strong>
            <span class="cr-kind">{KIND_LABEL[current.kind]}</span>
            <span class="cr-spacer"></span>
            <button disabled={busy} on:click={mergetool} title="Open the merge tool configured in git">External merge tool</button>
          </header>

          {#if error}<div class="cr-error">{error}</div>{/if}

          {#if loadingFile}
            <div class="cr-state"><div class="loading-spinner-mini"></div>Loading…</div>
          {:else if versions}
            <div class="cr-body">
              {#if versions.is_binary}
                <p class="cr-note">This file is not text. Choose one of the two versions.</p>
              {:else if current.kind === 'deleted-by-us'}
                <p class="cr-note">You deleted this file, the other side changed it. Keep their version or delete the file.</p>
              {:else if current.kind === 'deleted-by-them'}
                <p class="cr-note">The other side deleted this file, you changed it. Keep your version or delete the file.</p>
              {:else if current.kind === 'both-deleted'}
                <p class="cr-note">Both sides deleted this file.</p>
              {:else if textual && manual}
                <p class="cr-note">Edit the final content of the file. Remove all conflict markers before saving.</p>
                <textarea class="cr-editor" spellcheck="false" bind:value={manualText}></textarea>
              {:else if textual && hasHunks}
                <p class="cr-note">
                  {unresolved === 0
                    ? 'All conflicts in this file are decided.'
                    : `${unresolved} of ${conflictCount(segments)} conflicts left.`}
                </p>
                {#each segments as segment}
                  {#if segment.kind === 'text'}
                    <pre class="cr-text">{segment.text}</pre>
                  {:else if segment.choice}
                    <div class="cr-hunk resolved">
                      <div class="cr-hunk-bar">
                        <span>Resolved: {segment.choice === 'both' ? 'both sides' : segment.choice === 'ours' ? 'ours' : 'theirs'}</span>
                        <button on:click={() => choose(segment, null)}>Undo</button>
                      </div>
                      <pre class="cr-text">{segment.choice === 'ours' ? segment.ours : segment.choice === 'theirs' ? segment.theirs : segment.ours + segment.theirs}</pre>
                    </div>
                  {:else}
                    <div class="cr-hunk">
                      <div class="cr-hunk-bar">
                        <button on:click={() => choose(segment, 'ours')}>Use ours</button>
                        <button on:click={() => choose(segment, 'theirs')}>Use theirs</button>
                        <button on:click={() => choose(segment, 'both')}>Use both</button>
                      </div>
                      <div class="cr-cols">
                        <div class="cr-col ours">
                          <div class="cr-col-title">{sides.ours}{segment.oursLabel ? ` · ${segment.oursLabel}` : ''}</div>
                          <pre>{segment.ours || '(nothing)'}</pre>
                        </div>
                        {#if segment.base !== null}
                          <div class="cr-col base">
                            <div class="cr-col-title">Base</div>
                            <pre>{segment.base || '(nothing)'}</pre>
                          </div>
                        {/if}
                        <div class="cr-col theirs">
                          <div class="cr-col-title">{sides.theirs}{segment.theirsLabel ? ` · ${segment.theirsLabel}` : ''}</div>
                          <pre>{segment.theirs || '(nothing)'}</pre>
                        </div>
                      </div>
                    </div>
                  {/if}
                {/each}
              {:else if textual}
                <p class="cr-note">There are no conflict markers left in this file.</p>
                <pre class="cr-text">{versions.merged}</pre>
              {/if}
            </div>

            <footer class="cr-footer">
              {#if current.kind === 'deleted-by-us'}
                <button disabled={busy} on:click={() => resolve('theirs')}>Keep their version</button>
                <button class="danger" disabled={busy} on:click={() => resolve('delete')}>Delete the file</button>
              {:else if current.kind === 'deleted-by-them'}
                <button disabled={busy} on:click={() => resolve('ours')}>Keep our version</button>
                <button class="danger" disabled={busy} on:click={() => resolve('delete')}>Delete the file</button>
              {:else if current.kind === 'both-deleted'}
                <button class="danger" disabled={busy} on:click={() => resolve('delete')}>Delete the file</button>
              {:else}
                <button disabled={busy} on:click={() => resolve('ours')}>Take all ours</button>
                <button disabled={busy} on:click={() => resolve('theirs')}>Take all theirs</button>
                {#if textual}
                  <button disabled={busy || manual} on:click={startManual}>Edit result…</button>
                  <span class="cr-spacer"></span>
                  <button
                    class="primary"
                    disabled={busy || (!manual && unresolved > 0)}
                    title={!manual && unresolved > 0 ? 'Decide every conflict first' : 'Write the file and stage it'}
                    on:click={saveMerged}
                  >
                    Save and mark resolved
                  </button>
                {/if}
              {/if}
            </footer>
          {/if}
        {/if}
      </section>
    </div>
  </div>
{/if}

<style>
  .cr-overlay {
    position: fixed;
    inset: 0;
    z-index: 1900;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(2, 6, 23, 0.7);
  }
  .cr {
    width: min(1180px, calc(100vw - 40px));
    height: min(760px, calc(100vh - 40px));
    display: flex;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
    color: #e5e7eb;
    overflow: hidden;
  }
  .cr-list {
    width: 270px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 10px;
    border-right: 1px solid rgba(255, 255, 255, 0.07);
    overflow-y: auto;
  }
  .cr-list h3 {
    margin: 0 4px 8px;
    font-size: 0.9rem;
  }
  .cr-file {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 6px 10px;
    background: none;
    border: 1px solid transparent;
    border-radius: 8px;
    color: #cbd5e1;
    cursor: pointer;
    text-align: left;
  }
  .cr-file:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .cr-file.selected {
    background: rgba(234, 88, 12, 0.16);
    border-color: rgba(234, 88, 12, 0.35);
  }
  .cr-path {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.82rem;
  }
  .cr-kind {
    font-size: 0.7rem;
    color: #facc15;
  }
  .cr-spacer {
    flex: 1;
  }
  .cr-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .cr-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }
  .cr-header strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.9rem;
  }
  .cr-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    flex: 1;
    padding: 20px;
    color: #94a3b8;
  }
  .cr-error {
    margin: 8px 16px 0;
    padding: 8px 12px;
    border-radius: 8px;
    background: rgba(239, 68, 68, 0.12);
    color: #fca5a5;
    font-size: 0.8rem;
    white-space: pre-wrap;
  }
  .cr-body {
    flex: 1;
    overflow: auto;
    padding: 12px 16px;
  }
  .cr-note {
    margin: 0 0 10px;
    color: #94a3b8;
    font-size: 0.82rem;
  }
  .cr-text,
  .cr-col pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--font-mono, monospace);
    font-size: 0.78rem;
    line-height: 1.5;
    color: #cbd5e1;
  }
  .cr-hunk {
    margin: 10px 0;
    border: 1px solid rgba(250, 204, 21, 0.35);
    border-radius: 10px;
    overflow: hidden;
  }
  .cr-hunk.resolved {
    border-color: rgba(16, 185, 129, 0.4);
  }
  .cr-hunk.resolved .cr-text {
    padding: 8px 12px;
    background: rgba(16, 185, 129, 0.08);
  }
  .cr-hunk-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.04);
    font-size: 0.78rem;
    color: #94a3b8;
  }
  .cr-cols {
    display: flex;
  }
  .cr-col {
    flex: 1;
    min-width: 0;
    padding: 8px 12px;
  }
  .cr-col + .cr-col {
    border-left: 1px solid rgba(255, 255, 255, 0.07);
  }
  .cr-col.ours {
    background: rgba(59, 130, 246, 0.08);
  }
  .cr-col.theirs {
    background: rgba(168, 85, 247, 0.08);
  }
  .cr-col-title {
    margin-bottom: 4px;
    font-size: 0.72rem;
    color: #94a3b8;
  }
  .cr-editor {
    width: 100%;
    height: calc(100% - 32px);
    min-height: 300px;
    box-sizing: border-box;
    background: #0b0f19;
    color: #e5e7eb;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 10px;
    font-family: var(--font-mono, monospace);
    font-size: 0.78rem;
    line-height: 1.5;
    resize: none;
    outline: none;
  }
  .cr-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid rgba(255, 255, 255, 0.07);
  }
  .cr button {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    color: #cbd5e1;
    padding: 5px 14px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .cr button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }
  .cr button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .cr button.primary {
    background: #ea580c;
    border-color: #ea580c;
    color: #fff;
    font-weight: 600;
  }
  .cr button.danger {
    color: #fca5a5;
    border-color: rgba(239, 68, 68, 0.45);
  }
  .cr-close {
    align-self: stretch;
  }
</style>
