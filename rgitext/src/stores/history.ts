import { writable } from 'svelte/store';
import type {
  CommitDetails, CommitFile, DiffFile, DiffOptions, GraphState, RevisionFilter, RevisionRow,
} from '../lib/types';

export const DEFAULT_FILTER: RevisionFilter = {
  scope: 'current',
  search: '',
  author: '',
  path: '',
  follow: false,
  firstParent: false,
};

// Revision list (paged, with graph)
export const revisionRows = writable<RevisionRow[]>([]);
export const graphState = writable<GraphState | null>(null);
export const hasMoreRevisions = writable(false);
export const isLoadingCommits = writable(false);
export const isLoadingMoreCommits = writable(false);
export const commitsError = writable('');
export const revisionFilter = writable<RevisionFilter>({ ...DEFAULT_FILTER });

// Selected commit
export const selectedHash = writable<string | null>(null);
export const commitDetails = writable<CommitDetails | null>(null);
export const isLoadingCommitDetails = writable(false);

// Files and diff of the selected commit
export const commitFiles = writable<CommitFile[]>([]);
export const isLoadingCommitFiles = writable(false);
export const selectedCommitFile = writable('');
export const commitDiff = writable<DiffFile[]>([]);
export const isLoadingCommitFileDiff = writable(false);
export const commitFilesError = writable('');

// Diff display options (shared by the history and working directory views)
export const diffOptions = writable<DiffOptions>({ contextLines: 3, ignoreWhitespace: false });
export const diffMode = writable<'inline' | 'split'>('inline');
