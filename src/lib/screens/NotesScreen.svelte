<script lang="ts">
  import type { CsvImportReport, NoteImage, ParfumoNote, ParfumoStatus } from '$lib/backend';
  import NavIcon from '$lib/NavIcon.svelte';

  let {
    notes, visibleNotes, pagedNotes, notesByName, page, pageCount, pageSize,
    search = $bindable(), newNoteName = $bindable(), results, resultsOpen = $bindable(),
    highlightedResult = $bindable(), searching, searchError, savingResult, status,
    editingNote = $bindable(), editedNoteName = $bindable(), savingImage,
    imageDrafts = $bindable(), editingCategories = $bindable(), categoryDraft = $bindable(),
    refreshingMetadata, success, error, csvReport, csvBusy, listElement = $bindable(),
    onImport, onExport, onCreate, onResultKeydown, onSelectResult, onRefreshMetadata,
    onStartFragrance, onRename, onDelete, onSaveCategories, onSetImage, onRemoveImage, onChangePage
  }: {
    notes: string[]; visibleNotes: string[]; pagedNotes: string[]; notesByName: Map<string, NoteImage>;
    page: number; pageCount: number; pageSize: number; search: string; newNoteName: string;
    results: ParfumoNote[]; resultsOpen: boolean; highlightedResult: number; searching: boolean;
    searchError: string; savingResult: boolean; status: ParfumoStatus | null;
    editingNote: string; editedNoteName: string; savingImage: string; imageDrafts: Record<string, string>;
    editingCategories: string; categoryDraft: string; refreshingMetadata: boolean; success: string; error: string;
    csvReport: { kind: 'fragrances' | 'notes'; result: CsvImportReport } | null; csvBusy: boolean;
    listElement: HTMLDivElement | undefined; onImport: (event: Event) => void; onExport: () => void;
    onCreate: () => void; onResultKeydown: (event: KeyboardEvent) => void; onSelectResult: (note: ParfumoNote) => void;
    onRefreshMetadata: () => void; onStartFragrance: () => void; onRename: (note: string) => void;
    onDelete: (note: string) => void; onSaveCategories: (note: string) => void; onSetImage: (note: string) => void;
    onRemoveImage: (note: string) => void; onChangePage: (page: number) => void;
  } = $props();
  let fileInput = $state<HTMLInputElement>();
  function thumbnail(imageUrl: string, width = 64) {
    if (!imageUrl.startsWith('https://media.parfumo.com/notes_png/')) return imageUrl;
    const url = new URL(imageUrl); url.searchParams.set('width', String(width)); url.searchParams.set('aspect_ratio', '1:1'); return url.toString();
  }
</script>

