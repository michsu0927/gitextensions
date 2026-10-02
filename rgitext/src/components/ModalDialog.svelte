<script lang="ts">
  import { tick } from 'svelte';
  import { closeDialog, dialog, type FieldValue } from '../lib/dialog';

  let values: Record<string, FieldValue> = {};
  let error = '';
  let busy = false;
  let initializedFor: unknown = null;
  let container: HTMLDivElement | undefined;

  // Initialise the form every time a new dialog is opened.
  $: if ($dialog !== initializedFor) {
    initializedFor = $dialog;
    values = Object.fromEntries(($dialog?.fields ?? []).map((f) => [f.name, f.value]));
    error = '';
    busy = false;
    if ($dialog) void focusFirstField();
  }

  async function focusFirstField(): Promise<void> {
    await tick();
    container?.querySelector<HTMLElement>('input:not([type=checkbox]), textarea, select')?.focus();
  }

  function missingRequired(): string | null {
    for (const f of $dialog?.fields ?? []) {
      if (f.required && f.kind !== 'checkbox' && !String(values[f.name] ?? '').trim()) {
        return `${f.label} is required.`;
      }
    }
    return null;
  }

  async function submit(): Promise<void> {
    const spec = $dialog;
    if (!spec || busy) return;
    const problem = missingRequired();
    if (problem) {
      error = problem;
      return;
    }
    busy = true;
    error = '';
    try {
      closeDialogIfCurrent(spec);
      await spec.onSubmit(values);
    } catch (e) {
      // The dialog is already closed; failures are reported by the caller. Keep a trace anyway.
      console.error('Dialog action failed:', e);
    }
  }

  function closeDialogIfCurrent(spec: unknown): void {
    if ($dialog === spec) closeDialog();
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      closeDialog();
    } else if (event.key === 'Enter' && !(event.target instanceof HTMLTextAreaElement)) {
      event.preventDefault();
      void submit();
    }
  }
</script>

{#if $dialog}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="md-overlay" on:click|self={closeDialog}>
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div class="md" role="dialog" aria-modal="true" aria-label={$dialog.title} bind:this={container} on:keydown={onKeyDown}>
      <h3 class="md-title">{$dialog.title}</h3>
      {#if $dialog.description}
        <p class="md-description">{$dialog.description}</p>
      {/if}
      {#each $dialog.fields as field (field.name)}
        <div class="md-field">
          {#if field.kind === 'checkbox'}
            <label class="md-check">
              <input
                type="checkbox"
                checked={values[field.name] === true}
                on:change={(e) => (values[field.name] = e.currentTarget.checked)}
              />
              {field.label}
            </label>
          {:else}
            <label for="md-{field.name}">{field.label}{field.required ? ' *' : ''}</label>
            {#if field.kind === 'text'}
              <input
                id="md-{field.name}"
                type="text"
                placeholder={field.placeholder ?? ''}
                value={String(values[field.name] ?? '')}
                on:input={(e) => (values[field.name] = e.currentTarget.value)}
              />
            {:else if field.kind === 'textarea'}
              <textarea
                id="md-{field.name}"
                rows="4"
                placeholder={field.placeholder ?? ''}
                value={String(values[field.name] ?? '')}
                on:input={(e) => (values[field.name] = e.currentTarget.value)}
              ></textarea>
            {:else}
              <select
                id="md-{field.name}"
                value={String(values[field.name] ?? '')}
                on:change={(e) => (values[field.name] = e.currentTarget.value)}
              >
                {#each field.options ?? [] as option}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            {/if}
          {/if}
          {#if field.hint}<span class="md-hint">{field.hint}</span>{/if}
        </div>
      {/each}
      {#if error}<div class="md-error">{error}</div>{/if}
      <div class="md-actions">
        <button class="md-cancel" on:click={closeDialog}>Cancel</button>
        <button class="md-submit" class:danger={$dialog.danger} disabled={busy} on:click={submit}>
          {$dialog.submitLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .md-overlay {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(2, 6, 23, 0.65);
  }
  .md {
    width: min(480px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    padding: 20px 22px;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
    color: #e5e7eb;
  }
  .md-title {
    margin: 0 0 6px;
    font-size: 1.05rem;
  }
  .md-description {
    margin: 0 0 14px;
    color: #94a3b8;
    font-size: 0.85rem;
    white-space: pre-wrap;
  }
  .md-field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 12px;
    font-size: 0.82rem;
  }
  .md-field label {
    color: #94a3b8;
  }
  .md-check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: #cbd5e1 !important;
  }
  .md-field input[type='text'],
  .md-field textarea,
  .md-field select {
    background: #0b0f19;
    color: #e5e7eb;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 7px 10px;
    font-family: inherit;
    font-size: 0.85rem;
    outline: none;
  }
  .md-field input:focus,
  .md-field textarea:focus,
  .md-field select:focus {
    border-color: rgba(234, 88, 12, 0.7);
  }
  .md-hint {
    color: #64748b;
    font-size: 0.75rem;
  }
  .md-error {
    margin-bottom: 10px;
    color: #fca5a5;
    font-size: 0.82rem;
  }
  .md-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 6px;
  }
  .md-actions button {
    border-radius: 8px;
    padding: 7px 16px;
    font-size: 0.85rem;
    cursor: pointer;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: transparent;
    color: #cbd5e1;
  }
  .md-actions .md-submit {
    background: #ea580c;
    border-color: #ea580c;
    color: #fff;
    font-weight: 600;
  }
  .md-actions .md-submit.danger {
    background: #dc2626;
    border-color: #dc2626;
  }
  .md-actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
