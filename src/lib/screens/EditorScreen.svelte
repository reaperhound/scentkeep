<script lang="ts">
  import { backend, type Fragrance, type ParfumoNote, type ParfumoFragranceResult, type ParfumoFragranceDraft } from '$lib/backend';

  let { fragrance = $bindable(), importedNotes = $bindable(), entries, noteNames, error, saving, onSave }: {
    fragrance: Fragrance;
    importedNotes: ParfumoNote[];
    entries: Fragrance[];
    noteNames: string[];
    error: string;
    saving: boolean;
    onSave: () => void;
  } = $props();

  let parfumoQuery = $state('');
  let parfumoResults = $state<ParfumoFragranceResult[]>([]);
  let parfumoDraft = $state<ParfumoFragranceDraft | null>(null);
  let parfumoBusy = $state(false);
  let parfumoError = $state('');
  let parfumoSearched = $state(false);
  let parfumoApplied = $state(false);
  async function searchParfumo() {
    if (parfumoBusy || parfumoQuery.trim().length < 2) return;
    parfumoBusy = true; parfumoError = ''; parfumoResults = []; parfumoDraft = null; parfumoSearched = false; parfumoApplied = false;
    try {
      if (/^https?:/i.test(parfumoQuery.trim())) parfumoDraft = await backend.importParfumoFragrance(parfumoQuery.trim());
      else { parfumoResults = await backend.searchParfumoFragrances(parfumoQuery); parfumoSearched = true; }
    } catch (error) { parfumoError = String(error); }
    finally { parfumoBusy = false; }
  }
  async function readParfumo(url: string) {
    parfumoBusy = true; parfumoError = ''; parfumoDraft = null;
    try { parfumoDraft = await backend.importParfumoFragrance(url); }
    catch (error) { parfumoError = String(error); }
    finally { parfumoBusy = false; }
  }
  function applyParfumo() {
    if (!parfumoDraft) return;
    const draft = parfumoDraft;
    // Only available catalogue fields replace the editor's current values.
    for (const key of ['name', 'brand', 'audience', 'concentration', 'perfumer', 'imageUrl'] as const) {
      if (draft[key]) fragrance[key] = draft[key];
    }
    if (draft.year !== null) fragrance.year = draft.year;
    for (const key of ['topNotes', 'heartNotes', 'baseNotes'] as const) {
      if (draft[key].length) fragrance[key] = [...draft[key]];
    }
    importedNotes = [...new Map([...importedNotes, ...draft.notes].map((note) => [note.name.toLocaleLowerCase(), note])).values()];
    fragrance.description = [draft.description || fragrance.description, `Source: ${draft.sourceUrl}`].filter(Boolean).join('\n\n');
    parfumoApplied = true; parfumoDraft = null; parfumoResults = []; parfumoSearched = false;
  }

  type NoteLayer = 'topNotes' | 'heartNotes' | 'baseNotes';
  const steps = ['General information', 'Fragrance pyramid', 'Additional details'];
  const concentrations = ['Eau Fraîche', 'Eau de Cologne', 'Eau de Toilette', 'Eau de Parfum', 'Parfum', 'Extrait de Parfum', 'Fragrance Oil'];
  const categories = [{ value: 'Women', icon: '/female.svg', label: 'Women' }, { value: 'Men', icon: '/male.svg', label: 'Men' }, { value: 'Unisex', icon: '/unisex-symbol-color-icon.svg', label: 'Unisex' }];
  const ratingFields = [{ key: 'scent', label: 'Scent' }, { key: 'longevity', label: 'Longevity' }, { key: 'sillage', label: 'Sillage' }, { key: 'bottle', label: 'Bottle' }, { key: 'valueForMoney', label: 'Value for money' }] as const;
  let step = $state(0);
  let localError = $state('');
  let categoryMenuOpen = $state(false);
  let concentrationMenuOpen = $state(false);
  let brandSuggestionsOpen = $state(false);
  let highlightedBrandIndex = $state(-1);
  let noteInputs = $state<Record<NoteLayer, string>>({ topNotes: '', heartNotes: '', baseNotes: '' });
  let activeNoteInput = $state<NoteLayer | null>(null);
  let highlightedNoteIndex = $state(-1);
  let duplicateNoteLayer = $state<NoteLayer | null>(null);
  const brandSuggestions = $derived.by(() => {
    const query = fragrance.brand.trim().toLocaleLowerCase();
    if (!query) return [];
    return [...new Map(entries.map((item) => [item.brand.trim().toLocaleLowerCase(), item.brand.trim()])).values()]
      .filter((brand) => brand && brand.toLocaleLowerCase().includes(query) && brand.toLocaleLowerCase() !== query)
      .sort((a, b) => Number(!a.toLocaleLowerCase().startsWith(query)) - Number(!b.toLocaleLowerCase().startsWith(query)) || a.localeCompare(b)).slice(0, 6);
  });
  function overallRating() {
    const values = ratingFields.map(({ key }) => fragrance.ratings[key]).filter((value): value is number => value !== null && Number.isFinite(value));
    return values.length ? Math.round(values.reduce((sum, value) => sum + value, 0) / values.length * 10) / 10 : null;
  }
  function suggestionsFor(kind: NoteLayer) {
    const query = noteInputs[kind].trim().toLocaleLowerCase();
    return noteNames.filter((name) => name.toLocaleLowerCase().includes(query) && !fragrance[kind].some((added) => added.toLocaleLowerCase() === name.toLocaleLowerCase())).slice(0, 6);
  }
  function selectBrand(brand: string) { fragrance.brand = brand; brandSuggestionsOpen = false; highlightedBrandIndex = -1; }
  function handleBrandKeydown(event: KeyboardEvent) {
    if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && brandSuggestions.length) { event.preventDefault(); brandSuggestionsOpen = true; highlightedBrandIndex = event.key === 'ArrowDown' ? (highlightedBrandIndex + 1) % brandSuggestions.length : (highlightedBrandIndex - 1 + brandSuggestions.length) % brandSuggestions.length; }
    else if (event.key === 'Enter' && brandSuggestionsOpen && highlightedBrandIndex >= 0) { event.preventDefault(); selectBrand(brandSuggestions[highlightedBrandIndex]); }
    else if (event.key === 'Escape') { brandSuggestionsOpen = false; highlightedBrandIndex = -1; }
  }
  function addNote(kind: NoteLayer, selected?: string) {
    const value = (selected ?? noteInputs[kind]).trim();
    if (!value) return;
    const note = noteNames.find((name) => name.toLocaleLowerCase() === value.toLocaleLowerCase()) ?? value;
    if (fragrance[kind].some((name) => name.toLocaleLowerCase() === note.toLocaleLowerCase())) { duplicateNoteLayer = kind; activeNoteInput = null; return; }
    duplicateNoteLayer = null; fragrance[kind] = [...fragrance[kind], note]; noteInputs[kind] = ''; activeNoteInput = null;
  }
  function handleNoteKeydown(event: KeyboardEvent, kind: NoteLayer) {
    const suggestions = suggestionsFor(kind);
    if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && suggestions.length) { event.preventDefault(); activeNoteInput = kind; highlightedNoteIndex = event.key === 'ArrowDown' ? (highlightedNoteIndex + 1) % suggestions.length : highlightedNoteIndex < 0 ? suggestions.length - 1 : (highlightedNoteIndex - 1 + suggestions.length) % suggestions.length; }
    else if (event.key === 'Enter') { event.preventDefault(); addNote(kind, activeNoteInput === kind && highlightedNoteIndex >= 0 ? suggestions[highlightedNoteIndex] : undefined); }
    else if (event.key === 'Escape') activeNoteInput = null;
  }
  function removeNote(kind: NoteLayer, index: number) {
    if (window.confirm(`Are you sure you want to remove the note "${fragrance[kind][index]}" from this fragrance?`)) fragrance[kind] = fragrance[kind].filter((_, i) => i !== index);
  }
  function next() {
    localError = '';
    if (step === 0 && (!fragrance.name.trim() || !fragrance.brand.trim())) { localError = 'Add a fragrance name and brand to continue.'; return; }
    if (step === 0 && entries.some((item) => item.id !== fragrance.id && item.name.trim().toLocaleLowerCase() === fragrance.name.trim().toLocaleLowerCase() && item.brand.trim().toLocaleLowerCase() === fragrance.brand.trim().toLocaleLowerCase())) { localError = 'This fragrance is already in your collection.'; return; }
    if (step < 2) step += 1;
  }
