import { writable } from 'svelte/store';
import type { DiffFile, ExpandedDirs, SelectedWorkingFile, WorkingFile } from '../lib/types';

export const workingFiles = writable<WorkingFile[]>([]);
export const isLoadingWorkingFiles = writable(false);
export const workingFilesError = writable('');
export const selectedWorkingFile = writable<SelectedWorkingFile | null>(null);
export const selectedWorkingFileDiff = writable<DiffFile[]>([]);
export const isLoadingWorkingFileDiff = writable(false);

// Directory tree expansion (untracked directories)
export const expandedDirs = writable<ExpandedDirs>({});

// Multi-select checkboxes
export const selectedUnstagedPaths = writable<string[]>([]);
export const selectedStagedPaths = writable<string[]>([]);

// Commit form
export const commitMessage = writable('');
export const isCommitting = writable(false);
export const commitFeedback = writable('');
export const commitFeedbackError = writable('');
