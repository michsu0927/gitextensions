import { writable } from 'svelte/store';
import type { RepoState } from '../lib/types';

/** Operation (merge, rebase, ...) currently in progress in the repository. */
export const repoState = writable<RepoState>({ operation: null, conflicted: [] });

/** Result of the last user-triggered operation, shown as a dismissible message. */
export const opMessage = writable('');
export const opError = writable('');
export const opBusy = writable(false);

/** Set to the target of an interactive rebase to open the todo list dialog. */
export const rebaseDialogOnto = writable<string | null>(null);
