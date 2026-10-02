// Network operations: fetch, pull, push and clone with their option dialogs, live progress
// and cancellation.

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import * as actions from './actions';
import { api } from './api';
import { confirmDialog, openDialog } from './dialog';
import { runOperation } from './operations';
import type { CloneOptions, PullOptions, PushOptions } from './types';
import { remotes } from '../stores/branches';
import { progress } from '../stores/ops';
import { currentRepoPath, gitStatus, tempRepoPath } from '../stores/repo';

const repo = (): string => get(currentRepoPath);

let opCounter = 0;

/** Runs a network command with live progress and a cancel button. */
export function runNetwork(
  title: string,
  call: (opId: string) => Promise<string>,
  onDone?: (ok: boolean) => void,
): void {
  const opId = `op-${Date.now()}-${++opCounter}`;
  progress.set({ opId, title, lines: [] });
  void runOperation(title, () => call(opId)).then((ok) => {
    progress.set(null);
    onDone?.(ok);
  });
}

export function cancelNetwork(): void {
  const current = get(progress);
  if (current) void api.cancelOperation(current.opId);
}

/** Fetches from all remotes. */
export function fetchAll(): void {
  runNetwork('Fetched all remotes.', (opId) => api.fetchRemote(repo(), opId, { all: true, prune: false }));
}

async function remoteChoices(): Promise<{ value: string; label: string }[]> {
  const known = get(remotes);
  const names = known.length > 0 ? known : await api.getRemotes(repo()).catch(() => [] as string[]);
  return names.map((n) => ({ value: n, label: n }));
}

export async function pullDialog(): Promise<void> {
  const choices = await remoteChoices();
  openDialog({
    title: 'Pull',
    description: 'Fetch from a remote and integrate the changes into the current branch.',
    fields: [
      {
        name: 'remote',
        label: 'Remote',
        kind: 'select',
        value: '',
        options: [{ value: '', label: "(the branch's upstream)" }, ...choices],
      },
      {
        name: 'branch',
        label: 'Remote branch (optional)',
        kind: 'text',
        value: '',
        placeholder: 'defaults to the upstream branch',
      },
      {
        name: 'mode',
        label: 'Integrate with',
        kind: 'select',
        value: '',
        options: [
          { value: '', label: 'Repository default' },
          { value: 'merge', label: 'Merge' },
          { value: 'rebase', label: 'Rebase' },
          { value: 'ff-only', label: 'Fast-forward only' },
        ],
      },
      { name: 'autostash', label: 'Stash local changes and re-apply them afterwards', kind: 'checkbox', value: true },
      {
        name: 'prune',
        label: 'Prune remote-tracking branches that were deleted on the remote',
        kind: 'checkbox',
        value: false,
      },
    ],
    submitLabel: 'Pull',
    onSubmit: (v) => {
      const options: PullOptions = {
        remote: String(v.remote) || undefined,
        branch: String(v.branch).trim() || undefined,
        mode: String(v.mode) as PullOptions['mode'],
        autostash: v.autostash === true,
        prune: v.prune === true,
      };
      runNetwork('Pull finished.', (opId) => api.pullChanges(repo(), opId, options));
    },
  });
}

