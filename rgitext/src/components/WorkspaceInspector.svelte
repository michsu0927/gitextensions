<script>
  export let isBranchesCollapsed;
  export let branches = [];
  export let isLoadingBranches = false;
  export let branchesError = '';
  export let tags = [];
  export let isLoadingTags = false;
  export let tagsError = '';
  export let stashes = [];
  export let isLoadingStashes = false;
  export let stashesError = '';
  export let remotes = [];
  export let isLoadingRemotes = false;
  export let remotesError = '';
  /** @type {import("../lib/types").GitStatus | null} */
  export let gitStatus = null;
  export let isExecutingBranchAction = false;
  export let sidebarBranchMessage = '';
  export let sidebarBranchError = '';

  // sidebar branch search state
  export let branchSearchQuery = '';
  export let selectedSidebarBranch = '';
  export let selectedSidebarTag = '';
  /** @type {unknown} */
  export let selectedSidebarStash = null;

  // collapsible sections
  export let isSidebarBranchesExpanded = true;
  export let isSidebarTagsExpanded = false;
  export let isSidebarStashesExpanded = false;
  export let isSidebarRemotesExpanded = false;

  // callbacks
  export let refreshAll;
  export let performCheckout;
  export let performMerge;
  export let checkoutTag;
  export let performStashApply;
  export let remoteBranches = [];
  export let isLoadingRemoteBranches = false;
  export let remoteBranchesError = '';
  export let performCheckoutRemoteBranch;

  let selectedSidebarRemoteBranch = '';
  let expandedRemotes = {};

  function toggleRemoteExpand(remoteName) {
    expandedRemotes[remoteName] = !expandedRemotes[remoteName];
    expandedRemotes = { ...expandedRemotes };
  }

  function handleRemoteBranchCheckout(remoteBranch) {
    const slashIdx = remoteBranch.indexOf('/');
    let proposedLocalName = slashIdx !== -1 ? remoteBranch.substring(slashIdx + 1) : remoteBranch;
    
    const localName = prompt(`Checkout Remote Branch "${remoteBranch}"\nEnter local branch name to create and track:`, proposedLocalName);
    if (localName && localName.trim()) {
      performCheckoutRemoteBranch(remoteBranch, localName.trim());
    }
  }

  function toggleSidebarBranch(branch) {
    if (selectedSidebarBranch === branch) {
      selectedSidebarBranch = '';
    } else {
      selectedSidebarBranch = branch;
    }
  }
</script>

