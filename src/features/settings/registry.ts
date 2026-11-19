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

/** One tab of the settings modal: a section component that reads and writes the shared store. */
export interface SettingsSection {
  id: string;
  label: string;
  component: Component<{ settings: SettingsStore }>;
}

/** Settings modal sections in display order (S8). A new section is one entry here plus its component. */
export const SECTIONS: SettingsSection[] = [
  { id: "practice", label: "Práctica", component: PracticeSection },
  { id: "text", label: "Texto", component: TextSection },
  { id: "keyboard", label: "Teclado", component: KeyboardSection },
  { id: "metronome", label: "Metrónomo", component: MetronomeSection },
  { id: "code", label: "Código", component: CodeSection },
  { id: "appearance", label: "Apariencia", component: AppearanceSection },
  { id: "accessibility", label: "Accesibilidad", component: AccessibilitySection },
  { id: "ai", label: "IA", component: AiSection },
  { id: "profiles", label: "Perfiles", component: ProfilesSection },
];
