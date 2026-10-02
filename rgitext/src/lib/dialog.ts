import { writable } from 'svelte/store';

export type FieldValue = string | boolean;

export interface DialogField {
  name: string;
  label: string;
  kind: 'text' | 'textarea' | 'checkbox' | 'select';
  value: FieldValue;
  placeholder?: string;
  hint?: string;
  required?: boolean;
  options?: { value: string; label: string }[];
}

export interface DialogSpec {
  title: string;
  description?: string;
  fields: DialogField[];
  submitLabel: string;
  /** Style the submit button as a destructive action. */
  danger?: boolean;
  onSubmit: (values: Record<string, FieldValue>) => void | Promise<void>;
}

/** The dialog currently shown (null = none). */
export const dialog = writable<DialogSpec | null>(null);

export function openDialog(spec: DialogSpec): void {
  dialog.set(spec);
}

export function closeDialog(): void {
  dialog.set(null);
}

/** Convenience: a confirmation dialog without fields. */
export function confirmDialog(
  title: string,
  description: string,
  submitLabel: string,
  onConfirm: () => void | Promise<void>,
  danger = false,
): void {
  openDialog({ title, description, fields: [], submitLabel, danger, onSubmit: () => onConfirm() });
}
