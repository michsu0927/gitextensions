import { writable } from 'svelte/store';
import type { CommitFile, CommitItem } from '../lib/types';

export const commits = writable<CommitItem[]>([]);
export const isLoadingCommits = writable(false);
export const commitsError = writable('');
export const selectedCommit = writable<CommitItem | null>(null);

// Files and diff of the selected commit
export const commitFiles = writable<CommitFile[]>([]);
export const isLoadingCommitFiles = writable(false);
export const selectedCommitFile = writable('');
export const selectedCommitFileDiff = writable('');
export const isLoadingCommitFileDiff = writable(false);
export const commitFilesError = writable('');
