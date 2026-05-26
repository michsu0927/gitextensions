<script>
  export let commits = [];
  export let isLoadingCommits = false;
  export let commitsError = '';
  export let selectedCommit = null;
  export let commitFiles = [];
  export let isLoadingCommitFiles = false;
  export let selectedCommitFile = '';
  export let selectedCommitFileDiff = '';
  export let isLoadingCommitFileDiff = false;
  export let commitFilesError = '';

  // callbacks
  export let performResetBranch;

  // Context menu state
  let contextMenuCommit = null;
  let contextMenuX = 0;
  let contextMenuY = 0;

  function handleContextMenu(e, commit) {
    if (!commit.hash) return;
    contextMenuCommit = commit;
    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
  }

  function closeContextMenu() {
    contextMenuCommit = null;
  }

  function handleResetToCommit() {
    if (!contextMenuCommit) return;
    const hash = contextMenuCommit.hash;
    closeContextMenu();
    const mode = prompt(`Reset current branch to ${hash.substring(0, 7)}?\n\nEnter mode: 'hard', 'mixed', or 'soft':`, "mixed");
    if (mode && ['hard', 'mixed', 'soft'].includes(mode.toLowerCase().trim())) {
      performResetBranch(hash, mode.toLowerCase().trim());
    } else if (mode) {
      alert("Invalid mode! Please enter 'hard', 'mixed', or 'soft'.");
    }
  }

  function selectCommit(commit) {
    if (!commit.hash || selectedCommit?.hash === commit.hash) return;
    selectedCommit = commit;
  }

  // Copy diff tooltip state
  let copyDiffTooltip = '';
  let copyDiffTooltipTimer = null;

  function copyDiffCode(diffText) {
    if (!diffText) return;
    const lines = diffText.split('\n');
    const cleaned = lines
      .filter(line => {
        // Skip diff metadata lines
        if (line.startsWith('diff --git')) return false;
        if (line.startsWith('index ')) return false;
        if (line.startsWith('---')) return false;
        if (line.startsWith('+++')) return false;
        if (line.startsWith('@@')) return false;
        if (line.startsWith('new file mode')) return false;
        if (line.startsWith('old mode') || line.startsWith('new mode')) return false;
        if (line.startsWith('deleted file mode')) return false;
        if (line.startsWith('similarity index') || line.startsWith('rename from') || line.startsWith('rename to')) return false;
        return true;
      })
      .map(line => {
        // Strip leading + or - (single character) from diff content lines
        if ((line.startsWith('+') || line.startsWith('-')) && !line.startsWith('+++') && !line.startsWith('---')) {
          return line.substring(1);
        }
        // Context lines start with a space in unified diff
        if (line.startsWith(' ')) {
          return line.substring(1);
        }
        return line;
      })
      .join('\n');

    navigator.clipboard.writeText(cleaned).then(() => {
      copyDiffTooltip = 'Copied!';
      if (copyDiffTooltipTimer) clearTimeout(copyDiffTooltipTimer);
      copyDiffTooltipTimer = setTimeout(() => { copyDiffTooltip = ''; }, 1500);
    }).catch(() => {
      copyDiffTooltip = 'Failed';
      if (copyDiffTooltipTimer) clearTimeout(copyDiffTooltipTimer);
      copyDiffTooltipTimer = setTimeout(() => { copyDiffTooltip = ''; }, 1500);
    });
  }

  function formatDiff(diffText) {
    if (!diffText) return '<div class="diff-empty">No changes or binary file</div>';
    const lines = diffText.split('\n');
    let html = '<div class="diff-container monospaced">';
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      let className = 'diff-line-text';
      let escapedLine = line
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');

      if (line.startsWith('+') && !line.startsWith('+++')) {
        className = 'diff-line-add';
      } else if (line.startsWith('-') && !line.startsWith('---')) {
        className = 'diff-line-del';
      } else if (line.startsWith('@@')) {
        className = 'diff-line-chunk';
      } else if (line.startsWith('diff --git') || line.startsWith('index') || line.startsWith('---') || line.startsWith('+++')) {
        className = 'diff-line-header';
      }
      html += `<div class="diff-line ${className}">${escapedLine || '&nbsp;'}</div>`;
    }
    html += '</div>';
    return html;
  }

  function formatDate(timestamp) {
    if (!timestamp) return '';
    const date = new Date(timestamp * 1000);
    return date.toLocaleDateString(undefined, { 
      year: 'numeric', 
      month: 'short', 
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
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

  // Parse ASCII graph symbols into styled color spans
  function highlightGraph(graphStr) {
    if (!graphStr) return '';
    
    // Cycle through HSL colors for distinct branch lines
    const colors = [
      '#f97316', // Orange (active)
      '#3b82f6', // Blue
      '#10b981', // Emerald
      '#a855f7', // Purple
      '#ec4899', // Pink
      '#eab308'  // Yellow
    ];

    let colorIndex = 0;
    let result = '';

    for (let char of graphStr) {
      if (char === '*' || char === 'o') {
        result += `<span class="graph-node" style="color: #f97316">${char}</span>`;
      } else if (char === '|' || char === '/' || char === '\\' || char === '_') {
        const color = colors[colors.length - 1 - (colorIndex % colors.length)];
        result += `<span class="graph-line" style="color: ${color}">${char}</span>`;
        colorIndex++;
      } else {
        result += char;
      }
    }
    return result;
  }
</script>

<div class="history-container">
  <div class="history-list-section">
    <div class="section-title">
      <h2>Commit History Log</h2>
      <span class="badge">{commits.filter(c => c.hash).length} commits</span>
    </div>

    {#if isLoadingCommits}
      <div class="loading-state">
        <div class="loading-spinner"></div>
        <p>Parsing Git repository log and branch graph...</p>
      </div>
    {:else if commitsError}
      <div class="alert alert-error">
        <h3>Failed to load commits</h3>
        <p>{commitsError}</p>
      </div>
    {:else}
      <div class="table-wrapper">
        <table class="history-table">
          <thead>
            <tr>
              <th class="col-graph">Graph</th>
              <th class="col-hash">Hash</th>
              <th class="col-subject">Subject</th>
              <th class="col-author">Author</th>
              <th class="col-date">Date</th>
            </tr>
          </thead>
          <tbody>
            {#each commits as commit}
              <tr class="commit-row {selectedCommit?.hash === commit.hash ? 'selected' : ''} {!commit.hash ? 'pure-graph-row' : ''}" on:click={() => selectCommit(commit)} on:contextmenu|preventDefault={(e) => handleContextMenu(e, commit)}>
                <td class="col-graph monospaced">
                  {@html highlightGraph(commit.graph)}
                </td>
                <td class="col-hash monospaced">
                  {commit.hash ? commit.hash.substring(0, 7) : ''}
                </td>
                <td class="col-subject">
                  {#if commit.refs && commit.refs.length > 0}
                    {#each commit.refs as ref}
                      <span class="ref-label {ref.startsWith('HEAD') ? 'ref-head' : ref.includes('/') ? 'ref-remote' : 'ref-local'}">{ref.replace('HEAD -> ', '')}</span>
                    {/each}
                  {/if}
                  <span class="subject-text">{commit.subject || ''}</span>
                </td>
                <td class="col-author">
                  {commit.author || ''}
                </td>
                <td class="col-date">
                  {commit.date ? formatDate(commit.date) : ''}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>

  <!-- Context Menu -->
  {#if contextMenuCommit}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="context-menu-overlay" on:click={closeContextMenu}></div>
    <div class="context-menu shadow-premium" style="left: {contextMenuX}px; top: {contextMenuY}px;">
      <div class="context-menu-header">Commit {contextMenuCommit.hash.substring(0, 7)}</div>
      <button class="dropdown-item" on:click={handleResetToCommit}>
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 2v6h6M2.66 15.57a10 10 0 1 0-.57-8.38l5.67-5.67"/></svg>
        <span>Reset current branch to here...</span>
      </button>
    </div>
  {/if}

  <!-- Commit Selection Drawer -->
  {#if selectedCommit}
    <div class="drawer shadow-premium">
      <div class="drawer-header">
        <h3>Commit Details</h3>
        <button class="close-btn" on:click={() => selectedCommit = null}>&times;</button>
      </div>
      <div class="drawer-body-split">
        <!-- Left panel: Metadata & Files list -->
        <div class="drawer-split-left">
          <div class="drawer-row">
            <span class="drawer-label">Hash:</span>
            <span class="drawer-val monospaced">{selectedCommit.hash}</span>
          </div>
          <div class="drawer-row">
            <span class="drawer-label">Author:</span>
            <span class="drawer-val">{selectedCommit.author} &lt;{selectedCommit.email}&gt;</span>
          </div>
          <div class="drawer-row">
            <span class="drawer-label">Date:</span>
            <span class="drawer-val">{formatDate(selectedCommit.date)}</span>
          </div>
          <div class="drawer-row full">
            <span class="drawer-label">Message:</span>
            <div class="message-box">
              {selectedCommit.subject}
            </div>
          </div>

          <hr class="drawer-divider" />

          <!-- Changed Files List -->
          <div class="drawer-row full flex-1 flex-col overflow-hidden">
            <span class="drawer-label margin-bottom-8">Modified Files ({commitFiles.length}):</span>
            <div class="commit-files-list-wrapper">
              {#if isLoadingCommitFiles}
                <div class="sidebar-loading-mini">
                  <div class="loading-spinner-mini inline-spinner"></div>
                  Loading files...
                </div>
              {:else if commitFilesError}
                <div class="sidebar-error-mini">{commitFilesError}</div>
              {:else if commitFiles.length === 0}
                <div class="sidebar-empty-mini">No modified files found.</div>
              {:else}
                {#each commitFiles as file}
                  {@const isFileSelected = selectedCommitFile === file.path}
                  <div 
                    class="commit-file-item {isFileSelected ? 'selected' : ''}" 
                    role="button" tabindex="0"
                    on:click={() => selectedCommitFile = file.path}
                    on:keydown={(e) => e.key === 'Enter' && (selectedCommitFile = file.path)}
                  >
                    <span class="file-item-status {statusClass(file.status)}">{statusLabel(file.status)}</span>
                    <span class="file-item-path" title={file.path}>{file.path}</span>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        </div>

        <!-- Right panel: Code Diff Viewer -->
        <div class="drawer-split-right">
          {#if selectedCommitFile}
            <div class="diff-viewer-header">
              <svg class="margin-right-6 tag-icon-color" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline></svg>
              <span class="diff-file-title" title={selectedCommitFile}>{selectedCommitFile}</span>
              <button class="btn-copy-diff" title="Copy diff code (without +/-)" on:click={() => copyDiffCode(selectedCommitFileDiff)}>
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                <span>Copy</span>
                {#if copyDiffTooltip}<span class="copy-tooltip">{copyDiffTooltip}</span>{/if}
              </button>
            </div>
            <div class="diff-viewer-body">
              {#if isLoadingCommitFileDiff}
                <div class="loading-state-mini padding-24">
                  <div class="loading-spinner-mini"></div>
                  <span>Fetching code changes...</span>
                </div>
              {:else}
                {@html formatDiff(selectedCommitFileDiff)}
              {/if}
            </div>
          {:else}
            <div class="diff-viewer-empty">
              <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#475569" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
              <span>Select a file from the list to view its diff.</span>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Tab-specific animations and structures */
  .full {
    grid-column: span 2;
  }
  .margin-bottom-8 {
    margin-bottom: 8px !important;
  }
  .overflow-hidden {
    overflow: hidden;
  }
  .context-menu-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    z-index: 999;
  }
  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--bg-secondary, #1e293b);
    border: 1px solid var(--border-color, #334155);
    border-radius: 8px;
    padding: 4px 0;
    min-width: 240px;
  }
  .context-menu-header {
    padding: 6px 14px;
    font-size: 0.75rem;
    color: var(--text-muted, #94a3b8);
    font-family: var(--font-mono);
    border-bottom: 1px solid var(--border-color, #334155);
    margin-bottom: 2px;
  }
  .ref-label {
    display: inline-block;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 4px;
    margin-right: 4px;
    vertical-align: middle;
    white-space: nowrap;
  }
  .ref-head {
    background: #f97316;
    color: #fff;
  }
  .ref-local {
    background: #3b82f6;
    color: #fff;
  }
  .ref-remote {
    background: #10b981;
    color: #fff;
  }
</style>
