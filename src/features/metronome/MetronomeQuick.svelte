<script lang="ts">
  import { tick } from "svelte";
  import { nextFocusIndex } from "../practice/focus";
  import type { Sound } from "./api";
  import BeatDot from "./BeatDot.svelte";
  import { type BeatAnchor, MAX_ACCENT_EVERY, MAX_BPM, MIN_BPM, type MetronomeSettings, SOUNDS, stepBpm } from "./beat";

  // The beat dot is also the metronome's quick settings: a click opens a small panel to tune it
  // without leaving the text. Classes only (CSP). Keyboard (accessibility S2): opening moves the
  // focus into the panel, Tab stays inside it while open, and Escape (from the toggle or the panel)
  // closes it and gives the focus back to the toggle.
  let {
    anchor,
    metronome,
    onChange,
  }: {
    anchor: BeatAnchor;
    metronome: MetronomeSettings;
    onChange: (change: Partial<MetronomeSettings>) => void;
  } = $props();

  let open = $state(false);
  let toggle = $state<HTMLButtonElement>();
  let panel = $state<HTMLDivElement>();
  type Change<T extends HTMLElement> = Event & { currentTarget: T };
  const number = (e: Change<HTMLInputElement>) => e.currentTarget.valueAsNumber;

  const controls = () =>
    panel ? [...panel.querySelectorAll<HTMLElement>("button, input, select")].filter((c) => !c.hasAttribute("disabled")) : [];

  async function show() {
    open = true;
    await tick();
    (controls()[0] ?? panel)?.focus();
  }
  function close() {
    open = false;
    toggle?.focus();
  }
  function onToggleKeydown(e: KeyboardEvent) {
    // Escape on the toggle closes an open panel; it never reaches the practice (no restart).
    if (e.key !== "Escape") return;
    e.stopPropagation();
    e.preventDefault();
    if (open) close();
  }
  function onKeydown(e: KeyboardEvent) {
    // Keys typed in the panel never reach the practice (Escape would restart the text).
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      close();
      return;
    }
    if (e.key !== "Tab") return;
    const list = controls();
    const next = nextFocusIndex(list.indexOf(document.activeElement as HTMLElement), list.length, e.shiftKey);
    e.preventDefault();
    if (next !== null) list[next].focus();
  }
</script>

<div class="metro-quick">
  <button
    bind:this={toggle}
    type="button"
    class="toggle"
    aria-expanded={open}
    aria-haspopup="dialog"
    aria-label="Ajustes rápidos del metrónomo: {metronome.bpm} BPM"
    title="Ajustar el metrónomo"
    onclick={() => (open ? close() : void show())}
    onkeydown={onToggleKeydown}
  >
    <BeatDot {anchor} />
    <span class="bpm">{metronome.bpm} BPM</span>
  </button>
  {#if open}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div bind:this={panel} class="panel" role="dialog" aria-label="Metrónomo" tabindex="-1" onkeydown={onKeydown}>
      <div class="row">
        <label class="name" for="mq-bpm">Tempo</label>
        <button type="button" aria-label="Más lento" onclick={() => onChange({ bpm: stepBpm(metronome.bpm, -1) })}>−</button>
        <input
          id="mq-bpm"
          type="range"
          min={MIN_BPM}
          max={MAX_BPM}
          value={metronome.bpm}
          aria-label="Tempo"
          oninput={(e: Change<HTMLInputElement>) => onChange({ bpm: number(e) })}
        />
        <button type="button" aria-label="Más rápido" onclick={() => onChange({ bpm: stepBpm(metronome.bpm, 1) })}>+</button>
      </div>
      <div class="row">
        <label class="name" for="mq-sound">Sonido</label>
        <select id="mq-sound" onchange={(e: Change<HTMLSelectElement>) => onChange({ sound: e.currentTarget.value as Sound })}>
          {#each SOUNDS as s (s.id)}
            <option value={s.id} selected={s.id === metronome.sound}>{s.label}</option>
          {/each}
        </select>
      </div>
      <div class="row">
        <label class="name" for="mq-volume">Volumen</label>
        <input
          id="mq-volume"
          type="range"
          min="0"
          max="100"
          value={metronome.volume}
          oninput={(e: Change<HTMLInputElement>) => onChange({ volume: number(e) })}
        />
        <span class="value">{metronome.volume} %</span>
      </div>
      <div class="row">
        <label class="name" for="mq-accent">Acento cada</label>
        <input
          id="mq-accent"
          type="number"
          min="0"
          max={MAX_ACCENT_EVERY}
          value={metronome.accentEvery}
          onchange={(e: Change<HTMLInputElement>) => onChange({ accentEvery: number(e) })}
        />
      </div>
      <div class="row check">
        <input
          id="mq-adaptive"
          type="checkbox"
          checked={metronome.adaptive}
          onchange={(e: Change<HTMLInputElement>) => onChange({ adaptive: e.currentTarget.checked })}
        />
        <label for="mq-adaptive">Tempo adaptativo</label>
      </div>
      <div class="row end">
        <button type="button" onclick={() => onChange({ enabled: false })}>Apagar <kbd>F4</kbd></button>
        <button type="button" onclick={close}>Cerrar</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .metro-quick {
    position: relative;
    flex: none;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.2rem 0.55rem;
    font: inherit;
    font-size: 0.85rem;
    color: var(--muted);
    background: none;
    border: 1px solid transparent;
    border-radius: 999px;
  }
  .toggle:hover,
  .toggle[aria-expanded="true"] {
    color: var(--text);
    border-color: var(--border);
    background: var(--surface-2);
  }
  .bpm {
    font-variant-numeric: tabular-nums;
  }
  .panel {
    position: absolute;
    top: calc(100% + 0.4rem);
    left: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    width: 19rem;
    padding: 0.75rem 0.85rem;
    font-size: 0.85rem;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 0.6rem;
    box-shadow: 0 0.5rem 1.5rem color-mix(in srgb, black 35%, transparent);
  }
  .row {
    position: static;
    display: flex;
    min-height: 1.9rem;
    align-items: center;
    gap: 0.5rem;
  }
  .row.end {
    justify-content: flex-end;
  }
  .name {
    flex: none;
    width: 5.5rem;
    color: var(--muted);
  }
  .value {
    width: 3rem;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  input[type="range"] {
    flex: 1;
    min-width: 6rem;
    accent-color: var(--accent);
  }
  input[type="number"],
  select,
  .panel button {
    padding: 0.25rem 0.45rem;
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
  input[type="number"] {
    width: 4rem;
  }
  .panel button:hover {
    border-color: var(--accent);
  }
  .check {
    gap: 0.45rem;
  }
</style>
