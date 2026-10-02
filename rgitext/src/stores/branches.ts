import { writable } from 'svelte/store';

// Local branches
export const branches = writable<string[]>([]);
export const isLoadingBranches = writable(false);
export const branchesError = writable('');
export const selectedBranch = writable('');
export const branchMessage = writable('');
export const branchActionError = writable('');
export const isExecutingBranchAction = writable(false);

// Sidebar (workspace inspector)
export const isBranchesCollapsed = writable(false);
export const branchSearchQuery = writable('');
export const selectedSidebarBranch = writable('');
export const sidebarBranchMessage = writable('');
export const sidebarBranchError = writable('');

// Tags
export const tags = writable<string[]>([]);
export const isLoadingTags = writable(false);
export const tagsError = writable('');
export const selectedSidebarTag = writable('');

// Stashes
export const stashes = writable<string[]>([]);
export const isLoadingStashes = writable(false);
export const stashesError = writable('');
// The sidebar component decides the shape of the selected stash (index or entry).
export const selectedSidebarStash = writable<unknown>(null);

// Remotes
export const remotes = writable<string[]>([]);
export const isLoadingRemotes = writable(false);
export const remotesError = writable('');

// Remote branches
export const remoteBranches = writable<string[]>([]);
export const isLoadingRemoteBranches = writable(false);
export const remoteBranchesError = writable('');

// Sidebar accordion
export const isSidebarBranchesExpanded = writable(true);
export const isSidebarTagsExpanded = writable(false);
export const isSidebarStashesExpanded = writable(false);
export const isSidebarRemotesExpanded = writable(false);
