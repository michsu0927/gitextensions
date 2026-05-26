<script>
  export let show = false;
  export let expanded = false;
  export let logs = [];
  export let onClear;

  let activeTab = 'console'; // 'console' | 'terminal' | 'issues'
  let filter = 'all'; // 'all' | 'success' | 'error'
  let searchQuery = '';
  let expandedLogs = {};
  let showCopiedAlert = false;

  $: successCount = logs.filter(l => l.status === 'success').length;
  $: errorCount = logs.filter(l => l.status === 'error').length;

  $: filteredLogs = logs.filter(log => {
    const matchesFilter = filter === 'all' || 
                         (filter === 'success' && log.status === 'success') || 
                         (filter === 'error' && log.status === 'error');
    
    const matchesSearch = !searchQuery || 
                         log.command.toLowerCase().includes(searchQuery.toLowerCase()) || 
                         (log.args && JSON.stringify(log.args).toLowerCase().includes(searchQuery.toLowerCase())) || 
                         (log.result && JSON.stringify(log.result).toLowerCase().includes(searchQuery.toLowerCase())) || 
                         (log.error && log.error.toLowerCase().includes(searchQuery.toLowerCase()));
    
    return matchesFilter && matchesSearch;
  });

  function toggleLogExpand(id) {
    expandedLogs = {
      ...expandedLogs,
      [id]: !expandedLogs[id]
    };
  }

  function copyLogs() {
    const logsText = JSON.stringify(filteredLogs, null, 2);
    navigator.clipboard.writeText(logsText).then(() => {
      showCopiedAlert = true;
      setTimeout(() => {
        showCopiedAlert = false;
      }, 2000);
    });
  }

  function formatJson(val) {
    if (val === null || val === undefined) return '';
    if (typeof val === 'string') {
      try {
        // Try parsing string as JSON first
        const parsed = JSON.parse(val);
        return JSON.stringify(parsed, null, 2);
      } catch (e) {
        return val;
      }
    }
    return JSON.stringify(val, null, 2);
  }
</script>

