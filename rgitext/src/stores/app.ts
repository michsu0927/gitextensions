import { writable } from 'svelte/store';
import type { TabName } from '../lib/types';

const GIT_PATH_KEY = 'custom_git_path';

function loadSavedGitPath(): string {
  try {
    return localStorage.getItem(GIT_PATH_KEY) || '';
  } catch {
    return '';
  }
}

export function saveGitPath(path: string): void {
  try {
    localStorage.setItem(GIT_PATH_KEY, path);
  } catch (err) {
    console.warn('Could not persist git path:', err);
  }
}

export const activeTab = writable<TabName>('dashboard');

// Git executable
export const gitVersion = writable('Checking...');
export const gitError = writable('');
export const isChecking = writable(false);

// Settings tab
export const customGitPath = writable(loadSavedGitPath());
export const tempGitPath = writable(loadSavedGitPath());
export const isSavingSettings = writable(false);
export const settingsMessage = writable('');
export const settingsError = writable('');
