<script>
  export let gitVersion;
  export let gitError;
  export let isChecking;
  export let gitStatus;
  export let statusError;
  export let isLoadingStatus;

  // callbacks
  export let checkGit;
</script>

<div class="dashboard-hero">
  <div class="hero-bg-gradient"></div>
  <h1>Welcome to rGitExt</h1>
  <p class="subtitle">A premium high-performance GitExtensions desktop client GUI running inside a safe sandboxed web container.</p>
</div>

<div class="grid-layout">
  <!-- Git Status Card -->
  <div class="card glass full-width">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>
      <h3>System Git Environment</h3>
    </div>
    <div class="card-body">
      {#if isChecking}
        <div class="loading-state-mini">
          <div class="loading-spinner-mini"></div>
          <span>Verifying core Git installation...</span>
        </div>
      {:else if gitVersion}
        <div class="version-badge">
          <span class="dot success"></span>
          <span>{gitVersion}</span>
        </div>
        <p class="description">Git binary successfully detected and mapped. Ready to perform high-performance versioning operations.</p>
      {:else}
        <div class="settings-alert settings-alert-error">
          <span class="dot error"></span>
          <span>Git Executable Not Found</span>
        </div>
        <p class="description error">{gitError || 'Make sure git is installed and in your system PATH, or specify custom path in Settings.'}</p>
      {/if}
      <button class="btn btn-secondary" on:click={checkGit} disabled={isChecking}>Re-check environment</button>
    </div>
  </div>

  <!-- Repository Quick Summary -->
  <div class="card glass full-width">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
      <h3>Active Repository Summary</h3>
    </div>
    <div class="card-body">
      {#if isLoadingStatus}
        <div class="loading-state-mini">
          <div class="loading-spinner-mini"></div>
          <span>Reading active workspace status...</span>
        </div>
      {:else if gitStatus}
        <div class="status-summary-grid">
          <div class="status-summary-item">
            <span class="status-summary-lbl">Current Branch</span>
            <span class="status-summary-val highlight-branch">{gitStatus.branch || 'DETACHED HEAD'}</span>
          </div>
          <div class="status-summary-item">
            <span class="status-summary-lbl">Staged</span>
            <span class="status-summary-val staged-color">{gitStatus.staged_count} files</span>
          </div>
          <div class="status-summary-item">
            <span class="status-summary-lbl">Modified</span>
            <span class="status-summary-val modified-color">{gitStatus.unstaged_count} files</span>
          </div>
          <div class="status-summary-item">
            <span class="status-summary-lbl">Untracked</span>
            <span class="status-summary-val untracked-color">{gitStatus.untracked_count} files</span>
          </div>
        </div>
        
        <div class="status-detail-console">
          <pre>{gitStatus.raw_status}</pre>
        </div>
      {:else if statusError}
        <div class="settings-alert settings-alert-error">
          <strong>Status Error:</strong> {statusError}
        </div>
      {:else}
        <p class="description">No repository opened or unable to read status.</p>
      {/if}
    </div>
  </div>

  <!-- System Architecture Card -->
  <div class="card glass full-width">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect><rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>
      <h3>Tauri Sandbox Stack</h3>
    </div>
    <div class="card-body tech-stack">
      <div class="tech-item">
        <span class="tech-title">Backend engine</span>
        <span class="tech-value highlight-rust">Rust 1.75+</span>
      </div>
      <div class="tech-item">
        <span class="tech-title">GUI wrapper</span>
        <span class="tech-value highlight-tauri">Tauri v2</span>
      </div>
      <div class="tech-item">
        <span class="tech-title">Frontend framework</span>
        <span class="tech-value highlight-svelte">Svelte & Vite</span>
      </div>
      <div class="tech-item">
        <span class="tech-title">Styling</span>
        <span class="tech-value highlight-css">Vanilla CSS</span>
      </div>
    </div>
  </div>
</div>
