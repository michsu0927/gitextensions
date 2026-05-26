<script>
  export let tempGitPath;
  export let isSavingSettings;
  export let settingsMessage;
  export let settingsError;
  export let customGitPath;
  export let remoteBranches = [];
  export let remotes = [];

  // callbacks
  export let browseGitExecutable;
  export let saveSettings;
  export let configureAndFetchRemote;
  export let performCheckoutRemoteBranch;

  // local remote setting state
  let remoteName = 'origin';
  let remoteUrl = '';
  let isConfiguringRemote = false;
  let remoteFeedback = '';
  let remoteFeedbackError = '';

  let selectedRemoteForCheckout = 'origin';
  let selectedRemoteBranch = '';
  let selectedLocalBranch = '';
  let isCheckingOut = false;

  // Reactively select first remote if none selected or if remotes list changes
  $: if (remotes.length > 0 && !remotes.includes(selectedRemoteForCheckout)) {
    selectedRemoteForCheckout = remotes[0];
  }

  async function handleConfigureRemote() {
    if (!remoteName.trim() || !remoteUrl.trim()) return;
    isConfiguringRemote = true;
    remoteFeedback = '';
    remoteFeedbackError = '';
    try {
      const msg = await configureAndFetchRemote(remoteName.trim(), remoteUrl.trim());
      remoteFeedback = msg;
      selectedRemoteForCheckout = remoteName.trim();
      remoteUrl = ''; // Clear input on success
    } catch (e) {
      remoteFeedbackError = e.toString();
    } finally {
      isConfiguringRemote = false;
    }
  }

  function handleRemoteBranchSelectionChange() {
    if (selectedRemoteBranch) {
      const slashIdx = selectedRemoteBranch.indexOf('/');
      if (slashIdx !== -1) {
        selectedLocalBranch = selectedRemoteBranch.substring(slashIdx + 1);
      } else {
        selectedLocalBranch = selectedRemoteBranch;
      }
    } else {
      selectedLocalBranch = '';
    }
  }

  async function handleCheckoutRemoteBranch() {
    if (!selectedRemoteBranch || !selectedLocalBranch.trim()) return;
    isCheckingOut = true;
    remoteFeedback = '';
    remoteFeedbackError = '';
    try {
      await performCheckoutRemoteBranch(selectedRemoteBranch, selectedLocalBranch.trim());
      remoteFeedback = `Successfully checked out "${selectedRemoteBranch}" as local branch "${selectedLocalBranch.trim()}"!`;
      
      // Reset inputs
      selectedRemoteBranch = '';
      selectedLocalBranch = '';
    } catch (e) {
      remoteFeedbackError = `Checkout failed: ${e.toString()}`;
    } finally {
      isCheckingOut = false;
    }
  }
</script>

