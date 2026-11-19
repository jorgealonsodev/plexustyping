<script lang="ts">
  import { onMount } from "svelte";
  import { focusTyping } from "../practice/focus";
  import type { SettingsStore } from "../settings/store.svelte";
  import type { PracticeKind } from "./api";
  import { LANGUAGES } from "./languages.svelte";

  // Prose / words / code switch and the code language, in the header (S1, S6).
  let { settings }: { settings: SettingsStore } = $props();

  const KINDS: { id: PracticeKind; label: string }[] = [
    { id: "prose", label: "Prosa" },
    { id: "words", label: "Palabras" },
    { id: "code", label: "Código" },
  ];

  onMount(() => void LANGUAGES.load());

  // A stored language that is no longer bundled falls back to the first one.
  $effect(() => {
    const list = LANGUAGES.list;
    if (list.length > 0 && !list.some((l) => l.id === settings.codeLanguage)) settings.setCodeLanguage(list[0].id);
  });

  // Back to the text only after a mouse pick: arrow keys change the select too, and
  // keyboard users keep focus on it (accessibility S2).
  let pointerPick = false;

  function choose(e: Event & { currentTarget: HTMLSelectElement }) {
    settings.setCodeLanguage(e.currentTarget.value);
    if (pointerPick) focusTyping();
  }
</script>

<div class="mode">
  {#if settings.practiceKind === "code"}
    <select
      aria-label="Lenguaje de programación"
      onchange={choose}
      onpointerdown={() => (pointerPick = true)}
      onkeydown={() => (pointerPick = false)}
    >
      {#each LANGUAGES.list as l (l.id)}
        <option value={l.id} selected={l.id === settings.codeLanguage}>{l.name}</option>
      {/each}
    </select>
  {/if}
  <div class="kinds" role="group" aria-label="Modo de práctica">
    {#each KINDS as k (k.id)}
      <button
        type="button"
        aria-pressed={settings.practiceKind === k.id}
        class:active={settings.practiceKind === k.id}
        onclick={() => settings.setPracticeKind(k.id)}>{k.label}</button
      >
    {/each}
  </div>
</div>

<style>
  .mode {
    display: flex;
    align-items: center;
    /* Room for the native select (WebKitGTK may draw it taller than 2rem) in both modes. */
    height: 2.4rem;
    gap: 0.5rem;
    margin-left: auto;
  }
  .kinds {
    display: flex;
  }
  /* One fixed height: showing the language select must not change the header height. */
  .kinds button,
  select {
    box-sizing: border-box;
    height: 2rem;
  }
  .kinds button {
    padding: 0 0.7rem;
    font-size: 0.85rem;
    color: var(--muted);
    border-radius: 0;
  }
  .kinds button:not(:first-child) {
    margin-left: -1px;
  }
  .kinds button:first-child {
    border-radius: 0.375rem 0 0 0.375rem;
  }
  .kinds button:last-child {
    border-radius: 0 0.375rem 0.375rem 0;
  }
  /* Inside the button: an outside ring would overlap the joined neighbours. */
  .kinds button:focus-visible {
    position: relative;
    z-index: 1;
    outline-offset: -3px;
  }
  .kinds button.active:focus-visible {
    outline-color: var(--on-accent);
  }
  .kinds .active {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }
  select {
    max-width: 9rem;
    padding: 0 0.4rem;
    font: inherit;
    font-size: 0.85rem;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
</style>
