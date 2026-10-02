// Answers credential prompts of git and ssh (passwords, passphrases, host key questions).
// The backend emits an `askpass-request` for every prompt; the answer goes back with
// `submit_askpass`. Prompts are shown one at a time.

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { api } from './api';
import { openDialog } from './dialog';

interface PromptRequest {
  id: string;
  prompt: string;
}

const queue: PromptRequest[] = [];
let showing = false;

function looksSecret(prompt: string): boolean {
  return /password|passphrase|token|secret|pin\b/i.test(prompt) && !/username/i.test(prompt);
}

function answer(id: string, secret: string | null): void {
  void api.submitAskpass(id, secret).catch((e) => console.warn('Could not deliver the answer:', e));
  showing = false;
  showNext();
}

function showNext(): void {
  if (showing) return;
  const next = queue.shift();
  if (!next) return;
  showing = true;
  const secret = looksSecret(next.prompt);
  openDialog({
    title: 'Authentication required',
    description: next.prompt.trim(),
    fields: [
      {
        name: 'answer',
        label: secret ? 'Secret' : 'Answer',
        kind: secret ? 'password' : 'text',
        value: '',
      },
    ],
    submitLabel: 'OK',
    onSubmit: (v) => answer(next.id, String(v.answer)),
    onCancel: () => answer(next.id, null),
  });
}

/** Subscribes to credential prompts. Returns the unlisten function. */
export async function startAskpassListener(): Promise<UnlistenFn | undefined> {
  try {
    return await listen<PromptRequest>('askpass-request', (event) => {
      queue.push(event.payload);
      showNext();
    });
  } catch (err) {
    console.error('Failed to listen for credential prompts:', err);
    return undefined;
  }
}
