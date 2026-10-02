// User-triggered operations on commits and refs (merge, rebase, cherry-pick, ...) with their
// option dialogs. Results are reported through the `opMessage` / `opError` stores.

import { get } from 'svelte/store';
import * as actions from './actions';
import { api } from './api';
import { confirmDialog, openDialog } from './dialog';
import { shortHash } from './format';
import type { ControlAction, LocalChangesMode, MergeOptions, RevisionRow } from './types';
import { currentRepoPath, gitStatus } from '../stores/repo';
import { opBusy, opError, opMessage, rebaseDialogOnto } from '../stores/ops';

const repo = (): string => get(currentRepoPath);

/** Runs an operation, reports its outcome and refreshes everything it may have changed. */
export async function runOperation(label: string, run: () => Promise<string | void>): Promise<boolean> {
  opBusy.set(true);
  opError.set('');
  opMessage.set('');
  let ok = false;
  try {
    const result = await run();
    opMessage.set(result ? `${label}\n${result}` : label);
    ok = true;
  } catch (e) {
    opError.set(String(e));
  } finally {
    opBusy.set(false);
  }
  actions.refreshAll();
  return ok;
}

export function dismissOperationResult(): void {
  opMessage.set('');
  opError.set('');
}

// ---------------------------------------------------------------------------
// In-progress operation
// ---------------------------------------------------------------------------

export function controlOperation(action: ControlAction): Promise<boolean> | void {
  const labels: Record<ControlAction, string> = {
    continue: 'Continued',
    skip: 'Skipped the current step of',
    abort: 'Aborted',
  };
  const run = (): Promise<boolean> =>
    runOperation(`${labels[action]} the operation.`, () => api.operationControl(repo(), action));
  if (action === 'abort') {
    confirmDialog(
      'Abort the operation?',
      'Everything done so far by this operation is discarded and the repository returns to the state before it started.',
      'Abort',
      () => void run(),
      true,
    );
    return;
  }
  return run();
}

// ---------------------------------------------------------------------------
// Branches and tags
// ---------------------------------------------------------------------------

export function createBranchAt(hash: string): void {
  openDialog({
    title: `Create branch at ${shortHash(hash)}`,
    fields: [
      { name: 'name', label: 'Branch name', kind: 'text', value: '', required: true },
      { name: 'checkout', label: 'Switch to the new branch', kind: 'checkbox', value: true },
    ],
    submitLabel: 'Create branch',
    onSubmit: (v) =>
      void runOperation('Branch created.', () =>
        api.createBranch(repo(), String(v.name).trim(), hash, v.checkout === true),
      ),
  });
}

export function createTagAt(hash: string): void {
  openDialog({
    title: `Create tag at ${shortHash(hash)}`,
    fields: [
      { name: 'name', label: 'Tag name', kind: 'text', value: '', required: true },
      {
        name: 'message',
        label: 'Message',
        kind: 'textarea',
        value: '',
        hint: 'Leave empty for a lightweight tag; a message creates an annotated tag.',
      },
      { name: 'sign', label: 'Sign the tag with GPG (needs a message)', kind: 'checkbox', value: false },
    ],
    submitLabel: 'Create tag',
    onSubmit: (v) =>
      void runOperation('Tag created.', () =>
        api.createTag(repo(), String(v.name).trim(), hash, String(v.message), v.sign === true),
      ),
  });
}

export function deleteLocalBranch(name: string): void {
  openDialog({
    title: `Delete branch '${name}'?`,
    description: 'Unmerged commits on this branch may become unreachable.',
    fields: [{ name: 'force', label: 'Force delete (even if not merged)', kind: 'checkbox', value: false }],
    submitLabel: 'Delete branch',
    danger: true,
    onSubmit: (v) => void runOperation('Branch deleted.', () => api.deleteBranch(repo(), name, v.force === true)),
  });
}

export function deleteTagByName(name: string): void {
  confirmDialog(
    `Delete tag '${name}'?`,
    'The tag is only removed locally.',
    'Delete tag',
    () => void runOperation('Tag deleted.', () => api.deleteTag(repo(), name)),
    true,
  );
}

function hasLocalChanges(): boolean {
  const s = get(gitStatus);
  return !!s && s.staged_count + s.unstaged_count + s.untracked_count > 0;
}

/** Switches to a branch or commit; asks what to do with uncommitted changes when there are any. */
export function checkoutRevision(target: string, display = target): void {
  const run = (mode: LocalChangesMode) =>
    void runOperation(`Checked out ${display}.`, () => api.checkoutBranch(repo(), target, mode));
  if (!hasLocalChanges()) {
    run('keep');
    return;
  }
  openDialog({
    title: `Check out ${display}`,
    description: 'You have uncommitted changes. What should happen to them?',
    fields: [
      {
        name: 'mode',
        label: 'Local changes',
        kind: 'select',
        value: 'stash',
        options: [
          { value: 'stash', label: 'Stash, switch, then re-apply' },
          { value: 'merge', label: 'Carry them over (merge)' },
          { value: 'keep', label: "Don't touch them (fails on conflicts)" },
          { value: 'reset', label: 'Discard them (cannot be undone)' },
        ],
      },
    ],
    submitLabel: 'Check out',
    onSubmit: (v) => run(String(v.mode) as LocalChangesMode),
  });
}

