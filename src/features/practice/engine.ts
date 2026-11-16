// Live-display mirror of `plexustyping-core::replay`; Rust stays authoritative for results (L4).
// Both are tested against crates/plexustyping-core/tests/fixtures/replay.json.

export type ErrorMode = "stop-on-error" | "allow-continue" | "backspace-correct";

export type InputEvent = { kind: "char"; ch: string; tMs: number } | { kind: "backspace"; tMs: number };

export interface KeystrokeRecord {
  index: number;
  expected: string;
  typed: string;
  correct: boolean;
  msSincePrevious: number | null;
}

export interface Stats {
  durationMs: number;
  keystrokes: number;
  charKeystrokes: number;
  correctKeystrokes: number;
  errors: number;
  uncorrectedErrors: number;
  charsTyped: number;
  accuracy: number;
  grossWpm: number;
  netWpm: number;
  kpm: number;
}

export interface SessionResult {
  records: KeystrokeRecord[];
  stats: Stats;
  completed: boolean;
  cursor: number;
}

export type CharState = "pending" | "correct" | "error";

/** Incremental replay of one text: applies events as they are typed. */
export class TypingSession {
  readonly chars: string[];
  /** Every event received, applied or not, for `score_session`. */
  readonly events: InputEvent[] = [];
  cursor = 0;
  /** Timestamp of the first applied event (the clock start). */
  firstT: number | null = null;
  private lastT: number | null = null;
  private keystrokes = 0;
  private readonly records: KeystrokeRecord[] = [];
  private readonly typedAt: (string | null)[];
  /** stop-on-error: the last key at the cursor was wrong. */
  private blocked = false;

  constructor(
    text: string,
    readonly mode: ErrorMode,
  ) {
    this.chars = [...text];
    this.typedAt = this.chars.map(() => null);
  }

  get completed(): boolean {
    return this.cursor === this.chars.length;
  }

  /** Applies one event; returns whether it changed the session. */
  apply(event: InputEvent): boolean {
    this.events.push(event);
    if (this.completed) return false;
    if (event.kind === "char") {
      const expected = this.chars[this.cursor];
      const correct = event.ch === expected;
      this.records.push({
        index: this.cursor,
        expected,
        typed: event.ch,
        correct,
        msSincePrevious: this.lastT === null ? null : event.tMs - this.lastT,
      });
      if (correct || this.mode !== "stop-on-error") {
        this.typedAt[this.cursor] = event.ch;
        this.cursor += 1;
        this.blocked = false;
      } else {
        this.blocked = true;
      }
    } else {
      if (this.mode !== "backspace-correct" || this.cursor === 0) return false;
      this.cursor -= 1;
    }
    this.keystrokes += 1;
    this.firstT ??= event.tMs;
    this.lastT = event.tMs;
    return true;
  }

  state(index: number): CharState {
    if (index < this.cursor) return this.typedAt[index] === this.chars[index] ? "correct" : "error";
    return index === this.cursor && this.blocked ? "error" : "pending";
  }

  result(): SessionResult {
    const records = this.records.map((r) => ({ ...r }));
    let uncorrectedErrors = 0;
    for (let i = 0; i < this.cursor; i++) if (this.typedAt[i] !== this.chars[i]) uncorrectedErrors++;
    const charKeystrokes = records.length;
    const correctKeystrokes = records.filter((r) => r.correct).length;
    const durationMs = this.firstT === null || this.lastT === null ? 0 : this.lastT - this.firstT;
    const accuracy = charKeystrokes === 0 ? 1 : correctKeystrokes / charKeystrokes;
    let grossWpm = 0;
    let netWpm = 0;
    let kpm = 0;
    if (durationMs !== 0) {
      const minutes = durationMs / 60_000;
      const words = this.cursor / 5;
      grossWpm = words / minutes;
      netWpm = Math.max((words - uncorrectedErrors) / minutes, 0);
      kpm = this.keystrokes / minutes;
    }
    return {
      records,
      completed: this.completed,
      cursor: this.cursor,
      stats: {
        durationMs,
        keystrokes: this.keystrokes,
        charKeystrokes,
        correctKeystrokes,
        errors: charKeystrokes - correctKeystrokes,
        uncorrectedErrors,
        charsTyped: this.cursor,
        accuracy,
        grossWpm,
        netWpm,
        kpm,
      },
    };
  }
}

export function replay(text: string, mode: ErrorMode, events: InputEvent[]): SessionResult {
  const session = new TypingSession(text, mode);
  for (const event of events) session.apply(event);
  return session.result();
}

export type KeyLike = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "isComposing" | "getModifierState">;

/**
 * Maps a keydown to an input event, or `null` when it is not typing: dead keys (the composed
 * char arrives later as one key), modifiers, named keys, IME composition and Ctrl/Meta shortcuts.
 * AltGr is accepted even though Linux and Windows report it as Ctrl+Alt.
 */
export function keyToEvent(e: KeyLike, tMs: number): InputEvent | null {
  if (e.isComposing || e.key === "Dead" || e.metaKey) return null;
  if (e.ctrlKey && !e.getModifierState("AltGraph")) return null;
  if (e.key === "Backspace") return { kind: "backspace", tMs };
  return [...e.key].length === 1 ? { kind: "char", ch: e.key, tMs } : null;
}
