/** Short display date for a unix timestamp (seconds). */
export function formatDate(timestamp: number): string {
  if (!timestamp) return '';
  return new Date(timestamp * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** `2 hours ago` style date for dense lists; falls back to the date for old commits. */
export function formatRelative(timestamp: number): string {
  if (!timestamp) return '';
  const seconds = Math.floor(Date.now() / 1000) - timestamp;
  if (seconds < 0) return formatDate(timestamp);
  const units: [number, string][] = [
    [60, 'second'],
    [60, 'minute'],
    [24, 'hour'],
    [30, 'day'],
    [12, 'month'],
  ];
  let value = seconds;
  for (const [size, name] of units) {
    if (value < size) return value <= 1 && name === 'second' ? 'just now' : `${value} ${name}${value === 1 ? '' : 's'} ago`;
    value = Math.floor(value / size);
  }
  return formatDate(timestamp);
}

export function shortHash(hash: string): string {
  return hash.substring(0, 7);
}

export function formatSize(bytes: number | null): string {
  if (bytes === null) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