// ---------------------------------------------------------------------------
// Merge, rebase, cherry-pick, revert, reset
// ---------------------------------------------------------------------------

export function mergeInto(target: string, display = shortHash(target)): void {
  const current = get(gitStatus)?.branch || 'the current branch';
  openDialog({
    title: `Merge ${display} into ${current}`,
    fields: [
      {
        name: 'strategy',
        label: 'Merge type',
        kind: 'select',
        value: 'default',
        options: [
          { value: 'default', label: 'Default (fast-forward when possible)' },
          { value: 'no-ff', label: 'Always create a merge commit' },
          { value: 'ff-only', label: 'Fast-forward only' },
          { value: 'squash', label: 'Squash (stage the changes, do not commit)' },
        ],
      },
      { name: 'noCommit', label: 'Do not commit the result', kind: 'checkbox', value: false },
      { name: 'message', label: 'Merge message (optional)', kind: 'textarea', value: '' },
    ],
    submitLabel: 'Merge',
    onSubmit: (v) => {
      const options: MergeOptions = {
        strategy: String(v.strategy) as MergeOptions['strategy'],
        noCommit: v.noCommit === true,
        message: String(v.message).trim() || undefined,
      };
      void runOperation(`Merged ${display}.`, () => api.mergeBranch(repo(), target, options));
    },
  });
}

export function rebaseOnto(target: string, display = shortHash(target)): void {
  const current = get(gitStatus)?.branch || 'the current branch';
  openDialog({
    title: `Rebase ${current} onto ${display}`,
    description: 'The commits of the current branch are replayed on top of the target.',
    fields: [{ name: 'autostash', label: 'Stash local changes and re-apply them afterwards', kind: 'checkbox', value: true }],
    submitLabel: 'Rebase',
    onSubmit: (v) =>
      void runOperation(`Rebased onto ${display}.`, () => api.rebaseBranch(repo(), target, v.autostash === true)),
  });
}

export function interactiveRebaseOnto(target: string): void {
  rebaseDialogOnto.set(target);
}

function pickFields(row: RevisionRow, withOrigin: boolean) {
  const fields: Parameters<typeof openDialog>[0]['fields'] = [];
  if (withOrigin) {
    fields.push({
      name: 'recordOrigin',
      label: 'Append "(cherry picked from commit …)" to the message',
      kind: 'checkbox',
      value: false,
    });
  }
  fields.push({ name: 'noCommit', label: 'Apply the changes without committing', kind: 'checkbox', value: false });
  if (row.parents.length > 1) {
    fields.push({
      name: 'mainline',
      label: 'This is a merge commit. Use parent',
      kind: 'select',
      value: '1',
      options: row.parents.map((p, i) => ({ value: String(i + 1), label: `${i + 1}: ${shortHash(p)}` })),
    });
  }
  return fields;
}

export function cherryPick(row: RevisionRow): void {
  openDialog({
    title: `Cherry-pick ${shortHash(row.hash)}`,
    description: row.subject,
    fields: pickFields(row, true),
    submitLabel: 'Cherry-pick',
    onSubmit: (v) =>
      void runOperation('Cherry-picked.', () =>
        api.cherryPick(repo(), [row.hash], {
          recordOrigin: v.recordOrigin === true,
          noCommit: v.noCommit === true,
          mainline: v.mainline ? Number(v.mainline) : undefined,
        }),
      ),
  });
}

export function revertCommit(row: RevisionRow): void {
  openDialog({
    title: `Revert ${shortHash(row.hash)}`,
    description: row.subject,
    fields: pickFields(row, false),
    submitLabel: 'Revert',
    onSubmit: (v) =>
      void runOperation('Reverted.', () =>
        api.revertCommits(repo(), [row.hash], {
          noCommit: v.noCommit === true,
          mainline: v.mainline ? Number(v.mainline) : undefined,
        }),
      ),
  });
}

export function resetTo(hash: string): void {
  openDialog({
    title: `Reset current branch to ${shortHash(hash)}`,
    fields: [
      {
        name: 'mode',
        label: 'Mode',
        kind: 'select',
        value: 'mixed',
        options: [
          { value: 'soft', label: 'Soft: keep index and working tree' },
          { value: 'mixed', label: 'Mixed: keep working tree, reset index' },
          { value: 'hard', label: 'Hard: discard all local changes' },
        ],
      },
    ],
    submitLabel: 'Reset',
    danger: true,
    onSubmit: (v) => {
      const mode = String(v.mode);
      const go = () => void runOperation('Reset done.', () => api.resetCurrentBranch(repo(), hash, mode));
      if (mode === 'hard') {
        confirmDialog(
          'Discard all local changes?',
          'A hard reset throws away every uncommitted change. This cannot be undone.',
          'Hard reset',
          go,
          true,
        );
      } else {
        go();
      }
    },
  });
}
