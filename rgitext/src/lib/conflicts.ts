// Parsing of git conflict markers so that conflicts can be resolved hunk by hunk.

export type Choice = 'ours' | 'theirs' | 'both';

export interface TextSegment {
  kind: 'text';
  text: string;
}

export interface ConflictSegment {
  kind: 'conflict';
  /** The original lines including the markers, restored when a choice is undone. */
  raw: string;
  ours: string;
  /** Common ancestor (only with `merge.conflictStyle = diff3`). */
  base: string | null;
  theirs: string;
  oursLabel: string;
  theirsLabel: string;
  choice: Choice | null;
}

export type Segment = TextSegment | ConflictSegment;

const OURS = '<<<<<<<';
const BASE = '|||||||';
const SEPARATOR = '=======';
const THEIRS = '>>>>>>>';

/** Splits into lines, keeping their line endings so that rebuilding is lossless. */
function splitLines(text: string): string[] {
  return text.match(/[^\n]*\n|[^\n]+$/g) ?? [];
}

const startsWithMarker = (line: string, marker: string): boolean =>
  line.startsWith(marker) && (line.length === marker.length || /^\s/.test(line.charAt(marker.length)));

const labelOf = (line: string, marker: string): string => line.slice(marker.length).trim();

/** Splits file content into plain text and conflict hunks. An unterminated hunk stays text. */
export function parseConflicts(text: string): Segment[] {
  const lines = splitLines(text);
  const segments: Segment[] = [];
  let plain = '';
  let i = 0;

  const flushPlain = () => {
    if (plain) segments.push({ kind: 'text', text: plain });
    plain = '';
  };

  while (i < lines.length) {
    if (!startsWithMarker(lines[i], OURS)) {
      plain += lines[i++];
      continue;
    }

    // Look for the end of the hunk first, so that a broken one is left untouched.
    let separator = -1;
    let base = -1;
    let end = -1;
    for (let j = i + 1; j < lines.length; j++) {
      const line = lines[j];
      if (startsWithMarker(line, OURS)) break;
      if (startsWithMarker(line, BASE) && separator < 0 && base < 0) base = j;
      else if (line.replace(/\r?\n$/, '') === SEPARATOR && separator < 0) separator = j;
      else if (startsWithMarker(line, THEIRS) && separator >= 0) {
        end = j;
        break;
      }
    }
    if (separator < 0 || end < 0) {
      plain += lines[i++];
      continue;
    }

    flushPlain();
    const oursEnd = base >= 0 ? base : separator;
    segments.push({
      kind: 'conflict',
      raw: lines.slice(i, end + 1).join(''),
      ours: lines.slice(i + 1, oursEnd).join(''),
      base: base >= 0 ? lines.slice(base + 1, separator).join('') : null,
      theirs: lines.slice(separator + 1, end).join(''),
      oursLabel: labelOf(lines[i], OURS),
      theirsLabel: labelOf(lines[end], THEIRS),
      choice: null,
    });
    i = end + 1;
  }
  flushPlain();
  return segments;
}

function resolved(segment: ConflictSegment): string {
  switch (segment.choice) {
    case 'ours':
      return segment.ours;
    case 'theirs':
      return segment.theirs;
    case 'both':
      return segment.ours + segment.theirs;
    default:
      return segment.raw;
  }
}

/** The file content with every decided hunk replaced by its choice. */
export function buildMerged(segments: Segment[]): string {
  return segments.map((s) => (s.kind === 'text' ? s.text : resolved(s))).join('');
}

export function unresolvedCount(segments: Segment[]): number {
  return segments.filter((s) => s.kind === 'conflict' && s.choice === null).length;
}

export function conflictCount(segments: Segment[]): number {
  return segments.filter((s) => s.kind === 'conflict').length;
}
