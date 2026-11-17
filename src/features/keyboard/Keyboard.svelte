<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { NAMED_LABELS, fingerOf, rowsFor } from "./geometry";
  import { heatLevel } from "./heat";
  import { type Flash, type Highlight, keyLabels } from "./keys";
  import type { Layout } from "./types";

  let {
    layout,
    highlights,
    flash,
    heat = null,
  }: {
    layout: Layout | null;
    highlights: Highlight[];
    flash: Flash | null;
    /** Error score per key code (`heatScores`); `null` hides the heatmap. */
    heat?: Map<string, number> | null;
  } = $props();

  const FLASH_MS = 150;

  const rows = $derived(
    rowsFor(layout).map((row) =>
      row.map((spec) => ({
        ...spec,
        // Classes, not style attributes: widths are a fixed set of key units.
        widthClass: `w${spec.width * 100}`,
        finger: fingerOf(spec.code),
        labels: keyLabels(layout?.keys[spec.code], layout?.deadKeys ?? []),
        name: spec.unlabelled ? null : (NAMED_LABELS[spec.code] ?? null),
      })),
    ),
  );
  const roles = $derived(new Map(highlights.map((h) => [h.code, h.role])));

  let flashed = $state<Record<string, "ok" | "bad">>({});
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  $effect(() => {
    if (!flash) return;
    const { code, ok } = flash;
    untrack(() => {
      clearTimeout(timers.get(code));
      flashed[code] = ok ? "ok" : "bad";
      timers.set(
        code,
        setTimeout(() => {
          delete flashed[code];
          timers.delete(code);
        }, FLASH_MS),
      );
    });
  });

  onDestroy(() => timers.forEach((timer) => clearTimeout(timer)));
</script>

<!-- Decorative: the practice text already says what to type. -->
<div class="keyboard" aria-hidden="true">
  {#each rows as row, r (r)}
    <div class="row">
      {#each row as key, i (i)}
        {@const role = roles.get(key.code)}
        {@const level = heat ? heatLevel(heat.get(key.code) ?? 0) : 0}
        <div
          class="key {key.widthClass} {role && key.finger ? `finger-${key.finger}` : ''} {level ? `heat heat-${level}` : ''}"
          class:primary={role === "primary"}
          class:modifier={role === "modifier"}
          class:secondary={role === "secondary"}
          class:flash-ok={flashed[key.code] === "ok"}
          class:flash-bad={flashed[key.code] === "bad"}
          class:home={key.code === "KeyF" || key.code === "KeyJ"}
        >
          {#if key.labels.shift}<span class="shift" class:dead={key.labels.shift.dead}>{key.labels.shift.text}</span>{/if}
          {#if key.labels.main}
            <span class="main" class:dead={key.labels.main.dead}>{key.labels.main.text}</span>
          {:else if key.name}
            <span class="name">{key.name}</span>
          {/if}
          {#if key.labels.altgr}<span class="altgr" class:dead={key.labels.altgr.dead}>{key.labels.altgr.text}</span>{/if}
        </div>
      {/each}
    </div>
  {/each}
</div>
{#if heat}
  <p class="legend">menos ← <span class="scale" aria-hidden="true"></span> → más errores</p>
{/if}

<style>
  .keyboard {
    /* One key unit: shrinks with the window so the keyboard fits down to 720×480. */
    --u: min(4.25rem, calc((100vw - 3rem) / 15), 8vh);
    --gap: calc(var(--u) * 0.08);
    display: flex;
    flex-direction: column;
    align-items: center;
    font-family: var(--mono);
    font-size: calc(var(--u) * 0.4);
    line-height: 1;
    user-select: none;
  }
  .row {
    display: flex;
    width: calc(var(--u) * 15);
  }
  .key {
    position: relative;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    height: calc(var(--u) - var(--gap));
    margin: calc(var(--gap) / 2);
    color: var(--key-fg);
    background: var(--key-bg);
    border: 1px solid var(--key-border);
    border-radius: calc(var(--u) * 0.12);
  }
  .w100 { width: calc(var(--u) * 1 - var(--gap)); }
  .w125 { width: calc(var(--u) * 1.25 - var(--gap)); }
  .w150 { width: calc(var(--u) * 1.5 - var(--gap)); }
  .w175 { width: calc(var(--u) * 1.75 - var(--gap)); }
  .w200 { width: calc(var(--u) * 2 - var(--gap)); }
  .w225 { width: calc(var(--u) * 2.25 - var(--gap)); }
  .w250 { width: calc(var(--u) * 2.5 - var(--gap)); }
  .w275 { width: calc(var(--u) * 2.75 - var(--gap)); }
  .w675 { width: calc(var(--u) * 6.75 - var(--gap)); }
  .shift,
  .altgr {
    position: absolute;
    font-size: 0.7em;
    opacity: 0.75;
  }
  .shift {
    top: 0.2em;
    left: 0.3em;
  }
  .altgr {
    right: 0.3em;
    bottom: 0.2em;
  }
  .name {
    font-family: system-ui, sans-serif;
    font-size: 0.72em;
    opacity: 0.75;
    white-space: nowrap;
  }
  /* Dead keys compose with the next key: dotted underline. */
  .dead {
    text-decoration: underline dotted;
    text-underline-offset: 0.15em;
  }
  .home .main {
    text-decoration: underline;
    text-underline-offset: 0.2em;
  }
  /* Heatmap tint: the key background mixed toward --danger by score, under any highlight. */
  .heat { background: color-mix(in srgb, var(--danger) var(--heat), var(--key-bg)); }
  .heat-1 { --heat: 6%; }
  .heat-2 { --heat: 12%; }
  .heat-3 { --heat: 18%; }
  .heat-4 { --heat: 24%; }
  .heat-5 { --heat: 30%; }
  .heat-6 { --heat: 36%; }
  .heat-7 { --heat: 42%; }
  .heat-8 { --heat: 48%; }
  .heat-9 { --heat: 54%; }
  .heat-10 { --heat: 60%; }
  .legend {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    margin: 0.15rem 0 0;
    font-size: 0.75rem;
    line-height: 1;
    color: var(--muted);
  }
  .scale {
    width: 8rem;
    height: 0.5rem;
    background: linear-gradient(to right, var(--key-bg), color-mix(in srgb, var(--danger) 60%, var(--key-bg)));
    border: 1px solid var(--key-border);
    border-radius: 0.25rem;
  }
  .finger-l-pinky { --finger: var(--finger-l-pinky); }
  .finger-l-ring { --finger: var(--finger-l-ring); }
  .finger-l-middle { --finger: var(--finger-l-middle); }
  .finger-l-index { --finger: var(--finger-l-index); }
  .finger-r-index { --finger: var(--finger-r-index); }
  .finger-r-middle { --finger: var(--finger-r-middle); }
  .finger-r-ring { --finger: var(--finger-r-ring); }
  .finger-r-pinky { --finger: var(--finger-r-pinky); }
  .finger-thumb { --finger: var(--finger-thumb); }
  .primary,
  .modifier {
    color: var(--bg);
    background: var(--finger);
    border-color: var(--finger);
    font-weight: 700;
  }
  .modifier {
    opacity: 0.85;
  }
  .secondary {
    color: var(--finger);
    border: 2px dashed var(--finger);
  }
  /* After the highlight rules, so a flash wins on the same key. */
  .flash-ok,
  .flash-bad {
    color: var(--bg);
    opacity: 1;
  }
  .flash-ok {
    background: var(--success);
    border-color: var(--success);
  }
  .flash-bad {
    background: var(--danger);
    border-color: var(--danger);
  }
</style>
