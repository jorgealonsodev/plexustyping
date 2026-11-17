<script lang="ts">
  import type { CharState } from "../practice/engine";
  import { type TokenClass, newlineMark } from "./syntax";

  // A code text (S3, S4): newlines and indentation kept, each char coloured by its syntax class.
  let {
    chars,
    states,
    cursor,
    classes,
  }: { chars: string[]; states: CharState[]; cursor: number; classes: TokenClass[] } = $props();

  let block = $state<HTMLPreElement>();

  // Keeps the cursor line in view as the text scrolls inside its block.
  $effect(() => {
    void cursor;
    block?.querySelector(".cursor")?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
</script>

<!-- No whitespace inside: the block keeps every space and newline. -->
<pre bind:this={block} class="code" aria-label="Código de práctica">{#each chars as c, i (i)}<span
      class="ch {states[i]} syn-{classes[i]}"
      class:space={c === " "}
      class:nl={c === "\n"}
      class:cursor={i === cursor}>{newlineMark(c, states[i], i === cursor)}{c}</span
    >{/each}</pre>

<style>
  .code {
    margin: 0;
    padding: 0.75rem 1rem;
    max-height: max(8rem, 42vh);
    overflow: auto;
    font-family: var(--mono);
    font-size: clamp(0.9rem, min(1.8vw, 3vh), 1.5rem);
    line-height: 1.6;
    white-space: pre;
    tab-size: 4;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
  .ch {
    border-radius: 0.15em;
  }
  .syn-plain { --syn: var(--syntax-plain); }
  .syn-keyword { --syn: var(--syntax-keyword); }
  .syn-string { --syn: var(--syntax-string); }
  .syn-number { --syn: var(--syntax-number); }
  .syn-comment { --syn: var(--syntax-comment); }
  .syn-function { --syn: var(--syntax-function); }
  .syn-type { --syn: var(--syntax-type); }
  .syn-variable { --syn: var(--syntax-variable); }
  .syn-constant { --syn: var(--syntax-constant); }
  .syn-operator { --syn: var(--syntax-operator); }
  .syn-punctuation { --syn: var(--syntax-punctuation); }
  .syn-tag { --syn: var(--syntax-tag); }
  .syn-attribute { --syn: var(--syntax-attribute); }
  .syn-property { --syn: var(--syntax-property); }
  /* Still to type: the syntax colour, dimmed towards the background. */
  .pending {
    color: color-mix(in srgb, var(--syn) 45%, var(--surface));
  }
  .correct {
    color: var(--syn);
  }
  .error {
    color: var(--text-error);
    background: var(--error-bg);
  }
  .space.error {
    box-shadow: inset 0 -0.15em var(--text-error);
  }
  .nl {
    color: var(--muted);
  }
  .nl.error {
    color: var(--text-error);
  }
  .cursor {
    color: var(--syn);
    background: color-mix(in srgb, var(--cursor) 18%, transparent);
    box-shadow: inset 0 -0.12em var(--cursor);
    border-radius: 0.1em;
  }
  .nl.cursor {
    color: var(--muted);
  }
  .cursor.error {
    color: var(--text-error);
    background: color-mix(in srgb, var(--text-error) 22%, transparent);
    box-shadow: inset 0 -0.12em var(--text-error);
  }
</style>