export async function pushDialog(): Promise<void> {
  const choices = await remoteChoices();
  const branch = get(gitStatus)?.branch ?? '';
  const detached = branch.startsWith('HEAD');
  openDialog({
    title: 'Push',
    fields: [
      {
        name: 'remote',
        label: 'Remote',
        kind: 'select',
        value: choices.find((c) => c.value === 'origin')?.value ?? choices[0]?.value ?? '',
        options: choices,
      },
      { name: 'branch', label: 'Branch to push', kind: 'text', value: detached ? '' : branch, required: true },
      {
        name: 'remoteBranch',
        label: 'Name on the remote (optional)',
        kind: 'text',
        value: '',
        placeholder: 'same as the local branch',
      },
      { name: 'setUpstream', label: 'Track the remote branch (set upstream)', kind: 'checkbox', value: true },
      { name: 'tags', label: 'Also push all tags', kind: 'checkbox', value: false },
      {
        name: 'forceWithLease',
        label: 'Force, but only if nobody else pushed (--force-with-lease)',
        kind: 'checkbox',
        value: false,
      },
      { name: 'force', label: 'Force push (overwrites the remote branch!)', kind: 'checkbox', value: false },
    ],
    submitLabel: 'Push',
    onSubmit: (v) => {
      const options: PushOptions = {
        remote: String(v.remote) || undefined,
        branch: String(v.branch).trim() || undefined,
        remoteBranch: String(v.remoteBranch).trim() || undefined,
        setUpstream: v.setUpstream === true,
        tags: v.tags === true,
        forceWithLease: v.forceWithLease === true,
        force: v.force === true,
      };
      const go = () => runNetwork('Push finished.', (opId) => api.pushChanges(repo(), opId, options));
      if (options.force) {
        confirmDialog(
          'Force push?',
          `This overwrites '${options.branch}' on '${options.remote}', even if it contains commits you do not have.`,
          'Force push',
          go,
          true,
        );
      } else {
        go();
      }
    },
  });
}

function folderOf(path: string): string {
  const i = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return i > 0 ? path.slice(0, i) : path;
}

function repoNameFromUrl(url: string): string {
  const last = url
    .trim()
    .replace(/[\\/]+$/, '')
    .split(/[\\/:]/)
    .pop();
  return (last ?? '').replace(/\.git$/, '');
}

export function cloneDialog(): void {
  openDialog({
    title: 'Clone repository',
    fields: [
      { name: 'url', label: 'Repository URL', kind: 'text', value: '', required: true, placeholder: 'https://… or git@…' },
      { name: 'parent', label: 'Parent folder', kind: 'text', value: folderOf(repo()), required: true },
      { name: 'name', label: 'Folder name (optional)', kind: 'text', value: '', placeholder: 'derived from the URL' },
      { name: 'branch', label: 'Branch (optional)', kind: 'text', value: '' },
      { name: 'depth', label: 'Shallow clone depth (optional)', kind: 'text', value: '', placeholder: 'e.g. 1' },
      { name: 'recurse', label: 'Also clone submodules', kind: 'checkbox', value: false },
    ],
    submitLabel: 'Clone',
    onSubmit: (v) => {
      const url = String(v.url).trim();
      const name = String(v.name).trim() || repoNameFromUrl(url);
      const destination = `${String(v.parent).trim().replace(/[\\/]+$/, '')}/${name}`;
      const depth = Number.parseInt(String(v.depth), 10);
      const options: CloneOptions = {
        branch: String(v.branch).trim() || undefined,
        depth: Number.isFinite(depth) && depth > 0 ? depth : undefined,
        recurseSubmodules: v.recurse === true,
      };
      let cloned = '';
      runNetwork(
        `Cloned ${name}.`,
        async (opId) => {
          cloned = await api.cloneRepo(url, destination, opId, options);
          return cloned;
        },
        (ok) => {
          if (ok && cloned) {
            // Open the new repository.
            tempRepoPath.set(cloned);
            void actions.saveRepoPath();
          }
        },
      );
    },
  });
}

interface ProgressPayload {
  opId: string;
  kind: 'stdout' | 'stderr';
  text: string;
}

const MAX_PROGRESS_LINES = 200;

/** Subscribes to the live output of network operations. Returns the unlisten function. */
export async function startProgressListener(): Promise<UnlistenFn | undefined> {
  try {
    return await listen<ProgressPayload>('git-progress', (event) => {
      progress.update((p) =>
        p && p.opId === event.payload.opId
          ? { ...p, lines: [...p.lines.slice(-(MAX_PROGRESS_LINES - 1)), event.payload.text] }
          : p,
      );
    });
  } catch (err) {
    console.error('Failed to listen for progress events:', err);
    return undefined;
  }
}
