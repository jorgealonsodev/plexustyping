import { invoke } from "@tauri-apps/api/core";
import type { KeyPress, Layout } from "./types";

/** The built-in layout `layout`, to draw the keyboard. */
export const getLayout = (layout: string) => invoke<Layout>("get_layout", { layout });

/** The key presses for each char of `text` (`null` if untypeable); fetch once per text. */
export const keySequences = (layout: string, text: string) =>
  invoke<(KeyPress[] | null)[]>("key_sequences", { layout, text });
