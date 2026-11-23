import type { Component } from "svelte";
import AccessibilitySection from "./AccessibilitySection.svelte";
import AiSection from "./AiSection.svelte";
import AppearanceSection from "./AppearanceSection.svelte";
import CodeSection from "./CodeSection.svelte";
import KeyboardSection from "./KeyboardSection.svelte";
import MetronomeSection from "./MetronomeSection.svelte";
import PracticeSection from "./PracticeSection.svelte";
import ProfilesSection from "./ProfilesSection.svelte";
import TextSection from "./TextSection.svelte";
import type { SettingsStore } from "./store.svelte";
import { t } from "./messages";

/** One tab of the settings modal: a section component that reads and writes the shared store. */
export interface SettingsSection {
  readonly id: string;
  /** In the interface language. */
  readonly label: string;
  component: Component<{ settings: SettingsStore }>;
}

const section = (
  id: string,
  key: Parameters<typeof t>[0],
  component: Component<{ settings: SettingsStore }>,
): SettingsSection => ({
  id,
  get label() {
    return t(key);
  },
  component,
});

/** Settings modal sections in display order (S8). A new section is one entry here plus its component. */
export const SECTIONS: SettingsSection[] = [
  section("practice", "sectionPractice", PracticeSection),
  section("text", "sectionText", TextSection),
  section("keyboard", "sectionKeyboard", KeyboardSection),
  section("metronome", "sectionMetronome", MetronomeSection),
  section("code", "sectionCode", CodeSection),
  section("appearance", "sectionAppearance", AppearanceSection),
  section("accessibility", "sectionAccessibility", AccessibilitySection),
  section("ai", "sectionAi", AiSection),
  section("profiles", "sectionProfiles", ProfilesSection),
];
