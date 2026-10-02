// Shapes returned by the Rust backend (see src-tauri/git-core/src/models.rs).

export interface GitStatus {
  branch: string;
  staged_count: number;
  unstaged_count: number;
  untracked_count: number;
  raw_status: string;
}

export interface CommitFile {
  status: string;
  path: string;
  old_path: string | null;
}

export interface WorkingFile {
  path: string;
  status: string;
  is_staged: boolean;
  is_dir: boolean;
}

export interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

export interface SelectedWorkingFile {
  path: string;
  is_staged: boolean;
}

/** `'loading'` while a directory listing is being fetched. */
export type ExpandedDirs = Record<string, DirEntry[] | 'loading'>;

export type TabName = 'dashboard' | 'history' | 'commit' | 'branches' | 'settings';

export interface ConsoleLogEntry {
  id: string;
  timestamp: string;
  command: string;
  args: Record<string, unknown>;
  gitCommands: string[];
  status: 'pending' | 'success' | 'error';
  result: unknown;
  error: string | null;
}

// ---------------------------------------------------------------------------
// Revisions and graph (see src-tauri/git-core/src/revisions.rs and graph.rs)
// ---------------------------------------------------------------------------

export type RefKind = 'head' | 'remote' | 'tag' | 'stash' | 'other';

export interface RefInfo {
  name: string;
  kind: RefKind;
  is_head: boolean;
}

export interface GraphLine {
  kind: 'pass' | 'in' | 'out';
  from: number;
  to: number;
  color: number;
}

export interface GraphRow {
  node_lane: number;
  color: number;
  lines: GraphLine[];
  width: number;
}

export interface GraphState {
  lanes: ({ hash: string; color: number } | null)[];
  next_color: number;
}

export interface Revision {
  hash: string;
  parents: string[];
  author: string;
  email: string;
  author_date: number;
  commit_date: number;
  subject: string;
  refs: RefInfo[];
}

export interface RevisionRow extends Revision {
  graph: GraphRow;
}

export interface RevisionPage {
  rows: RevisionRow[];
  graph_state: GraphState;
  has_more: boolean;
}

/** Filters of the history view (camelCase: sent to the backend as is). */
export interface RevisionFilter {
  scope: string;
  search: string;
  author: string;
  path: string;
  follow: boolean;
  firstParent: boolean;
}

export interface CommitDetails {
  hash: string;
  parents: string[];
  author: string;
  email: string;
  author_date: number;
  committer: string;
  committer_email: string;
  commit_date: number;
  refs: RefInfo[];
  message: string;
}

// ---------------------------------------------------------------------------
// Diff, blame, tree (see diff.rs, blame.rs, tree.rs)
// ---------------------------------------------------------------------------

export interface DiffLine {
  kind: 'context' | 'add' | 'del' | 'noeol';
  old_no: number | null;
  new_no: number | null;
  text: string;
}

export interface DiffHunk {
  header: string;
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: DiffLine[];
}

export interface DiffFile {
  old_path: string | null;
  new_path: string | null;
  change: 'modified' | 'added' | 'deleted' | 'renamed' | 'copied';
  is_binary: boolean;
  additions: number;
  deletions: number;
  hunks: DiffHunk[];
}

export interface DiffOptions {
  contextLines: number;
  ignoreWhitespace: boolean;
}

export interface BlameLine {
  line_no: number;
  hash: string;
  orig_line_no: number;
  author: string;
  email: string;
  author_date: number;
  summary: string;
  file_name: string;
  is_boundary: boolean;
  text: string;
}

export interface TreeEntry {
  name: string;
  path: string;
  kind: 'blob' | 'tree' | 'commit';
  mode: string;
  hash: string;
  size: number | null;
}

export interface FileContent {
  is_binary: boolean;
  truncated: boolean;
  size: number;
  text: string;
}

// ---------------------------------------------------------------------------
// Commit workflow (Phase 2)
// ---------------------------------------------------------------------------

export interface StashEntry {
  index: number;
  name: string;
  message: string;
  timestamp: number;
  hash: string;
}

/** A hunk of the displayed diff, either completely (`lines` omitted) or selected lines only. */
export interface HunkSelection {
  hunk: number;
  lines?: number[];
}

export interface CommitOptions {
  amend: boolean;
  sign: boolean;
  resetAuthor: boolean;
  noVerify: boolean;
}

export interface DiscardTarget {
  path: string;
  untracked: boolean;
}

export type SelectionAction = 'stage' | 'unstage' | 'discard';