{#if showCopiedAlert}
  <div class="toast-alert">
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#10b981" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
    <span>Logs copied to clipboard!</span>
  </div>
{/if}

<div class="console-drawer {show ? 'show' : ''} {expanded ? 'expanded' : ''}">
  <!-- Console Header -->
  <div class="console-header">
    <div class="console-tabs">
      <button 
        class="console-tab {activeTab === 'console' ? 'active' : ''}" 
        on:click={() => activeTab = 'console'}
      >
        Console
        {#if logs.length > 0}
          <span class="tab-badge">{logs.length}</span>
        {/if}
      </button>
      <button 
        class="console-tab {activeTab === 'terminal' ? 'active' : ''}" 
        on:click={() => activeTab = 'terminal'}
      >
        Terminal
      </button>
      <button 
        class="console-tab {activeTab === 'issues' ? 'active' : ''}" 
        on:click={() => activeTab = 'issues'}
      >
        Issues
        {#if errorCount > 0}
          <span class="tab-badge error">{errorCount}</span>
        {/if}
      </button>
    </div>

    <!-- Utilities Toolbar -->
    <div class="console-toolbar">
      {#if activeTab === 'console'}
        <!-- Search Input -->
        <div class="toolbar-search-wrapper">
          <svg class="search-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
          <input 
            type="text" 
            placeholder="Filter by query..." 
            bind:value={searchQuery} 
            class="toolbar-search"
          />
          {#if searchQuery}
            <button class="clear-search-btn" on:click={() => searchQuery = ''}>&times;</button>
          {/if}
        </div>

        <!-- Filter Select -->
        <div class="filter-select-wrapper">
          <select bind:value={filter} class="toolbar-select">
            <option value="all">All Logs ({logs.length})</option>
            <option value="success">Success ({successCount})</option>
            <option value="error">Errors ({errorCount})</option>
          </select>
        </div>

        <!-- Clear Button -->
        <button class="toolbar-btn" on:click={onClear} title="Clear logs">
          <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
          <span>Clear</span>
        </button>

        <!-- Copy Logs Button -->
        <button class="toolbar-btn" on:click={copyLogs} title="Copy filtered logs to clipboard">
          <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
          <span>Copy</span>
        </button>
      {/if}

      <div class="toolbar-divider"></div>

      <!-- Maximize/Minimize Toggle -->
      <button 
        class="toolbar-btn icon-only" 
        on:click={() => expanded = !expanded} 
        title={expanded ? "Minimize Console" : "Maximize Console"}
      >
        {#if expanded}
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7"></path></svg>
        {:else}
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3M21 8V5a2 2 0 0 0-2-2h-3M3 16v3a2 2 0 0 0 2 2h3M16 21h3a2 2 0 0 0 2-2v-3M10 21V10H21"></path></svg>
        {/if}
      </button>

      <!-- Close button -->
      <button 
        class="toolbar-btn icon-only close-drawer-btn" 
        on:click={() => show = false} 
        title="Close Console"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>
  </div>

  <!-- Console Content Body -->
  <div class="console-body">
    {#if activeTab === 'console'}
      {#if filteredLogs.length === 0}
        <div class="console-empty-state">
          <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" class="empty-icon"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>
          <h3>No logs yet</h3>
          <p>Execute Git operations or stage, commit, pull, and push files to view details in the console.</p>
        </div>
      {:else}
        <div class="logs-list">
          {#each filteredLogs as log (log.id)}
            <div class="log-item {expandedLogs[log.id] ? 'expanded' : ''} {log.status}">
              <!-- Log Summary Line (Clickable Header) -->
              <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
              <div class="log-summary" on:click={() => toggleLogExpand(log.id)}>
                <span class="expand-arrow">
                  <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
                </span>
                
                <span class="status-badge {log.status}">
                  {log.status === 'success' ? 'SUCCESS' : log.status === 'error' ? 'ERROR' : 'PENDING'}
                </span>
                
                <span class="log-command-name">
                  {#if log.gitCommands && log.gitCommands.length > 0}
                    <strong class="highlight git-cmd">{log.gitCommands[0]}</strong>
                    {#if log.gitCommands.length > 1}
                      <span class="multi-cmd-badge">+{log.gitCommands.length - 1} more</span>
                    {/if}
                    <span class="invoke-tag">invoke("{log.command}")</span>
                  {:else}
                    tauri.invoke("<strong class="highlight">{log.command}</strong>")
                  {/if}
                </span>

                <span class="log-timestamp">{log.timestamp}</span>
              </div>

              <!-- Log Details Section (Slide-out accordion) -->
              {#if expandedLogs[log.id]}
                <div class="log-details">
                  <!-- Section: Executed Shell Git Commands -->
                  {#if log.gitCommands && log.gitCommands.length > 0}
                    <div class="details-section">
                      <div class="section-title">Execute command:</div>
                      {#each log.gitCommands as cmd}
                        <pre class="json-block command-line-block"><code>{cmd}</code></pre>
                      {/each}
                    </div>
                  {/if}
                  <!-- Section: Arguments -->
                  {#if log.args && Object.keys(log.args).length > 0}
                    <div class="details-section">
                      <div class="section-title">Arguments:</div>
                      <pre class="json-block"><code>{formatJson(log.args)}</code></pre>
                    </div>
                  {/if}

                  <!-- Section: Success Result -->
                  {#if log.status === 'success'}
                    {#if log.result !== undefined && log.result !== null && log.result !== ''}
                      <div class="details-section">
                        <div class="section-title">Response payload:</div>
                        <pre class="json-block success"><code>{formatJson(log.result)}</code></pre>
                      </div>
                    {:else}
                      <div class="details-section">
                        <div class="section-title">Response:</div>
                        <span class="empty-response-tag">Empty response (command ran successfully)</span>
                      </div>
                    {/if}
                  {/if}

                  <!-- Section: Error details -->
                  {#if log.status === 'error' && log.error}
                    <div class="details-section">
                      <div class="section-title error">Execution Error Trace:</div>
                      <pre class="json-block error"><code>{log.error}</code></pre>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

    {:else if activeTab === 'terminal'}
      <div class="console-empty-state">
        <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" class="empty-icon"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect><line x1="8" y1="21" x2="16" y2="21"></line><line x1="12" y1="17" x2="12" y2="21"></line></svg>
        <h3>No active terminal sessions</h3>
        <p>Terminal output logs are currently piped to the primary application execution console.</p>
      </div>

    {:else if activeTab === 'issues'}
      {#if errorCount === 0}
        <div class="console-empty-state">
          <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#10b981" stroke-width="1" class="empty-icon"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>
          <h3>Clean Sheet!</h3>
          <p>No backend execution errors or tracebacks detected in the current session workspace.</p>
        </div>
      {:else}
        <div class="logs-list">
          {#each logs.filter(l => l.status === 'error') as log (log.id)}
            <div class="log-item expanded error">
              <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
              <div class="log-summary" on:click={() => toggleLogExpand(log.id)}>
                <span class="expand-arrow">
                  <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="9 18 15 12 9 6"></polyline></svg>
                </span>
                
                <span class="status-badge error">ERROR</span>
                
                <span class="log-command-name">
                  {#if log.gitCommands && log.gitCommands.length > 0}
                    <strong class="highlight git-cmd">{log.gitCommands[0]}</strong>
                    {#if log.gitCommands.length > 1}
                      <span class="multi-cmd-badge">+{log.gitCommands.length - 1} more</span>
                    {/if}
                    <span class="invoke-tag">invoke("{log.command}")</span>
                  {:else}
                    tauri.invoke("<strong class="highlight">{log.command}</strong>")
                  {/if}
                </span>

                <span class="log-timestamp">{log.timestamp}</span>
              </div>

              <div class="log-details">
                {#if log.args && Object.keys(log.args).length > 0}
                  <div class="details-section">
                    <div class="section-title">Arguments:</div>
                    <pre class="json-block"><code>{formatJson(log.args)}</code></pre>
                  </div>
                {/if}
                <div class="details-section">
                  <div class="section-title error">Execution Error Trace:</div>
                  <pre class="json-block error"><code>{log.error}</code></pre>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>