</script>

<div class="page-content">
  <div class="heading"><div><h1>{fragrance.id === undefined ? 'Add a fragrance' : 'Edit fragrance'}</h1><p>Capture the details that make each scent unforgettable.</p></div><div class="step-count">STEP 0{step + 1} <span>/ 03</span></div></div>
  <div class="progress" aria-label="Form progress">{#each steps as label, index}<button class:current={step === index} class:done={step > index} onclick={() => { if (index < step) step = index; }}><span class="step-number">{step > index ? '✓' : `0${index + 1}`}</span><span>{label}</span></button>{#if index < 2}<div class:filled={step > index} class="connector"></div>{/if}{/each}</div>
  {#if localError || error}<div class="message error" role="alert">{localError || error}</div>{/if}
  {#if step === 0}
    <section class="parfumo-import" aria-label="Import from Parfumo">
      <h2>Find on Parfumo</h2>
      <p>Search by fragrance or brand, or paste a fragrance link. Connect Parfumo in Settings first.</p>
      <form onsubmit={(event) => { event.preventDefault(); searchParfumo(); }}>
        <input aria-label="Parfumo fragrance search or link" placeholder="e.g. Dior Sauvage, or a Parfumo link" bind:value={parfumoQuery} disabled={parfumoBusy} />
        <button type="submit" class="primary-button" disabled={parfumoBusy || parfumoQuery.trim().length < 2}>{parfumoBusy ? 'Loading…' : 'Search Parfumo'}</button>
      </form>
      {#if parfumoBusy}<p role="status">Reading the fragrance and note details from Parfumo…</p>{/if}
      {#if parfumoError}<p class="message error" role="alert">{parfumoError}</p>{/if}
      {#if parfumoSearched && !parfumoResults.length}<p role="status">No fragrances found. Try another name or paste the fragrance’s Parfumo link.</p>{/if}
      {#if parfumoResults.length && !parfumoDraft}
        <ul class="parfumo-results">{#each parfumoResults as result}<li><button type="button" disabled={parfumoBusy} onclick={() => readParfumo(result.url)}>{#if result.imageUrl}<img src={result.imageUrl} alt="" loading="lazy" />{/if}<span><strong>{result.name}</strong><small>{decodeURI(new URL(result.url).pathname).replaceAll('_', ' ')}</small></span><span>Review →</span></button></li>{/each}</ul>
      {/if}
      {#if parfumoDraft}
        <div class="parfumo-draft">
          {#if parfumoDraft.imageUrl}<img class="parfumo-bottle" src={parfumoDraft.imageUrl} alt={parfumoDraft.name} />{/if}
          <div><h3>{parfumoDraft.name}</h3><p>{parfumoDraft.brand}{parfumoDraft.year ? ` · ${parfumoDraft.year}` : ''}</p>
          <p>{[parfumoDraft.concentration, parfumoDraft.audience, parfumoDraft.perfumer].filter(Boolean).join(' · ')}</p>
          {#each [{ label: 'Top notes', notes: parfumoDraft.topNotes }, { label: 'Heart notes', notes: parfumoDraft.heartNotes }, { label: 'Base notes', notes: parfumoDraft.baseNotes }] as layer}<p><strong>{layer.label}:</strong> {layer.notes.join(', ') || 'Not available'}</p>{/each}
          {#if parfumoDraft.description}<p>{parfumoDraft.description}</p>{/if}
          {#each parfumoDraft.warnings as warning}<p role="status">{warning}</p>{/each}
          <p><small>Available details will replace the corresponding form fields. When you save the fragrance, its notes and available images, categories, and source links will also be saved in the Notes library. Images remain linked to Parfumo.</small></p>
          <button type="button" class="primary-button" onclick={applyParfumo}>Use these details</button>
          <button type="button" onclick={() => parfumoDraft = null}>Dismiss</button></div>
        </div>
      {/if}
      {#if parfumoApplied}<p role="status">Details added to the form. Review all three steps, then save the fragrance and its notes.</p>{/if}
    </section>
  {/if}
  <div class="form-card">
    {#if step === 0}
      <div class="card-heading"><div><h2>The essentials</h2><p>Start with the fragrance’s identity.</p></div></div>
      <div class="form-grid">
        <label class="field wide"><span class="field-label-row"><span class="field-label-text">Fragrance name <b>*</b></span></span><input placeholder="e.g. Odéon Sunset" bind:value={fragrance.name} /></label>
        <div class="field brand-field"><label class="field-label-row" for="fragrance-brand"><span class="field-label-text">Brand / house <b>*</b></span></label><div class="brand-input-wrap"><input id="fragrance-brand" role="combobox" aria-autocomplete="list" aria-expanded={brandSuggestionsOpen && brandSuggestions.length > 0} aria-controls="brand-suggestions" autocomplete="off" placeholder="e.g. Memo Paris" bind:value={fragrance.brand} onfocus={() => { brandSuggestionsOpen = true; highlightedBrandIndex = -1; }} oninput={() => { brandSuggestionsOpen = true; highlightedBrandIndex = -1; }} onblur={() => { brandSuggestionsOpen = false; highlightedBrandIndex = -1; }} onkeydown={handleBrandKeydown} />{#if brandSuggestionsOpen && brandSuggestions.length}<div id="brand-suggestions" class="note-suggestions brand-suggestions" role="listbox" aria-label="Saved brands">{#each brandSuggestions as suggestion, index}<button type="button" role="option" aria-selected={highlightedBrandIndex === index} class:highlighted={highlightedBrandIndex === index} tabindex="-1" onpointerdown={(event) => event.preventDefault()} onmouseenter={() => highlightedBrandIndex = index} onclick={() => selectBrand(suggestion)}>{suggestion}</button>{/each}</div>{/if}</div></div>
        <label class="field"><span class="field-label-row"><span class="field-label-text">Collection</span><span class="field-hint">Optional</span></span><input placeholder="e.g. Fleurs Bohèmes" bind:value={fragrance.collection} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Release year</span></span><input type="number" min="1800" max="2100" bind:value={fragrance.year} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Perfumer</span><span class="field-hint">Optional</span></span><input placeholder="e.g. Aliénor Massenet" bind:value={fragrance.perfumer} /></label>
        <div class="field concentration-field"><div class="field-label-row"><span class="field-label-text">Concentration</span><span class="field-hint">Optional</span></div><div class="category-picker"><button type="button" class="category-trigger" aria-haspopup="listbox" aria-expanded={concentrationMenuOpen} onclick={() => { concentrationMenuOpen = !concentrationMenuOpen; categoryMenuOpen = false; }}>{fragrance.concentration || 'Select concentration'}<span class:rotated={concentrationMenuOpen} class="category-chevron"></span></button>{#if concentrationMenuOpen}<div class="category-options concentration-options" role="listbox" aria-label="Concentration"><button type="button" onclick={() => { fragrance.concentration = ''; concentrationMenuOpen = false; }}>Select concentration</button>{#each concentrations as concentration}<button type="button" class:selected={fragrance.concentration === concentration} onclick={() => { fragrance.concentration = concentration; concentrationMenuOpen = false; }}>{concentration}{#if fragrance.concentration === concentration}<span>✓</span>{/if}</button>{/each}</div>{/if}</div></div>
      </div>
      <div class="section-divider"></div><div class="field category-field"><div class="field-label-row"><span class="field-label-text">Status</span></div><div class="category-picker"><button type="button" class="category-trigger" onclick={() => categoryMenuOpen = !categoryMenuOpen}>{fragrance.shelf}<span class:rotated={categoryMenuOpen} class="category-chevron"></span></button>{#if categoryMenuOpen}<div class="category-options" role="listbox" aria-label="Status">{#each ['Collection', 'Wishlist', 'Empties'] as shelf}<button type="button" class:selected={fragrance.shelf === shelf} onclick={() => { fragrance.shelf = shelf as typeof fragrance.shelf; categoryMenuOpen = false; }}>{shelf}{#if fragrance.shelf === shelf}<span>✓</span>{/if}</button>{/each}</div>{/if}</div></div>
      <div class="field-title">Who is this fragrance for?</div><div class="choice-grid">{#each categories as category}<button type="button" class:selected={fragrance.audience === category.value} onclick={() => fragrance.audience = category.value}><span class="choice-icon"><img src={category.icon} alt="" /></span><strong>{category.label}</strong><span class="radio-dot"></span></button>{/each}</div>
    {:else if step === 1}
      <div class="card-heading"><div><h2>Fragrance pyramid</h2><p>Build the scent, layer by layer. Add each note separately.</p></div></div>
      {#each [{ key: 'topNotes', title: 'Top notes', hint: 'The first impression', example: 'Candied apple' }, { key: 'heartNotes', title: 'Heart notes', hint: 'The heart of the scent', example: 'Caramel' }, { key: 'baseNotes', title: 'Base notes', hint: 'The lasting impression', example: 'Sandalwood' }] as group}
        {@const kind = group.key as NoteLayer}<div class="note-section"><div class="note-title"><div><h3>{group.title}</h3><p>{group.hint}</p></div></div><div class="note-entry"><div class="note-input-wrap"><input aria-label={`Add ${group.title.toLowerCase()}`} role="combobox" aria-expanded={activeNoteInput === kind && suggestionsFor(kind).length > 0} aria-controls="note-suggestions" autocomplete="off" placeholder={`e.g. ${group.example}`} bind:value={noteInputs[kind]} onfocus={() => { activeNoteInput = kind; highlightedNoteIndex = -1; }} oninput={() => { activeNoteInput = kind; highlightedNoteIndex = -1; duplicateNoteLayer = null; }} onblur={() => activeNoteInput = null} onkeydown={(event) => handleNoteKeydown(event, kind)} />{#if activeNoteInput === kind && suggestionsFor(kind).length}<div class="note-suggestions" role="listbox" aria-label="Saved notes"><div class="suggestions-heading">Saved notes</div>{#each suggestionsFor(kind) as suggestion, index}<button type="button" class:highlighted={highlightedNoteIndex === index} tabindex="-1" onpointerdown={(event) => event.preventDefault()} onmouseenter={() => highlightedNoteIndex = index} onclick={() => addNote(kind, suggestion)}>{suggestion}</button>{/each}</div>{/if}</div><button type="button" onclick={() => addNote(kind)}>+ Add note</button></div>{#if duplicateNoteLayer === kind}<div class="note-error" role="alert">This note is already in the {group.title.toLowerCase()}.</div>{/if}<div class="note-chips">{#each fragrance[kind] as note, index}<button type="button" title={`Remove ${note}`} onclick={() => removeNote(kind, index)}>{note} <span>×</span></button>{/each}{#if fragrance[kind].length === 0}<span class="empty-hint">No notes added yet</span>{/if}</div></div>
      {/each}
    {:else}
      <div class="card-heading"><div><h2>Finishing touches</h2><p>Add context and a little more character.</p></div></div>
      <label class="field"><span class="field-label-row"><span class="field-label-text">Description</span><span class="field-hint">Optional</span></span><textarea rows="5" placeholder="What makes this fragrance special?" bind:value={fragrance.description}></textarea></label>
      <div class="form-grid details">{#each ratingFields as { key, label }}<label class="field"><span class="field-label-row"><span class="field-label-text">{label}</span><span class="field-hint">0 to 10</span></span><input type="number" min="0" max="10" step="0.1" bind:value={fragrance.ratings[key]} /></label>{/each}</div>
      <div class="detail-rating"><strong>Overall rating: {overallRating()?.toFixed(1) ?? 'Not rated yet'}</strong></div>
      <div class="form-grid details"><label class="field"><span class="field-label-row"><span class="field-label-text">Bottles owned</span></span><input type="number" min="0" step="1" bind:value={fragrance.bottlesOwned} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Bottles finished</span></span><input type="number" min="0" step="1" bind:value={fragrance.bottlesFinished} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Price bought</span><span class="field-hint">Optional</span></span><input type="number" min="0" step="0.01" bind:value={fragrance.priceBought} /></label>{#if fragrance.shelf === 'Wishlist'}<label class="field"><span class="field-label-row"><span class="field-label-text">Current price</span><span class="field-hint">Optional</span></span><input type="number" min="0" step="0.01" bind:value={fragrance.currentPrice} /></label>{/if}</div>
      <label class="favourite-field"><input type="checkbox" bind:checked={fragrance.favourite} /> Favourite</label><label class="field"><span class="field-label-row"><span class="field-label-text">Image URL</span><span class="field-hint">Optional</span></span><input type="url" placeholder="https://example.com/fragrance.jpg" bind:value={fragrance.imageUrl} /><small>A link to the bottle image. Leave blank to use a placeholder.</small></label><div class="summary"><div><strong>Ready for your collection</strong><p>{fragrance.name || 'Your fragrance'} by {fragrance.brand || 'your chosen house'}, {fragrance.year}, {fragrance.audience}</p></div></div>
    {/if}
  </div>
  <div class="form-footer"><span>Fields marked <b>*</b> are required</span><div>{#if step > 0}<button class="back-button" disabled={saving} onclick={() => { step -= 1; localError = ''; }}>← Back</button>{/if}{#if step < 2}<button class="primary-button" onclick={next}>Continue</button>{:else}<button class="primary-button" disabled={saving} onclick={onSave}>{#if saving}<span class="save-spinner" aria-hidden="true"></span>Saving…{:else}{fragrance.id === undefined ? 'Save fragrance' : 'Save changes'}{/if}</button>{/if}</div></div>
  {#if saving}<p class="save-status" role="status">Saving your fragrance…</p>{/if}
</div>

<style>
  .parfumo-import { margin-bottom: 24px; padding: 24px; border: 1px solid #e5dfd5; border-radius: 16px; background: #fffdf9; }
  .parfumo-import h2, .parfumo-import h3 { margin: 0 0 8px; }
  .parfumo-import p { line-height: 1.6; }
  .parfumo-import form { display: flex; gap: 12px; }
  .parfumo-import input { flex: 1; min-width: 0; padding: 12px; border: 1px solid #d8d0c4; border-radius: 8px; }
  .parfumo-results { list-style: none; padding: 0; max-height: 360px; overflow: auto; }
  .parfumo-results button { display: flex; align-items: center; gap: 16px; width: 100%; text-align: left; padding: 12px; border: 0; border-bottom: 1px solid #e5dfd5; background: transparent; cursor: pointer; }
  .parfumo-results button:hover { background: #f4efe7; }
  .parfumo-results img { width: 48px; height: 64px; object-fit: contain; }
  .parfumo-results small { display: block; overflow-wrap: anywhere; margin-top: 4px; }
  .parfumo-results button > span:first-of-type { flex: 1; }
  .parfumo-draft { display: flex; gap: 24px; margin-top: 24px; }
  .parfumo-bottle { width: 120px; height: 180px; object-fit: contain; }
  @media (max-width: 600px) { .parfumo-import form, .parfumo-draft { flex-direction: column; } }
</style>
