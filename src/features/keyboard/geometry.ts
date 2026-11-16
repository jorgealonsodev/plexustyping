// Physical ISO keyboard: rows of `KeyboardEvent.code` with widths in key units, and the
// standard touch-typing finger for each key. Characters come from the active layout.

export type Finger =
  | "l-pinky"
  | "l-ring"
  | "l-middle"
  | "l-index"
  | "r-index"
  | "r-middle"
  | "r-ring"
  | "r-pinky"
  | "thumb";

export interface KeySpec {
  code: string;
  /** Width in key units; every row adds up to 15. */
  width: number;
}

const keys = (codes: string[]): KeySpec[] => codes.map((code) => ({ code, width: 1 }));
const digits = keys(["Backquote", ..."1234567890".split("").map((d) => `Digit${d}`), "Minus", "Equal"]);
const letters = (row: string) => row.split("").map((c) => `Key${c}`);

export const ISO_ROWS: KeySpec[][] = [
  [...digits, { code: "Backspace", width: 2 }],
  [{ code: "Tab", width: 1.5 }, ...keys([...letters("QWERTYUIOP"), "BracketLeft", "BracketRight"]), { code: "Enter", width: 1.5 }],
  // The ISO Enter spans two rows: its lower part closes the home row.
  [{ code: "CapsLock", width: 1.75 }, ...keys([...letters("ASDFGHJKL"), "Semicolon", "Quote", "Backslash"]), { code: "Enter", width: 1.25 }],
  [
    { code: "ShiftLeft", width: 1.25 },
    ...keys(["IntlBackslash", ...letters("ZXCVBNM"), "Comma", "Period", "Slash"]),
    { code: "ShiftRight", width: 2.75 },
  ],
  [
    { code: "ControlLeft", width: 1.5 },
    { code: "MetaLeft", width: 1.25 },
    { code: "AltLeft", width: 1.5 },
    { code: "Space", width: 6.75 },
    { code: "AltRight", width: 1.5 },
    { code: "ControlRight", width: 2.5 },
  ],
];

const FINGER_KEYS: Record<Finger, string[]> = {
  "l-pinky": ["Backquote", "Digit1", "Tab", "KeyQ", "CapsLock", "KeyA", "ShiftLeft", "IntlBackslash", "KeyZ", "ControlLeft", "MetaLeft"],
  "l-ring": ["Digit2", "KeyW", "KeyS", "KeyX"],
  "l-middle": ["Digit3", "KeyE", "KeyD", "KeyC"],
  "l-index": ["Digit4", "Digit5", "KeyR", "KeyT", "KeyF", "KeyG", "KeyV", "KeyB"],
  "r-index": ["Digit6", "Digit7", "KeyY", "KeyU", "KeyH", "KeyJ", "KeyN", "KeyM"],
  "r-middle": ["Digit8", "KeyI", "KeyK", "Comma"],
  "r-ring": ["Digit9", "KeyO", "KeyL", "Period"],
  "r-pinky": [
    "Digit0",
    "Minus",
    "Equal",
    "Backspace",
    "KeyP",
    "BracketLeft",
    "BracketRight",
    "Enter",
    "Semicolon",
    "Quote",
    "Backslash",
    "Slash",
    "ShiftRight",
    "ControlRight",
  ],
  thumb: ["AltLeft", "Space", "AltRight"],
};

const FINGERS = new Map(
  (Object.entries(FINGER_KEYS) as [Finger, string[]][]).flatMap(([finger, codes]) => codes.map((code) => [code, finger] as const)),
);

/** The finger that presses `code`, or `null` for keys outside the drawn keyboard. */
export const fingerOf = (code: string): Finger | null => FINGERS.get(code) ?? null;

export const handOf = (finger: Finger): "left" | "right" | null =>
  finger.startsWith("l-") ? "left" : finger.startsWith("r-") ? "right" : null;

/** Labels of keys that type no character. */
export const NAMED_LABELS: Record<string, string> = {
  Backspace: "⌫",
  Tab: "Tab",
  Enter: "Intro",
  CapsLock: "Bloq Mayús",
  ShiftLeft: "Mayús",
  ShiftRight: "Mayús",
  ControlLeft: "Ctrl",
  ControlRight: "Ctrl",
  MetaLeft: "Super",
  AltLeft: "Alt",
  AltRight: "Alt Gr",
};
