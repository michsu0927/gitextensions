import { writable } from 'svelte/store';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getRawGitCommand } from '../lib/gitCommandLabels';
import type { ConsoleLogEntry } from '../lib/types';

// Keep the history bounded so a long session does not grow without limit.
const MAX_CONSOLE_LOGS = 200;

export const gitConsoleLogs = writable<ConsoleLogEntry[]>([]);
export const showConsoleLogDrawer = writable(false);
export const isConsoleLogDrawerExpanded = writable(false);

export function beginLog(command: string, args: Record<string, unknown>): string {
  const id = Math.random().toString(36).substring(2, 9);
  const timestamp = new Date().toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
  const entry: ConsoleLogEntry = {
    id,
    timestamp,
    command,
    args,
    gitCommands: getRawGitCommand(command, args),
    status: 'pending',
    result: null,
    error: null,
  };
  gitConsoleLogs.update((logs) => [entry, ...logs].slice(0, MAX_CONSOLE_LOGS));
  return id;
}

export function finishLog(id: string, outcome: { result: unknown } | { error: string }): void {
  gitConsoleLogs.update((logs) =>
    logs.map((log) => {
      if (log.id !== id) return log;
      return 'error' in outcome
        ? { ...log, status: 'error', error: outcome.error }
        : { ...log, status: 'success', result: outcome.result };
    }),
  );
}

/** Attaches a real git command line (from the backend) to the newest/pending entries. */
function appendGitCommandLine(commandLine: string): void {
  gitConsoleLogs.update((logs) =>
    logs.map((log, index) =>
      index === 0 || log.status === 'pending'
        ? { ...log, gitCommands: [...log.gitCommands, commandLine] }
        : log,
    ),
  );
}

export function clearConsoleLogs(): void {
  gitConsoleLogs.set([]);
}

/** Subscribes to the backend's `git-command-log` event. Returns the unlisten function. */
export async function startGitLogListener(): Promise<UnlistenFn | undefined> {
  try {
    return await listen<string>('git-command-log', (event) => appendGitCommandLine(event.payload));
  } catch (err) {
    console.error('Failed to establish Tauri git log event listener:', err);
    return undefined;
  }
}
