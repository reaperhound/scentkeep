<script lang="ts">
  import { backend, type ParfumoNote, type ParfumoStatus } from '$lib/backend';

  let { query, layer, selected, status, onSelect }: {
    query: string;
    layer: string;
    selected: string[];
    status: ParfumoStatus | null;
    onSelect: (note: ParfumoNote) => void;
  } = $props();
  let results = $state<ParfumoNote[]>([]);
  let busy = $state(false);
  let error = $state('');
  let searchedQuery = $state('');
  const currentQuery = $derived(query.trim());

  async function search() {
    if (busy || !status?.connected || currentQuery.length < 2) return;
    const term = currentQuery;
    busy = true;
    error = '';
    results = [];
    searchedQuery = term;
    try { results = await backend.searchParfumoNotes(term); }
    catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
</script>

<div class="parfumo-lookup">
  <button type="button" disabled={busy || !status?.connected || currentQuery.length < 2} onclick={search}>
    {busy ? 'Searching Parfumo…' : 'Search Parfumo notes'}
  </button>
  <small>{status === null ? 'Checking Parfumo connection…' : !status.connected ? 'Connect Parfumo in Settings to search its notes.' : currentQuery.length < 2 ? 'Enter at least two characters above to search Parfumo.' : 'Search Parfumo and select a note to add it to this layer.'}</small>
  {#if searchedQuery && searchedQuery === currentQuery}
    {#if busy}<p role="status">Searching Parfumo for “{searchedQuery}”…</p>
    {:else if error}<p class="note-error" role="alert">{error}</p>
    {:else if !results.length}<p role="status">No matching Parfumo notes. Try another name or add your own above.</p>
    {:else}
      <ul aria-label={`Parfumo results for ${layer}`}>
        {#each results as note}
          {@const added = selected.some((name) => name.toLocaleLowerCase() === note.name.toLocaleLowerCase())}
          <li><button type="button" disabled={added} onclick={() => onSelect(note)}>
            {#if note.imageUrl}<img src={note.imageUrl} alt="" loading="lazy" />{/if}
            <span>{note.name}<small>{added ? `Already in ${layer}` : `Add to ${layer}`}</small></span>
          </button></li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .parfumo-lookup { margin: 10px 0 16px; }
  small { display: block; margin-top: 5px; color: #746b60; }
  ul { list-style: none; padding: 0; max-height: 260px; overflow-y: auto; border: 1px solid #e5dfd5; border-radius: 8px; }
  li button { display: flex; align-items: center; gap: 12px; width: 100%; padding: 10px 12px; text-align: left; border: 0; background: transparent; cursor: pointer; }
  li button:hover, li button:focus-visible { background: #f4efe7; }
  li button:disabled { opacity: .6; cursor: default; }
  img { width: 40px; height: 40px; object-fit: contain; }
</style>
