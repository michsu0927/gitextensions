<script lang="ts">
  import type { CommitDetails, RefInfo } from '../../lib/types';
  import { copyText, formatDate, shortHash } from '../../lib/format';

  export let details: CommitDetails | null = null;
  export let loading = false;
  export let onSelectHash: (hash: string) => void;

  let copied = false;

  $: subject = details ? details.message.split('\n')[0] : '';
  $: body = details ? details.message.split('\n').slice(1).join('\n').replace(/^\n+/, '') : '';
  $: committerDiffers =
    details !== null &&
    (details.committer !== details.author ||
      details.committer_email !== details.email ||
      details.commit_date !== details.author_date);

  function refClass(ref: RefInfo): string {
    return ref.is_head && ref.kind === 'head' ? 'cd-ref cd-ref-current' : `cd-ref cd-ref-${ref.kind}`;
  }

  async function copyHash(): Promise<void> {
    if (!details) return;
    copied = await copyText(details.hash);
    setTimeout(() => (copied = false), 1200);
  }
</script>

<div class="cd">
  {#if loading && !details}
    <div class="cd-state"><div class="loading-spinner-mini"></div><span>Loading commit…</span></div>
  {:else if !details}
    <div class="cd-state">Select a commit.</div>
  {:else}
    <div class="cd-subject">{subject}</div>
    {#if body}
      <pre class="cd-body">{body}</pre>
    {/if}

    {#if details.refs.length > 0}
      <div class="cd-refs">
        {#each details.refs as ref}
          <span class={refClass(ref)}>{ref.name}</span>
        {/each}
      </div>
    {/if}

    <dl class="cd-meta">
      <dt>Commit</dt>
      <dd>
        <span class="monospaced cd-hash">{details.hash}</span>
        <button class="cd-copy" on:click={copyHash} title="Copy full hash">{copied ? 'Copied' : 'Copy'}</button>
      </dd>
      <dt>Author</dt>
      <dd>{details.author} &lt;{details.email}&gt; · {formatDate(details.author_date)}</dd>
      {#if committerDiffers}
        <dt>Committer</dt>
        <dd>{details.committer} &lt;{details.committer_email}&gt; · {formatDate(details.commit_date)}</dd>
      {/if}
      <dt>{details.parents.length > 1 ? 'Parents' : 'Parent'}</dt>
      <dd>
        {#each details.parents as parent}
          <button class="cd-link monospaced" on:click={() => onSelectHash(parent)}>{shortHash(parent)}</button>
        {:else}
          <span class="cd-muted">none (root commit)</span>
        {/each}
      </dd>
    </dl>
  {/if}
</div>

<style>
  .cd {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 0.82rem;
    color: #cbd5e1;
  }
  .cd-state {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #64748b;
  }
  .cd-subject {
    font-size: 0.95rem;
    font-weight: 600;
    color: #f1f5f9;
    word-break: break-word;
  }
  .cd-body {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    color: #94a3b8;
    max-height: 140px;
    overflow: auto;
  }
  .cd-refs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .cd-ref {
    font-size: 0.68rem;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    color: #fff;
  }
  .cd-ref-current { background: #ea580c; }
  .cd-ref-head { background: #2563eb; }
  .cd-ref-remote { background: #059669; }
  .cd-ref-tag { background: #ca8a04; }
  .cd-ref-stash { background: #64748b; }
  .cd-ref-other { background: #475569; }
  .cd-meta {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 0;
  }
  .cd-meta dt {
    color: #64748b;
  }
  .cd-meta dd {
    margin: 0;
    min-width: 0;
    word-break: break-word;
  }
  .cd-hash {
    font-size: 0.75rem;
  }
  .cd-copy,
  .cd-link {
    background: none;
    border: none;
    color: #fb923c;
    cursor: pointer;
    padding: 0 4px;
    font-size: 0.78rem;
  }
  .cd-copy:hover,
  .cd-link:hover {
    text-decoration: underline;
  }
  .cd-muted {
    color: #64748b;
  }
</style>
