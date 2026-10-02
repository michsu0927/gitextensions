<script>
  export let branches = [];
  export let isLoadingBranches = false;
  export let branchesError = '';
  export let selectedBranch = '';
  export let branchMessage = '';
  export let branchActionError = '';
  export let isExecutingBranchAction = false;
  /** @type {import("../lib/types").GitStatus | null} */
  export let gitStatus = null;

  // callbacks
  export let performCheckout;
  export let performMerge;
  export let performRebase;
  export let performCreateBranch;
  export let performRenameBranch;
  export let performDeleteBranch;
  export let performPull;
  export let performPush;

  // state
  let isBranchesCardCollapsed = false;
  let activeDropdownBranch = '';

  function toggleDropdown(branchName) {
    if (activeDropdownBranch === branchName) {
      activeDropdownBranch = '';
    } else {
      activeDropdownBranch = branchName;
    }
  }

  // Click outside to close dropdown
  import { onMount, onDestroy } from 'svelte';
  function handleDocumentClick(e) {
    if (!e.target.closest('.dropdown-container')) {
      activeDropdownBranch = '';
    }
  }
  onMount(() => {
    document.addEventListener('click', handleDocumentClick);
  });
  onDestroy(() => {
    document.removeEventListener('click', handleDocumentClick);
  });

  function handleCreateBranch() {
    const newName = prompt("Enter name for the new branch:");
    if (newName && newName.trim()) {
      performCreateBranch(newName.trim());
    }
  }

  function handleRenameBranch(branch) {
    const newName = prompt(`Enter new name for branch "${branch}":`, branch);
    if (newName && newName.trim() && newName.trim() !== branch) {
      performRenameBranch(branch, newName.trim());
    }
  }

  function handleDeleteBranch(branch) {
    if (confirm(`Are you sure you want to delete branch "${branch}"?`)) {
      performDeleteBranch(branch, false);
    }
  }

  function statusLabel(status) {
    const map = {
      'M': 'M',
      'A': 'A',
      'D': 'D',
      'R': 'R',
      'C': 'C',
      'U': 'U',
      '??': 'N',
    };
    return map[status] || status;
  }

  function statusClass(status) {
    if (status === '??') return 'status-badge-untracked';
    return `status-badge-${status.toLowerCase()}`;
  }
</script>

