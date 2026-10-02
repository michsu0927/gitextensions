// Shapes returned by the Rust backend (see src-tauri/git-core/src/models.rs).

export interface CommitItem {
  graph: string;
  hash: string;
  author: string;
  email: string;
  date: number;
  subject: string;
  refs: string[];
}

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

export type TabName = 'dashboard' | 'history' | 'branches' | 'settings';

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