<aside class="left-branches-sidebar {isBranchesCollapsed ? 'collapsed' : ''}">
  <div class="branches-sidebar-header">
    <div class="branches-sidebar-title">
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="9"></rect><rect x="14" y="3" width="7" height="5"></rect><rect x="14" y="12" width="7" height="9"></rect><rect x="3" y="16" width="7" height="5"></rect></svg>
      <span>Workspace Inspector</span>
    </div>
    <div class="branches-sidebar-header-actions">
      <button class="btn-sidebar-refresh" on:click={refreshAll} title="Refresh all lists" disabled={isLoadingBranches || isLoadingTags || isLoadingStashes || isLoadingRemotes}>
        <svg class={isLoadingBranches || isLoadingTags || isLoadingStashes || isLoadingRemotes ? 'spin' : ''} xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>
      </button>
      <button class="btn-sidebar-collapse" on:click={() => isBranchesCollapsed = true} title="Collapse Panel">
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
      </button>
    </div>
  </div>

  <!-- Scrollable Inspector Content -->
  <div class="inspector-scroll-area">
    
    <!-- SECTION 1: BRANCHES -->
    <div class="inspector-accordion {isSidebarBranchesExpanded ? 'expanded' : ''}">
      <button class="accordion-trigger" on:click={() => isSidebarBranchesExpanded = !isSidebarBranchesExpanded}>
        <svg class="chevron-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
        <svg class="section-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#ea580c" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>
        <span>Branches</span>
        <span class="badge-mini">{branches.length}</span>
      </button>
      
      {#if isSidebarBranchesExpanded}
        <div class="accordion-content">
          <!-- Search Input -->
          <div class="branches-sidebar-search">
            <div class="search-input-wrapper">
              <svg class="search-icon" xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
              <input 
                type="text" 
                placeholder="Filter branches..." 
                bind:value={branchSearchQuery}
              />
              {#if branchSearchQuery}
                <button class="search-clear-btn" on:click={() => branchSearchQuery = ''}>&times;</button>
              {/if}
            </div>
          </div>

          <!-- List -->
          <div class="branches-sidebar-list">
            {#if isLoadingBranches && branches.length === 0}
              <div class="sidebar-loading-mini">Loading...</div>
            {:else if branchesError}
              <div class="sidebar-error-mini">{branchesError}</div>
            {:else}
              {@const filteredBranches = branches.filter(b => b.toLowerCase().includes(branchSearchQuery.toLowerCase()))}
              {#if filteredBranches.length === 0}
                <div class="sidebar-empty-mini">No branches</div>
              {:else}
                {#each filteredBranches as branch}
                  {@const isActive = gitStatus?.branch === branch}
                  {@const isSelected = selectedSidebarBranch === branch}
                  <div class="sidebar-branch-item {isActive ? 'active' : ''} {isSelected ? 'selected' : ''}" role="button" tabindex="0" on:click={() => toggleSidebarBranch(branch)} on:keydown={(e) => e.key === 'Enter' && toggleSidebarBranch(branch)}>
                    <div class="branch-item-main">
                      <svg class="branch-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>
                      <span class="branch-name" title={branch}>{branch}</span>
                      {#if isActive}
                        <span class="active-dot" title="Currently Active Branch"></span>
                      {/if}
                    </div>

                    {#if isSelected}
                      <div class="branch-item-actions" role="toolbar" tabindex="0" on:click|stopPropagation on:keydown|stopPropagation>
                        <button class="btn-sidebar-act btn-checkout" disabled={isActive || isExecutingBranchAction} on:click={() => performCheckout(branch)}>Checkout</button>
                        <button class="btn-sidebar-act btn-merge" disabled={isActive || isExecutingBranchAction} on:click={() => performMerge(branch)}>Merge</button>
                      </div>
                    {/if}
                  </div>
                {/each}
              {/if}
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- SECTION 2: TAGS -->
    <div class="inspector-accordion {isSidebarTagsExpanded ? 'expanded' : ''}">
      <button class="accordion-trigger" on:click={() => isSidebarTagsExpanded = !isSidebarTagsExpanded}>
        <svg class="chevron-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
        <svg class="section-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#10b981" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"></path><line x1="7" y1="7" x2="7.01" y2="7"></line></svg>
        <span>Tags</span>
        <span class="badge-mini">{tags.length}</span>
      </button>

      {#if isSidebarTagsExpanded}
        <div class="accordion-content">
          <div class="branches-sidebar-list">
            {#if isLoadingTags && tags.length === 0}
              <div class="sidebar-loading-mini">Loading...</div>
            {:else if tagsError}
              <div class="sidebar-error-mini">{tagsError}</div>
            {:else if tags.length === 0}
              <div class="sidebar-empty-mini">No tags in repo</div>
            {:else}
              {#each tags as tag}
                {@const isSelected = selectedSidebarTag === tag}
                <div class="sidebar-branch-item {isSelected ? 'selected' : ''}" role="button" tabindex="0" on:click={() => selectedSidebarTag = isSelected ? '' : tag} on:keydown={(e) => e.key === 'Enter' && (selectedSidebarTag = isSelected ? '' : tag)}>
                  <div class="branch-item-main">
                    <svg class="branch-icon tag-icon-color" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"></path><line x1="7" y1="7" x2="7.01" y2="7"></line></svg>
                    <span class="branch-name" title={tag}>{tag}</span>
                  </div>

                  {#if isSelected}
                    <div class="branch-item-actions" role="toolbar" tabindex="0" on:click|stopPropagation on:keydown|stopPropagation>
                      <button class="btn-sidebar-act btn-checkout tag-checkout-btn" disabled={isExecutingBranchAction} on:click={() => checkoutTag(tag)}>Checkout Tag</button>
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- SECTION 3: STASHES -->
    <div class="inspector-accordion {isSidebarStashesExpanded ? 'expanded' : ''}">
      <button class="accordion-trigger" on:click={() => isSidebarStashesExpanded = !isSidebarStashesExpanded}>
        <svg class="chevron-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
        <svg class="section-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#a855f7" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line></svg>
        <span>Stashes</span>
        <span class="badge-mini">{stashes.length}</span>
      </button>

      {#if isSidebarStashesExpanded}
        <div class="accordion-content">
          <div class="branches-sidebar-list">
            {#if isLoadingStashes && stashes.length === 0}
              <div class="sidebar-loading-mini">Loading...</div>
            {:else if stashesError}
              <div class="sidebar-error-mini">{stashesError}</div>
            {:else if stashes.length === 0}
              <div class="sidebar-empty-mini">No stashes</div>
            {:else}
              {#each stashes as stash, idx}
                {@const isSelected = selectedSidebarStash === idx}
                <div class="sidebar-branch-item stash-item {isSelected ? 'selected' : ''}" role="button" tabindex="0" on:click={() => selectedSidebarStash = isSelected ? null : idx} on:keydown={(e) => e.key === 'Enter' && (selectedSidebarStash = isSelected ? null : idx)}>
                  <div class="branch-item-main">
                    <svg class="branch-icon stash-icon-color" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path></svg>
                    <span class="branch-name" title={stash}>{stash}</span>
                  </div>

                  {#if isSelected}
                    <div class="branch-item-actions" role="toolbar" tabindex="0" on:click|stopPropagation on:keydown|stopPropagation>
                      <button class="btn-sidebar-act btn-checkout stash-apply-btn" disabled={isExecutingBranchAction} on:click={() => performStashApply(idx)}>Apply Stash</button>
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- SECTION 4: REMOTES -->
    <div class="inspector-accordion {isSidebarRemotesExpanded ? 'expanded' : ''}">
      <button class="accordion-trigger" on:click={() => isSidebarRemotesExpanded = !isSidebarRemotesExpanded}>
        <svg class="chevron-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
        <svg class="section-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"></path><path d="M12 5l7 7-7 7"></path></svg>
        <span>Remotes</span>
        <span class="badge-mini">{remotes.length}</span>
      </button>

      {#if isSidebarRemotesExpanded}
        <div class="accordion-content">
          <div class="branches-sidebar-list">
            {#if isLoadingRemotes && remotes.length === 0}
              <div class="sidebar-loading-mini">Loading...</div>
            {:else if remotesError}
              <div class="sidebar-error-mini">{remotesError}</div>
            {:else if remotes.length === 0}
              <div class="sidebar-empty-mini">No remotes configured</div>
            {:else}
              {#each remotes as remote}
                {@const isRemoteExpanded = !!expandedRemotes[remote]}
                <div class="remote-group-container">
                  <!-- Remote Header -->
                  <div class="sidebar-branch-item {isRemoteExpanded ? 'selected' : ''}" role="button" tabindex="0" on:click={() => toggleRemoteExpand(remote)} on:keydown={(e) => e.key === 'Enter' && toggleRemoteExpand(remote)}>
                    <div class="branch-item-main">
                      <svg class="chevron-icon {isRemoteExpanded ? 'rotate-90' : ''}" xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
                      <svg class="branch-icon remote-icon-color" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"></path><path d="M12 5l7 7-7 7"></path></svg>
                      <span class="branch-name remote-name-text" title={remote}>{remote}</span>
                    </div>
                  </div>

                  <!-- Remote Branches List -->
                  {#if isRemoteExpanded}
                    <div class="remote-branches-sublist" style="padding-left: 12px; margin-top: 2px; margin-bottom: 6px;">
                      {#if isLoadingRemoteBranches && remoteBranches.length === 0}
                        <div class="sidebar-loading-mini">Loading branches...</div>
                      {:else if remoteBranchesError}
                        <div class="sidebar-error-mini">{remoteBranchesError}</div>
                      {:else}
                        {@const specificRemoteBranches = remoteBranches.filter(b => b.startsWith(remote + '/'))}
                        {#if specificRemoteBranches.length === 0}
                          <div class="sidebar-empty-mini">No branches in remote</div>
                        {:else}
                          {#each specificRemoteBranches as remoteBranch}
                            {@const isRBSelected = selectedSidebarRemoteBranch === remoteBranch}
                            <!-- svelte-ignore a11y-interactive-supports-focus -->
                            <div class="sidebar-branch-item {isRBSelected ? 'selected' : ''}" role="button" on:click|stopPropagation={() => selectedSidebarRemoteBranch = isRBSelected ? '' : remoteBranch} on:keydown|stopPropagation={(e) => e.key === 'Enter' && (selectedSidebarRemoteBranch = isRBSelected ? '' : remoteBranch)}>
                              <div class="branch-item-main">
                                <svg class="branch-icon" xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>
                                <span class="branch-name" title={remoteBranch}>{remoteBranch.substring(remote.length + 1)}</span>
                              </div>

                              {#if isRBSelected}
                                <div class="branch-item-actions" role="toolbar" tabindex="0" on:click|stopPropagation on:keydown|stopPropagation>
                                  <button class="btn-sidebar-act btn-checkout" disabled={isExecutingBranchAction} on:click={() => handleRemoteBranchCheckout(remoteBranch)}>Checkout</button>
                                </div>
                              {/if}
                            </div>
                          {/each}
                        {/if}
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>

  </div>

  <!-- Compact status block for branches actions -->
  {#if sidebarBranchMessage || sidebarBranchError}
    <div class="sidebar-branch-feedback">
      {#if sidebarBranchMessage}
        <div class="sidebar-feedback-success">
          <span>{sidebarBranchMessage}</span>
          <button class="feedback-close" on:click={() => sidebarBranchMessage = ''}>&times;</button>
        </div>
      {/if}
      {#if sidebarBranchError}
        <div class="sidebar-feedback-error">
          <span>{sidebarBranchError}</span>
          <button class="feedback-close" on:click={() => sidebarBranchError = ''}>&times;</button>
        </div>
      {/if}
    </div>
  {/if}
</aside>

<style>
  /* Sidebar Inspector specific styles */
  .left-branches-sidebar {
    width: 250px;
    background-color: #0c101d;
    border-right: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s;
    height: 100%;
    z-index: 10;
    flex-shrink: 0;
  }

  .left-branches-sidebar.collapsed {
    width: 0;
    border-right: none;
  }

  .branches-sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    min-width: 250px;
  }

  .branches-sidebar-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    color: #e2e8f0;
    font-size: 0.95rem;
  }

  .branches-sidebar-title svg {
    color: #ea580c;
  }

  .branches-sidebar-header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-sidebar-refresh, .btn-sidebar-collapse {
    background: transparent;
    border: none;
    color: #64748b;
    border-radius: 4px;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-sidebar-refresh:hover, .btn-sidebar-collapse:hover {
    color: white;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .branches-sidebar-search {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    min-width: 250px;
  }

  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    color: #475569;
    pointer-events: none;
    display: flex;
    align-items: center;
  }

  .branches-sidebar-search input {
    width: 100%;
    background-color: #070a13;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 6px 30px 6px 28px;
    font-size: 0.8rem;
    color: white;
    transition: border-color 0.2s;
  }

  .branches-sidebar-search input:focus {
    outline: none;
    border-color: #ea580c;
  }

  .search-clear-btn {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: #64748b;
    font-size: 1.1rem;
    cursor: pointer;
    line-height: 1;
    padding: 0;
  }

  .search-clear-btn:hover {
    color: white;
  }

  .branches-sidebar-list {
    max-height: 280px;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 250px;
  }

  .branches-sidebar-list::-webkit-scrollbar {
    width: 6px;
  }

  .branches-sidebar-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .branches-sidebar-list::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 3px;
  }

  .branches-sidebar-list::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.3);
  }

  .sidebar-branch-item {
    padding: 10px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    display: flex;
    flex-direction: column;
    gap: 8px;
    border: 1px solid transparent;
  }

  .sidebar-branch-item:hover {
    background-color: rgba(255, 255, 255, 0.02);
  }

  .sidebar-branch-item.selected {
    background-color: rgba(255, 255, 255, 0.03);
    border-color: rgba(255, 255, 255, 0.05);
  }

  .sidebar-branch-item.active {
    background-color: rgba(59, 130, 246, 0.03);
    border-color: rgba(59, 130, 246, 0.1);
  }

  .branch-item-main {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    overflow: hidden;
  }

  .sidebar-branch-item .branch-icon {
    color: #475569;
    flex-shrink: 0;
    transition: color 0.2s;
  }

  .sidebar-branch-item:hover .branch-icon {
    color: #94a3b8;
  }

  .sidebar-branch-item.active .branch-icon {
    color: #3b82f6;
  }

  .sidebar-branch-item .branch-name {
    font-size: 0.8rem;
    font-weight: 500;
    color: #94a3b8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    text-align: left;
  }

  .sidebar-branch-item:hover .branch-name {
    color: white;
  }

  .sidebar-branch-item.active .branch-name {
    color: #60a5fa;
    font-weight: 600;
  }

  .sidebar-branch-item .active-dot {
    width: 6px;
    height: 6px;
    background-color: #3b82f6;
    border-radius: 50%;
    box-shadow: 0 0 6px #3b82f6;
    flex-shrink: 0;
  }

  .branch-item-actions {
    display: flex;
    gap: 6px;
    padding-left: 22px;
    animation: fadeIn 0.2s ease-out;
  }

  .btn-sidebar-act {
    flex: 1;
    border: none;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 4px 8px;
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
  }

  .btn-sidebar-act.btn-checkout {
    background-color: #3b82f6;
    color: white;
  }

  .btn-sidebar-act.btn-checkout:hover:not(:disabled) {
    background-color: #60a5fa;
  }

  .btn-sidebar-act.btn-merge {
    background-color: transparent;
    color: #ea580c;
    border: 1px solid rgba(234, 88, 12, 0.3);
  }

  .btn-sidebar-act.btn-merge:hover:not(:disabled) {
    background-color: rgba(234, 88, 12, 0.08);
    border-color: #ea580c;
  }

  .btn-sidebar-act:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .sidebar-branch-feedback {
    padding: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
    display: flex;
    flex-direction: column;
    gap: 8px;
    background-color: #070a13;
    min-width: 250px;
  }

  .sidebar-feedback-success, .sidebar-feedback-error {
    position: relative;
    padding: 8px 24px 8px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    line-height: 1.3;
    text-align: left;
  }

  .sidebar-feedback-success {
    background-color: rgba(16, 185, 129, 0.08);
    border: 1px solid rgba(16, 185, 129, 0.15);
    color: #34d399;
  }

  .sidebar-feedback-error {
    background-color: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.15);
    color: #f87171;
  }

  .feedback-close {
    position: absolute;
    right: 4px;
    top: 4px;
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 0.9rem;
    line-height: 1;
    padding: 0;
    opacity: 0.7;
  }

  .feedback-close:hover {
    opacity: 1;
  }
</style>
