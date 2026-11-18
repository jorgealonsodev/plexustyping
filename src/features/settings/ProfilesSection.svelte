<script lang="ts">
  import { deleteConfirmation } from "../profiles/profiles";
  import SettingField from "./SettingField.svelte";
  import type { SettingsStore } from "./store.svelte";

  // "Perfiles" (S4): each profile has its own history, statistics, layout, language and theme.
  let { settings }: { settings: SettingsStore } = $props();

  let newName = $state("");
  let renaming = $state<number | null>(null);
  let renameValue = $state("");
  let confirming = $state<number | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  const shown = $derived(error ?? settings.profileError);

  const focus = (el: HTMLElement) => el.focus();

  /** Runs one profile command at a time; its Spanish error stays shown until the next one. */
  async function run(op: () => Promise<string | null>, done?: () => void) {
    if (busy) return;
    busy = true;
    error = await op();
    busy = false;
    if (error === null) done?.();
  }

  function create(e: SubmitEvent) {
    e.preventDefault();
    void run(
      () => settings.createProfile(newName),
      () => (newName = ""),
    );
  }

  function startRename(id: number, name: string) {
    confirming = null;
    renaming = id;
    renameValue = name;
  }

  function rename(e: SubmitEvent, id: number) {
    e.preventDefault();
    void run(
      () => settings.renameProfile(id, renameValue),
      () => (renaming = null),
    );
  }

  function remove(id: number) {
    void run(
      () => settings.deleteProfile(id),
      () => (confirming = null),
    );
  }
</script>

<p class="intro">
  Cada perfil tiene su historial, sus estadísticas, su distribución, su idioma y su tema; el resto de ajustes es común.
</p>

{#if settings.profiles.length === 0}
  <p class="notice">{settings.profilesReady ? "No se pudieron cargar los perfiles." : "Cargando perfiles…"}</p>
{:else}
  <ul class="profiles" aria-label="Perfiles">
    {#each settings.profiles as p (p.id)}
      {@const active = p.id === settings.activeProfile?.id}
      <li class:active aria-current={active ? "true" : undefined}>
        {#if renaming === p.id}
          <form class="row" onsubmit={(e) => rename(e, p.id)}>
            <input type="text" aria-label="Nuevo nombre de «{p.name}»" bind:value={renameValue} {@attach focus} />
            <button type="submit" disabled={busy}>Guardar</button>
            <button type="button" onclick={() => (renaming = null)}>Cancelar</button>
          </form>
        {:else}
          <div class="row">
            <span class="name">{p.name}</span>
            {#if active}
              <span class="badge">Activo</span>
            {:else}
              <button type="button" disabled={busy} onclick={() => void run(() => settings.useProfile(p.id))}>Usar</button>
            {/if}
            <button type="button" disabled={busy} onclick={() => startRename(p.id, p.name)}>Renombrar</button>
            <button
              type="button"
              disabled={busy}
              onclick={() => {
                renaming = null;
                confirming = p.id;
              }}>Borrar</button
            >
          </div>
        {/if}
        {#if confirming === p.id}
          <div class="confirm" role="alert">
            <p>{deleteConfirmation(p.name)}</p>
            <button type="button" class="danger" disabled={busy} onclick={() => remove(p.id)} {@attach focus}>Borrar</button>
            <button type="button" onclick={() => (confirming = null)}>Cancelar</button>
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<form onsubmit={create}>
  <SettingField label="Nuevo perfil" hint="Empieza con el idioma y la distribución del perfil activo.">
    {#snippet children({ id, hintId })}
      <input {id} type="text" aria-describedby={hintId} placeholder="Nombre" bind:value={newName} />
      <button type="submit" disabled={busy || settings.activeProfile === null}>Crear</button>
    {/snippet}
  </SettingField>
</form>

{#if shown}<p class="notice error" role="alert">{shown}</p>{/if}

<style>
  .intro {
    margin: 0.5rem 0 0.75rem;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .profiles {
    margin: 0 0 0.75rem;
    padding: 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
  }
  li {
    padding: 0.45rem 0.75rem;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  li.active {
    border-left: 3px solid var(--accent);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  li.active .name {
    font-weight: 600;
  }
  .row input {
    flex: 1;
    min-width: 0;
  }
  .badge {
    padding: 0.1rem 0.5rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--on-accent);
    background: var(--accent);
    border-radius: 0.3rem;
  }
  button {
    padding: 0.25rem 0.6rem;
    font-size: 0.85rem;
  }
  .confirm {
    margin-top: 0.4rem;
  }
  .confirm p {
    margin: 0 0 0.4rem;
    font-size: 0.85rem;
  }
  .confirm button + button {
    margin-left: 0.4rem;
  }
  .danger {
    color: var(--danger);
    border-color: var(--danger);
  }
  .notice {
    margin: 0.4rem 0 0;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .notice.error {
    color: var(--danger);
  }
</style>