<div class="branches-tab-layout animate-fade-in">
  
  <!-- Repository Branches Card (Collapsible) -->
  <div class="card glass branch-list-card">
    <div class="card-header flex justify-between items-center cursor-pointer" role="button" tabindex="0" on:click={() => isBranchesCardCollapsed = !isBranchesCardCollapsed} on:keydown={(e) => e.key === 'Enter' && (isBranchesCardCollapsed = !isBranchesCardCollapsed)}>
      <div class="flex items-center gap-12 text-orange">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ea580c" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>
        <h3>Repository Branches</h3>
        <span class="badge">{branches.length} branches</span>
        <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
        <div class="header-branch-actions flex items-center gap-8" on:click|stopPropagation>
          <button 
            class="btn-header-action btn-pull" 
            on:click={performPull}
            disabled={isExecutingBranchAction}
            title="Pull changes"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><polyline points="19 12 12 19 5 12"></polyline></svg>
            <span>Pull</span>
          </button>
          <button 
            class="btn-header-action btn-push" 
            on:click={performPush}
            disabled={isExecutingBranchAction}
            title="Push changes"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"></line><polyline points="5 12 12 5 19 12"></polyline></svg>
            <span>Push</span>
          </button>
        </div>
      </div>
      <button class="btn-collapse-toggle">
        <svg class="chevron-icon {isBranchesCardCollapsed ? '' : 'rotate-90'}" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
      </button>
    </div>

    {#if !isBranchesCardCollapsed}
      <div class="card-body branch-card-body">
        {#if isLoadingBranches}
          <div class="loading-state-mini">
            <div class="loading-spinner-mini"></div>
            <span>Loading branches...</span>
          </div>
        {:else if branchesError}
          <div class="alert alert-error">{branchesError}</div>
        {:else}
          <!-- Branch Action Status Banners -->
          {#if branchMessage}
            <div class="settings-alert settings-alert-success margin-bottom-12">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
              <pre class="alert-pre">{branchMessage}</pre>
            </div>
          {/if}

          {#if branchActionError}
            <div class="settings-alert settings-alert-error margin-bottom-12">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
              <pre class="alert-pre">{branchActionError}</pre>
            </div>
          {/if}

          <div class="table-wrapper {activeDropdownBranch ? 'expanded-branches-height' : 'max-height-240'}">
            <table class="history-table branches-table">
              <thead>
                <tr>
                  <th class="col-active-branch-indicator"></th>
                  <th class="col-branch-name">Branch Name</th>
                  <th class="col-branch-actions">Actions</th>
                </tr>
              </thead>
              <tbody>
                {#each branches as branch}
                  {@const isActive = gitStatus?.branch === branch}
                  <tr class="branch-row {isActive ? 'active-branch-row' : ''} {selectedBranch === branch ? 'selected' : ''}" on:click={() => selectedBranch = branch}>
                    <td class="col-active-branch-indicator">
                      {#if isActive}
                        <span class="active-tag-circle" title="Currently Active Branch"></span>
                      {/if}
                    </td>
                    <td class="col-branch-name monospaced">
                      {branch}
                      {#if isActive}
                        <span class="active-badge-tag">current</span>
                      {/if}
                    </td>
                    <td class="col-branch-actions">
                      <div class="branch-action-buttons">
                        <button 
                          class="btn-branch-act btn-checkout" 
                          disabled={isActive || isExecutingBranchAction} 
                          on:click|stopPropagation={() => performCheckout(branch)}
                        >
                          Checkout
                        </button>
                        <button 
                          class="btn-branch-act btn-merge" 
                          disabled={isActive || isExecutingBranchAction} 
                          on:click|stopPropagation={() => performMerge(branch)}
                        >
                          Merge to Current
                        </button>
                        <div class="dropdown-container">
                          <button 
                            class="btn-branch-act btn-more" 
                            disabled={isExecutingBranchAction} 
                            on:click|stopPropagation={() => toggleDropdown(branch)}
                            title="More actions"
                          >
                            &bull;&bull;&bull;
                          </button>
                          {#if activeDropdownBranch === branch}
                          <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
                            <div class="dropdown-menu shadow-premium" on:click|stopPropagation>
                              <button class="dropdown-item" on:click={() => { activeDropdownBranch = ''; performCheckout(branch); }}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect><path d="M9 14l2 2 4-4"></path></svg>
                                <span>Checkout branch...</span>
                              </button>
                              <button class="dropdown-item" on:click={() => { activeDropdownBranch = ''; performMerge(branch); }}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22V12"></path><path d="M18 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"></path><path d="M6 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"></path><path d="M12 12a6 6 0 0 1 6-6"></path><path d="M12 12a6 6 0 0 0-6-6"></path></svg>
                                <span>Merge into current branch...</span>
                              </button>
                              <button class="dropdown-item" on:click={() => { activeDropdownBranch = ''; performRebase(branch); }}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20"></path><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path></svg>
                                <span>Rebase current branch on this branch...</span>
                              </button>
                              <button class="dropdown-item" on:click={() => { activeDropdownBranch = ''; handleCreateBranch(); }}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                                <span>Create branch...</span>
                              </button>
                              <div class="dropdown-divider"></div>
                              <button class="dropdown-item" on:click={() => { activeDropdownBranch = ''; handleRenameBranch(branch); }}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 1 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>
                                <span>Rename branch...</span>
                              </button>
                              <button class="dropdown-item dropdown-item-danger" on:click={() => { activeDropdownBranch = ''; handleDeleteBranch(branch); }}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>
                                <span>Delete branch...</span>
                              </button>
                            </div>
                          {/if}
                        </div>
                      </div>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    {/if}
  </div>

</div>

<style>
  /* Branches-specific Layout alignments */
  .margin-bottom-12 {
    margin-bottom: 12px !important;
  }
</style>
