<script>
  export let isBranchesCollapsed;
  export let currentRepoPath;
  export let tempRepoPath;
  export let isEditingPath;
  export let pathError;
  export let isChecking;
  export let isLoadingCommits;
  export let isLoadingStatus;
  export let isLoadingBranches;

  // callbacks
  export let refreshAll;
  export let browseRepoPath;
  export let browseRepoPathFromInput;
  export let saveRepoPath;
  export let handlePathKeydown;
</script>

<header class="top-bar">
  <div class="repo-info">
    {#if isBranchesCollapsed}
      <button class="btn-expand-branches-topbar" on:click={() => isBranchesCollapsed = false} title="Expand Branches Panel">
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>
        <span>Show Branches</span>
      </button>
    {/if}
    <span class="repo-badge">Active Repo</span>
    {#if isEditingPath}
      <div class="path-editor-group">
        <input 
          type="text" 
          class="top-path-input" 
          bind:value={tempRepoPath} 
          on:keydown={handlePathKeydown}
          placeholder="Enter absolute repository path..."
        />
        <button class="btn-browse-icon" title="Browse folder..." on:click={browseRepoPathFromInput}>
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
        </button>
        <button class="btn-action btn-save" on:click={saveRepoPath}>Open</button>
        <button class="btn-action btn-cancel" on:click={() => { isEditingPath = false; pathError = ''; }}>Cancel</button>
      </div>
    {:else}
      <span class="repo-path" title="Click to edit path" role="button" tabindex="0" on:click={() => { isEditingPath = true; tempRepoPath = currentRepoPath; }} on:keydown={(e) => e.key === 'Enter' && (isEditingPath = true, tempRepoPath = currentRepoPath)}>
        {currentRepoPath}
      </span>
      <div class="repo-action-buttons">
        <button class="btn-change-path" on:click={() => { isEditingPath = true; tempRepoPath = currentRepoPath; }}>
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
          Type Path
        </button>
        <button class="btn-change-path btn-browse" on:click={browseRepoPath}>
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
          Browse...
        </button>
      </div>
    {/if}
  </div>
  <div class="top-actions">
    <button class="btn btn-secondary btn-icon" on:click={refreshAll} disabled={isChecking || isLoadingCommits || isLoadingStatus || isLoadingBranches}>
      <svg class={isChecking || isLoadingCommits || isLoadingStatus || isLoadingBranches ? 'spin' : ''} xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>
    </button>
  </div>
</header>
