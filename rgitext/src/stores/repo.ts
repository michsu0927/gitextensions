import { writable } from 'svelte/store';
import type { GitStatus } from '../lib/types';

// Active repository path (header)
export const currentRepoPath = writable('');
export const tempRepoPath = writable('');
export const isEditingPath = writable(false);
export const pathError = writable('');

// `git status` summary
export const gitStatus = writable<GitStatus | null>(null);
export const isLoadingStatus = writable(false);
export const statusError = writable('');
