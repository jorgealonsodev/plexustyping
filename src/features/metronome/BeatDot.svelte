<script lang="ts">
  import { type BeatAnchor, beatAt, isAccent } from "./beat";

  // Flashes on each beat (S7), driven by requestAnimationFrame from beat 0; classes only (CSP).
  let { anchor }: { anchor: BeatAnchor } = $props();

  let on = $state(false);
  let accent = $state(false);

  $effect(() => {
    const { epochMs, intervalMs, accentEvery } = anchor;
    const flashMs = Math.min(120, intervalMs / 2);
    let frame = 0;
    const tick = () => {
      const beat = beatAt(performance.timeOrigin + performance.now(), epochMs, intervalMs);
      const lit = beat !== null && beat.sinceMs < flashMs;
      if (lit !== on) on = lit;
      const strong = lit && isAccent(beat.index, accentEvery);
      if (strong !== accent) accent = strong;
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
</script>

<span class="dot" class:on class:accent role="img" aria-label="Metrónomo: {anchor.bpm} BPM" title="{anchor.bpm} BPM"></span>

<style>
  .dot {
    flex: none;
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 50%;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .on.accent {
    background: var(--warning);
    border-color: var(--warning);
    box-shadow: 0 0 0 0.2rem color-mix(in srgb, var(--warning) 35%, transparent);
  }
</style>