<div class="page-content">
  <div class="heading"><div><h1>Perfume notes</h1><p>See every note in your collection and give it an image.</p><p class="note-source-tip">Image tip: When adding a note image link, try Parfumo first if it has a suitable photo; its images are often clearer than Fragrantica’s.</p></div><div class="csv-actions"><button class="back-button" disabled={csvBusy} onclick={() => fileInput?.click()}><NavIcon name="import" size={18} /><span>Import CSV</span></button><button class="back-button" disabled={csvBusy} onclick={onExport}><NavIcon name="export" size={18} /><span>Export CSV</span></button><input type="file" accept=".csv,.xlsx,.xls,text/csv" bind:this={fileInput} onchange={onImport} hidden /></div></div>
  {#if success}<div class="message success">{success}</div>{/if}{#if error}<div class="message error" role="alert">{error}</div>{/if}
  {#if csvReport?.kind === 'notes'}<div class="message import-result" role="status"><strong>Imported {csvReport.result.imported} {csvReport.result.imported === 1 ? 'note' : 'notes'}.</strong>{#if csvReport.result.duplicates.length}<p>Skipped {csvReport.result.duplicates.length} duplicate {csvReport.result.duplicates.length === 1 ? 'note' : 'notes'}:</p><ul>{#each csvReport.result.duplicates as duplicate}<li>{duplicate}</li>{/each}</ul>{/if}</div>{/if}
  <div class="parfumo-note-search">
    <form class="new-note-form" onsubmit={(event) => { event.preventDefault(); onCreate(); resultsOpen = false; }}><input aria-label="Search Parfumo or enter a note name" role="combobox" aria-autocomplete="list" aria-expanded={resultsOpen && status?.connected && newNoteName.trim().length >= 2} aria-controls="parfumo-note-results" autocomplete="off" placeholder="Search Parfumo notes or add your own..." disabled={savingResult} bind:value={newNoteName} onfocus={() => { resultsOpen = true; highlightedResult = -1; }} oninput={() => { resultsOpen = true; highlightedResult = -1; }} onkeydown={onResultKeydown} /><button type="submit" disabled={!newNoteName.trim() || savingResult}>+ Add note</button></form>
    {#if savingResult}<p class="parfumo-save-status" role="status"><span class="save-spinner" aria-hidden="true"></span>Saving note…</p>{/if}
    {#if status?.connected && newNoteName.trim().length >= 2 && resultsOpen}<div id="parfumo-note-results" class="parfumo-note-results" role="listbox" aria-label="Parfumo note results"><div class="suggestions-heading">Parfumo notes</div>{#if searching}<p>Searching Parfumo…</p>{:else if searchError}<p class="parfumo-result-error">{searchError}</p>{:else if results.length === 0}<p>No matching Parfumo notes.</p>{:else}{#each results as note, index}<button type="button" role="option" aria-selected={highlightedResult === index} class:highlighted={highlightedResult === index} tabindex="-1" disabled={savingResult} onclick={() => onSelectResult(note)}>{#if note.imageUrl}<img src={thumbnail(note.imageUrl)} alt="" loading="lazy" decoding="async" />{:else}<span class="parfumo-result-placeholder"><NavIcon name="notes" size={18} /></span>{/if}<span>{note.name}<small>Save Parfumo note</small></span></button>{/each}{/if}</div>{/if}
  </div>
  {#if !status?.connected}<p class="note-source-tip">Connect Parfumo in Settings to search its notes. You can still add a note by name.</p>{/if}
  <div class="library-toolbar"><div class="search-box">⌕ <input aria-label="Search notes" placeholder="Search perfume notes..." bind:value={search} /></div><div class="note-toolbar-actions">{#if status?.connected}<button type="button" disabled={refreshingMetadata} onclick={onRefreshMetadata}>{refreshingMetadata ? 'Filling metadata…' : 'Fill missing metadata'}</button>{/if}<span>{visibleNotes.length} {visibleNotes.length === 1 ? 'note' : 'notes'}</span></div></div>
  {#if visibleNotes.length === 0}
    <div class="empty-state"><div><NavIcon name="notes" size={48} /></div><h2>{notes.length === 0 ? 'No notes in your collection yet' : 'No notes found'}</h2><p>{notes.length === 0 ? 'Add a note above, or add notes to a fragrance.' : 'Try another search term.'}</p>{#if notes.length === 0}<button class="primary-button" onclick={onStartFragrance}>Add a fragrance</button>{/if}</div>
  {:else}
    <div class="notes-grid" bind:this={listElement}>{#each pagedNotes as note (note)}{@const info = notesByName.get(note.toLocaleLowerCase())}<article class="note-card"><div class="note-art">{#if info?.imageUrl}<img src={info.imageUrl} alt={note} loading="lazy" decoding="async" />{:else}<NavIcon name="notes" size={48} />{/if}</div><div class="note-card-body">
      {#if editingNote === note}<form class="rename-note-form" onsubmit={(event) => { event.preventDefault(); onRename(note); }}><input aria-label={`Rename ${note}`} bind:value={editedNoteName} /><button type="submit" disabled={!editedNoteName.trim()}>Save</button><button type="button" onclick={() => editingNote = ''}>Cancel</button></form>{:else}<h2>{note}</h2>{/if}<p>Used in {info?.usageCount ?? 0} fragrances</p>
      {#if editingCategories === note}<form class="note-category-form" onsubmit={(event) => { event.preventDefault(); onSaveCategories(note); }}><label for={`categories-${note}`}>Categories, separated by commas</label><input id={`categories-${note}`} bind:value={categoryDraft} placeholder="Fruits, Green notes" /><div><button type="submit">Save categories</button><button type="button" onclick={() => editingCategories = ''}>Cancel</button></div></form>{:else}{#if info?.categories.length}<div class="note-categories">{#each info.categories as category}<span>{category}</span>{/each}</div>{/if}<button class="edit-categories-button" type="button" onclick={() => { editingCategories = note; categoryDraft = info?.categories.join(', ') ?? ''; }}>Edit categories</button>{/if}
      <div class="note-source-slot">{#if info?.parfumoUrl}<a class="note-source-link" href={info.parfumoUrl} target="_blank" rel="noreferrer">View on Parfumo</a>{/if}</div><form class="note-image-form" onsubmit={(event) => { event.preventDefault(); onSetImage(note); }}><input type="url" aria-label={`Image link for ${note}`} placeholder="https://example.com/note.jpg" value={imageDrafts[note] ?? ''} oninput={(event) => imageDrafts[note] = event.currentTarget.value} /><div><button type="submit" disabled={savingImage === note || !imageDrafts[note]?.trim()}>{savingImage === note ? 'Saving…' : info?.imageUrl ? 'Update image link' : '+ Add image link'}</button>{#if info?.imageUrl}<button type="button" onclick={() => onRemoveImage(note)}>Remove image</button>{/if}</div></form><div class="note-manage-actions"><button onclick={() => { editingNote = note; editedNoteName = note; }}>Rename</button><button class="danger" onclick={() => onDelete(note)}>Delete note</button></div>
    </div></article>{/each}</div>
    {#if pageCount > 1}<nav class="pagination" aria-label="Perfume notes pagination"><span>Showing {(page - 1) * pageSize + 1}–{Math.min(page * pageSize, visibleNotes.length)} of {visibleNotes.length}</span><div><button type="button" disabled={page === 1} onclick={() => onChangePage(page - 1)}>Previous</button><span aria-live="polite">Page {page} of {pageCount}</span><button type="button" disabled={page === pageCount} onclick={() => onChangePage(page + 1)}>Next</button></div></nav>{/if}
  {/if}
</div>