<div class="settings-container">
  <div class="card glass settings-card">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#ea580c" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>
      <h2>Settings</h2>
    </div>
    
    <div class="card-body settings-body">
      <!-- Section 1: Git Executable Configuration -->
      <div class="settings-section">
        <h3>Git Configuration</h3>
        <p class="settings-desc">Specify a custom path to the Git executable. If left blank, the app will try to run "git" from the system PATH or default Windows installation folders.</p>
        
        <div class="settings-input-group">
          <div class="settings-input-wrapper">
            <input 
              type="text" 
              placeholder="e.g. C:\Program Files\Git\bin\git.exe" 
              bind:value={tempGitPath} 
              class="settings-input"
            />
            <button class="btn btn-secondary settings-browse-btn" on:click={browseGitExecutable}>
              Browse File...
            </button>
          </div>
          <button class="btn btn-primary settings-save-btn" on:click={saveSettings} disabled={isSavingSettings}>
            {isSavingSettings ? 'Verifying...' : 'Save Configuration'}
          </button>
        </div>

        {#if settingsMessage}
          <div class="settings-alert settings-alert-success">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
            <span>{settingsMessage}</span>
          </div>
        {/if}

        {#if settingsError}
          <div class="settings-alert settings-alert-error">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
            <span>{settingsError}</span>
          </div>
        {/if}
      </div>

      <!-- Section 2: Git Remote URL Configuration (New) -->
      <div class="settings-section border-top-divider" style="padding-top: 24px;">
        <h3>Configure Git Remote URL</h3>
        <p class="settings-desc">Link a remote Git repository (e.g., origin or upstream), fetch its branches, and select one to track and check out locally.</p>
        
        <div class="remote-form-grid" style="display: grid; grid-template-columns: 120px 1fr; gap: 12px; align-items: center; margin-bottom: 16px;">
          <span style="font-weight: 600; color: #94a3b8; font-size: 0.85rem;">Remote Name:</span>
          <input 
            type="text" 
            placeholder="e.g. origin, upstream, fork" 
            bind:value={remoteName} 
            class="settings-input"
            style="width: 100%; max-width: 200px;"
          />
          
          <span style="font-weight: 600; color: #94a3b8; font-size: 0.85rem;">Remote URL:</span>
          <input 
            type="text" 
            placeholder="e.g. https://github.com/username/repository.git" 
            bind:value={remoteUrl} 
            class="settings-input"
            style="width: 100%;"
          />
        </div>

        <button class="btn btn-primary" on:click={handleConfigureRemote} disabled={isConfiguringRemote || !remoteName.trim() || !remoteUrl.trim()} style="width: fit-content; display: inline-flex; align-items: center; gap: 8px;">
          {#if isConfiguringRemote}
            <div class="loading-spinner-mini"></div>
            <span>Configuring & Fetching...</span>
          {:else}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>
            <span>Save Remote & Fetch Branches</span>
          {/if}
        </button>

        {#if remoteFeedback}
          <div class="settings-alert settings-alert-success" style="margin-top: 12px;">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
            <span>{remoteFeedback}</span>
          </div>
        {/if}

        {#if remoteFeedbackError}
          <div class="settings-alert settings-alert-error" style="margin-top: 12px;">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
            <span>{remoteFeedbackError}</span>
          </div>
        {/if}

        <!-- Checkout Remote Branch Sub-section -->
        {#if remotes.length > 0}
          <div style="margin-top: 24px; padding-top: 20px; border-top: 1px solid rgba(255, 255, 255, 0.05); display: flex; flex-direction: column; gap: 12px; text-align: left;">
            <h4 style="margin: 0; color: #ea580c; font-size: 0.95rem; font-weight: 600;">Checkout Remote Branch</h4>
            <p class="settings-desc" style="margin: 0;">Select a branch from any remote to checkout and track locally.</p>
            
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; align-items: end;">
              <!-- Select Remote -->
              <div style="display: flex; flex-direction: column; gap: 6px;">
                <span style="font-size: 0.75rem; font-weight: 600; color: #64748b; text-transform: uppercase;">1. Select Remote:</span>
                <select bind:value={selectedRemoteForCheckout} class="settings-input" style="height: 38px; cursor: pointer; background-color: #0b0f19; color: #fff; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 8px; padding: 0 10px;">
                  {#each remotes as r}
                    <option value={r} style="background-color: #0c101d;">{r}</option>
                  {/each}
                </select>
              </div>

              <!-- Select Branch -->
              <div style="display: flex; flex-direction: column; gap: 6px;">
                <span style="font-size: 0.75rem; font-weight: 600; color: #64748b; text-transform: uppercase;">2. Select Remote Branch:</span>
                <select bind:value={selectedRemoteBranch} on:change={handleRemoteBranchSelectionChange} class="settings-input" style="height: 38px; cursor: pointer; background-color: #0b0f19; color: #fff; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 8px; padding: 0 10px;">
                  <option value="" style="background-color: #0c101d;">-- Choose a branch --</option>
                  {#each remoteBranches.filter(b => b.startsWith(selectedRemoteForCheckout + '/')) as b}
                    <option value={b} style="background-color: #0c101d;">{b}</option>
                  {/each}
                </select>
              </div>

              <!-- Local Branch Name Input -->
              <div style="display: flex; flex-direction: column; gap: 6px;">
                <span style="font-size: 0.75rem; font-weight: 600; color: #64748b; text-transform: uppercase;">3. Local Branch Name:</span>
                <input 
                  type="text" 
                  placeholder="e.g. feature-login" 
                  bind:value={selectedLocalBranch} 
                  class="settings-input"
                  style="height: 38px;"
                />
              </div>
            </div>

            <button class="btn btn-primary" on:click={handleCheckoutRemoteBranch} disabled={isCheckingOut || !selectedRemoteBranch || !selectedLocalBranch.trim()} style="width: fit-content; margin-top: 8px; display: inline-flex; align-items: center; gap: 8px;">
              {#if isCheckingOut}
                <div class="loading-spinner-mini"></div>
                <span>Checking out...</span>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect><path d="M9 14l2 2 4-4"></path></svg>
                <span>Checkout Branch & Track</span>
              {/if}
            </button>
          </div>
        {/if}
      </div>

      <!-- Section 3: About App -->
      <div class="settings-section border-top-divider" style="padding-top: 24px;">
        <h3>About rGitExt</h3>
        <div class="about-info">
          <div class="about-row">
            <span class="about-lbl">App Version:</span>
            <span class="about-val">0.1.0 (Developer Build)</span>
          </div>
          <div class="about-row">
            <span class="about-lbl">Tauri Engine:</span>
            <span class="about-val">v2.0.0</span>
          </div>
          <div class="about-row">
            <span class="about-lbl">Active Git Path:</span>
            <span class="about-val monospaced-path">{customGitPath || 'System Default (Auto-Resolved)'}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
