<script lang="ts">
  import { onMount } from 'svelte';
  import { save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { isTauri } from '@tauri-apps/api/core';
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { check, type Update } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { backend, type CsvImportReport, type Fragrance, type NoteImage } from '$lib/backend';
  import NavIcon from '$lib/NavIcon.svelte';

  const blank = (): Fragrance => ({ collection: '', name: '', brand: '', year: new Date().getFullYear(), audience: 'Unisex', concentration: '', description: '', topNotes: [], heartNotes: [], baseNotes: [], perfumer: '', bottleRating: null, ratingCount: null, imageUrl: '', shelf: 'Collection', bottlesOwned: 0, bottlesFinished: 0, favourite: false, priceBought: null, currentPrice: null });
  let fragrance = $state<Fragrance>(blank());
  let entries = $state<Fragrance[]>([]);
  let step = $state(0);
  let view = $state<'form' | 'library' | 'notes' | 'detail'>('library');
  let activeShelf = $state<'Collection' | 'Wishlist' | 'Empties'>('Collection');
  let categoryMenuOpen = $state(false);
  let concentrationMenuOpen = $state(false);
  let sidebarCollapsed = $state(false);
  let profileName = $state('Scentkeeper');
  let profilePicture = $state('');
  let draftProfileName = $state('');
  let draftProfilePicture = $state('');
  let profilePictureUrl = $state('');
  let profileDialogOpen = $state(false);
  let mobileSidebarOpen = $state(false);
  let selectedId = $state<number | null>(null);
  let noteImages = $state<NoteImage[]>([]);
  let noteSearch = $state('');
  let newNoteName = $state('');
  let editingNote = $state('');
  let editedNoteName = $state('');
  let savingNoteImage = $state('');
  let noteImageDrafts = $state<Record<string, string>>({});
  let error = $state('');
  let success = $state('');
  let saving = $state(false);
  let search = $state('');
  let collectionFilter = $state<'All' | 'Owned' | 'Wishlist' | 'Empties'>('All');
  let collectionLayout = $state<'grid' | 'list'>('grid');
  let openCardMenuId = $state<number | null>(null);
  let noteInputs = $state({ topNotes: '', heartNotes: '', baseNotes: '' });
  type NoteLayer = 'topNotes' | 'heartNotes' | 'baseNotes';
  let activeNoteInput = $state<NoteLayer | null>(null);
  let highlightedNoteIndex = $state(-1);
  let duplicateNoteLayer = $state<NoteLayer | null>(null);
  type CsvKind = 'fragrances' | 'notes';
  let fragranceCsvInput = $state<HTMLInputElement>();
  let noteCsvInput = $state<HTMLInputElement>();
  let csvBusy = $state(false);
  let csvReport = $state<{ kind: CsvKind; result: CsvImportReport } | null>(null);
  let availableUpdate = $state<Update | null>(null);
  let updateMessage = $state('');
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let canInstallUpdate = $state(true);
  const steps = ['General information', 'Fragrance pyramid', 'Additional details'];
  const concentrations = ['Eau Fraîche', 'Eau de Cologne', 'Eau de Toilette', 'Eau de Parfum', 'Parfum', 'Extrait de Parfum', 'Fragrance Oil'];
  const categories = [
    { value: 'Women', icon: '/female.svg', label: 'Women' },
    { value: 'Men', icon: '/male.svg', label: 'Men' },
    { value: 'Unisex', icon: '/unisex-symbol-color-icon.svg', label: 'Unisex' }
  ];
  const sexIconFor = (audience: string) => categories.find((category) => category.value === audience)?.icon ?? '/unisex-symbol-color-icon.svg';
  const allNotes = $derived(noteImages.map((note) => note.name).sort((a, b) => a.localeCompare(b)));
  const visibleNotes = $derived(allNotes.filter((note) => note.toLocaleLowerCase().includes(noteSearch.toLocaleLowerCase())));
  const selectedFragrance = $derived(entries.find((item) => item.id === selectedId));
  const visibleEntries = $derived(entries.filter((item) => (activeShelf === 'Collection' ? (collectionFilter === 'All' || (collectionFilter === 'Owned' ? item.shelf === 'Collection' : item.shelf === collectionFilter)) : item.shelf === activeShelf) && `${item.name} ${item.brand} ${item.collection}`.toLowerCase().includes(search.toLowerCase())));

  onMount(async () => {
    try {
      profileName = localStorage.getItem('scentkeep.profile.name') || 'Scentkeeper';
      profilePicture = localStorage.getItem('scentkeep.profile.picture') || '';
      entries = await backend.listFragrances();
      noteImages = await backend.listNoteImages(); }
    catch (e) { error = String(e); }
    if (isTauri()) {
      canInstallUpdate = await invoke<boolean>('can_install_update');
      void checkForUpdates(true);
    }
  });

  function editProfile() { draftProfileName = profileName; draftProfilePicture = profilePicture; profilePictureUrl = profilePicture.startsWith('http') ? profilePicture : ''; profileDialogOpen = true; }
  function saveProfile() { profileName = draftProfileName.trim() || 'Scentkeeper'; profilePicture = draftProfilePicture; localStorage.setItem('scentkeep.profile.name', profileName); localStorage.setItem('scentkeep.profile.picture', profilePicture); profileDialogOpen = false; }
  async function checkForUpdates(silent = false) {
    if (!isTauri() || checkingUpdate || installingUpdate) return;
    checkingUpdate = true;
    if (!silent) updateMessage = '';
    try {
      availableUpdate = await check();
      updateMessage = availableUpdate
        ? canInstallUpdate ? `Version ${availableUpdate.version} is available.` : `Version ${availableUpdate.version} is available. Download the package for your Linux distribution from the release page.`
        : silent ? '' : 'You are up to date.';
    } catch (e) {
      updateMessage = `Could not check for updates: ${String(e)}`;
    } finally { checkingUpdate = false; }
  }

  async function installUpdate() {
    if (!availableUpdate || installingUpdate) return;
    installingUpdate = true;
    updateMessage = 'Downloading update…';
    try {
      await availableUpdate.downloadAndInstall((event) => {
        if (event.event === 'Finished') updateMessage = 'Installing update…';
      });
      updateMessage = 'Restarting to finish the update…';
      await relaunch();
    } catch (e) {
      updateMessage = `Could not install update: ${String(e)}`;
      installingUpdate = false;
    }
  }

  async function refreshNotesAndFragrances() {
    entries = await backend.listFragrances();
    noteImages = await backend.listNoteImages();
  }
  async function exportCsv(kind: CsvKind) {
    error = ''; success = ''; csvReport = null;
    try {
      const path = await saveDialog({
        defaultPath: kind === 'fragrances' ? 'scentkeep-fragrances.csv' : 'scentkeep-notes.csv',
        filters: [{ name: 'CSV files', extensions: ['csv'] }]
      });
      if (!path) return;
      csvBusy = true;
      if (kind === 'fragrances') await backend.exportFragrancesCsv(path);
      else await backend.exportNotesCsv(path);
      success = `${kind === 'fragrances' ? 'Fragrances' : 'Notes'} exported to CSV.`;
    } catch (e) { error = String(e); }
    finally { csvBusy = false; }
  }
  async function importCsv(event: Event, kind: CsvKind) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    error = ''; success = ''; csvReport = null;
    if (!file.name.toLowerCase().endsWith('.csv')) {
      error = 'Excel workbooks are not supported. Save the sheet as a CSV file, then import it.';
      return;
    }
    csvBusy = true;
    try {
      const content = await file.text();
      const result = kind === 'fragrances'
        ? await backend.importFragrancesCsv(content)
        : await backend.importNotesCsv(content);
      await refreshNotesAndFragrances();
      csvReport = { kind, result };
    } catch (e) { error = String(e); }
    finally { csvBusy = false; }
  }
  async function createNote() {
    error = ''; success = '';
    try {
      await backend.createNote(newNoteName);
      noteImages = await backend.listNoteImages();
      success = `${newNoteName.trim()} added to your notes.`;
      newNoteName = '';
    } catch (e) { error = String(e); }
  }
  async function renameNote(oldName: string) {
    error = ''; success = '';
    try {
      await backend.renameNote(oldName, editedNoteName);
      await refreshNotesAndFragrances();
      success = `${oldName} renamed to ${editedNoteName.trim()}.`;
      editingNote = ''; editedNoteName = '';
    } catch (e) { error = String(e); }
  }
  async function deleteNote(name: string) {
    if (!window.confirm(`Are you sure you want to delete the note "${name}"? It will also be removed from all fragrances.`)) return;
    error = ''; success = '';
    try {
      await backend.deleteNote(name);
      await refreshNotesAndFragrances();
      success = `${name} deleted from your notes and fragrances.`;
    } catch (e) { error = String(e); }
  }
  function imageFor(note: string) {
    return noteImages.find((image) => image.name.toLocaleLowerCase() === note.toLocaleLowerCase())?.imageUrl;
  }
  async function setNoteImage(note: string) {
    const imageUrl = (noteImageDrafts[note] ?? '').trim();
    error = ''; success = '';
    savingNoteImage = note;
    try {
      await backend.setNoteImage(note, imageUrl);
      noteImages = await backend.listNoteImages();
      noteImageDrafts[note] = '';
      success = `Image link saved for ${note}.`;
    } catch (e) { error = String(e); }
    finally { savingNoteImage = ''; }
  }
  async function removeNoteImage(note: string) {
    error = ''; success = '';
    try {
      await backend.removeNoteImage(note);
      noteImages = await backend.listNoteImages();
      success = `Image removed from ${note}.`;
    } catch (e) { error = String(e); }
  }
  function suggestionsFor(kind: NoteLayer) {
    const query = noteInputs[kind].trim().toLocaleLowerCase();
    return allNotes.filter((name) =>
      name.toLocaleLowerCase().includes(query) &&
      !fragrance[kind].some((added) => added.toLocaleLowerCase() === name.toLocaleLowerCase())
    ).slice(0, 6);
  }
  function handleNoteKeydown(event: KeyboardEvent, kind: NoteLayer) {
    const suggestions = suggestionsFor(kind);
    if (event.key === 'ArrowDown' && suggestions.length) {
      event.preventDefault();
      activeNoteInput = kind;
      highlightedNoteIndex = (highlightedNoteIndex + 1) % suggestions.length;
    } else if (event.key === 'ArrowUp' && suggestions.length) {
      event.preventDefault();
      activeNoteInput = kind;
      highlightedNoteIndex = highlightedNoteIndex < 0 ? suggestions.length - 1 : (highlightedNoteIndex - 1 + suggestions.length) % suggestions.length;
    } else if (event.key === 'Enter') {
      event.preventDefault();
      addNote(kind, activeNoteInput === kind && highlightedNoteIndex >= 0 ? suggestions[highlightedNoteIndex] : undefined);
    } else if (event.key === 'Escape') {
      activeNoteInput = null;
    }
  }
  function addNote(kind: NoteLayer, selected?: string) {
    const value = (selected ?? noteInputs[kind]).trim();
    if (!value) return;
    const note = allNotes.find((name) => name.toLocaleLowerCase() === value.toLocaleLowerCase()) ?? value;
    if (fragrance[kind].some((name) => name.toLocaleLowerCase() === note.toLocaleLowerCase())) {
      duplicateNoteLayer = kind;
      activeNoteInput = null;
      return;
    }
    duplicateNoteLayer = null;
    fragrance[kind] = [...fragrance[kind], note];
    noteInputs[kind] = '';
    activeNoteInput = null;
  }
  function removeNote(kind: 'topNotes' | 'heartNotes' | 'baseNotes', index: number) {
    if (!window.confirm(`Are you sure you want to remove the note "${fragrance[kind][index]}" from this fragrance?`)) return;
    fragrance[kind] = fragrance[kind].filter((_, i) => i !== index);
  }
  function next() {
    error = '';
    if (step === 0 && (!fragrance.name.trim() || !fragrance.brand.trim())) {
      error = 'Add a fragrance name and brand to continue.';
      return;
    }
    if (step === 0 && entries.some((item) =>
      item.id !== fragrance.id &&
      item.name.trim().toLocaleLowerCase() === fragrance.name.trim().toLocaleLowerCase() &&
      item.brand.trim().toLocaleLowerCase() === fragrance.brand.trim().toLocaleLowerCase()
    )) {
      error = 'This fragrance is already in your collection.';
      return;
    }
    if (step < 2) step += 1;
  }
  function showShelf(shelf: 'Collection' | 'Wishlist' | 'Empties') { activeShelf = shelf; view = 'library'; error = ''; success = ''; mobileSidebarOpen = false; }
  function startNew() {
    fragrance = { ...blank(), shelf: activeShelf };
    mobileSidebarOpen = false;
    noteInputs = { topNotes: '', heartNotes: '', baseNotes: '' };
    activeNoteInput = null;
    duplicateNoteLayer = null;
    step = 0; concentrationMenuOpen = false; view = 'form'; error = ''; success = '';
  }
  function openDetail(item: Fragrance) {
    openCardMenuId = null; selectedId = item.id ?? null; activeShelf = item.shelf; view = 'detail'; error = ''; success = '';
  }
  function startEdit(item: Fragrance) {
    fragrance = { ...item, topNotes: [...item.topNotes], heartNotes: [...item.heartNotes], baseNotes: [...item.baseNotes] };
    noteInputs = { topNotes: '', heartNotes: '', baseNotes: '' };
    activeNoteInput = null;
    duplicateNoteLayer = null;
    step = 0; concentrationMenuOpen = false; view = 'form'; error = ''; success = ''; mobileSidebarOpen = false;
  }
  async function remove(item: Fragrance) {
    if (item.id === undefined || !window.confirm(`Are you sure you want to delete "${item.name}"?`)) return;
    error = ''; success = '';
    try {
      await backend.deleteFragrance(item.id);
      await refreshNotesAndFragrances();
      success = `${item.name} was deleted.`;
    } catch (e) { error = String(e); }
  }
  async function save() {
    error = ''; success = '';
    if (!fragrance.name.trim() || !fragrance.brand.trim()) {
      error = 'A fragrance name and brand are required.'; step = 0; return;
    }
    saving = true;
    try {
      if (fragrance.id === undefined) await backend.saveFragrance(fragrance);
      else await backend.updateFragrance(fragrance);
      await refreshNotesAndFragrances();
      const wasEditing = fragrance.id !== undefined;
      const savedShelf = fragrance.shelf; fragrance = blank(); activeShelf = savedShelf; noteInputs = { topNotes: '', heartNotes: '', baseNotes: '' };
      step = 0; view = 'library'; success = wasEditing ? 'Fragrance updated.' : 'Fragrance saved to your collection.';
    } catch (e) { error = String(e); }
    finally { saving = false; }
  }
</script>

<svelte:head>
  <title>Scentkeep — Fragrance collection</title>
  <meta name="description" content="A personal fragrance catalogue" />
</svelte:head>

<div class="app-shell" class:sidebar-collapsed={sidebarCollapsed} class:mobile-sidebar-open={mobileSidebarOpen}>
  <aside class="sidebar" aria-label="Main navigation">
    <button class="sidebar-toggle" type="button" aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"} aria-expanded={!sidebarCollapsed} onclick={() => sidebarCollapsed = !sidebarCollapsed}>{sidebarCollapsed ? "→" : "←"}</button>
    <div class="brand-mark"><img class="brand-icon" src="/aroma.svg" alt="" /><span>Scentkeep<small>THE FRAGRANCE<br />JOURNAL</small></span></div>
    <div class="sidebar-label">WORKSPACE</div>
    {#each ['Collection', 'Wishlist', 'Empties'] as shelf}
      <button title={shelf} class:active={view === 'library' && activeShelf === shelf} class="nav-link" onclick={() => showShelf(shelf as typeof activeShelf)}><span class="nav-icon"><NavIcon name={shelf === 'Wishlist' ? 'wishlist' : shelf === 'Collection' ? 'collection' : 'empties'} /></span><span class="nav-text">{shelf}</span><em>{entries.filter((item) => item.shelf === shelf).length}</em></button>
    {/each}
    <button title="Add fragrance" class:active={view === 'form'} class="nav-link" onclick={startNew}><span class="nav-icon"><NavIcon name="add" /></span><span class="nav-text">Add fragrance</span></button>
    <button title="Perfume notes" class:active={view === 'notes'} class="nav-link" onclick={() => { view = 'notes'; error = ''; success = ''; mobileSidebarOpen = false; }}><span class="nav-icon"><NavIcon name="notes" /></span><span class="nav-text">Perfume notes</span><em>{allNotes.length}</em></button>
    <div class="side-bottom"><div class="side-orb">ⓘ</div><strong>Your scent story starts here.</strong><p>Keep every note, memory and discovery in one beautiful place.</p></div>
  </aside>

  <main>
    <header class="topbar"><button class="mobile-menu" type="button" aria-label="Toggle navigation" aria-expanded={mobileSidebarOpen} onclick={() => mobileSidebarOpen = !mobileSidebarOpen}>☰</button><button class="top-back" type="button" aria-label="Back to collection" onclick={() => showShelf('Collection')}>←</button><nav class="breadcrumb" aria-label="Breadcrumb">{#if view === 'library'}<span aria-current="page">{activeShelf}</span>{:else if view === 'form' || view === 'detail'}{@const shelf = view === 'detail' ? (selectedFragrance?.shelf ?? 'Collection') : fragrance.shelf}<button onclick={() => showShelf(shelf)}>{shelf}</button><span>/</span><span aria-current="page">{view === 'form' ? (fragrance.id === undefined ? 'Add fragrance' : 'Edit fragrance') : (selectedFragrance?.name ?? 'Fragrance')}</span>{:else}<button onclick={() => showShelf('Collection')}>Collection</button><span>/</span><span aria-current="page">Perfume notes</span>{/if}</nav><div class="top-actions"><button class="update-check" aria-label={checkingUpdate ? 'Checking for updates' : 'Check for updates'} title={checkingUpdate ? 'Checking for updates' : 'Check for updates'} onclick={() => checkForUpdates()} disabled={checkingUpdate || installingUpdate}><img class="action-icon" src="/refresh.svg" alt="" />{checkingUpdate ? 'Checking…' : 'Check for updates'}</button><span class="local-badge" title="Saved locally"><span class="local-dot"></span><span class="local-text">Saved locally</span></span><button class="avatar profile-avatar" type="button" aria-label="Edit profile" title="Edit profile" onclick={editProfile}>{#if profilePicture}<img src={profilePicture} alt="" />{:else}{profileName.trim().charAt(0).toUpperCase() || 'S'}{/if}</button></div></header>
    {#if updateMessage}
      <div class="update-notice" role="status"><span>{updateMessage}</span>{#if availableUpdate && !installingUpdate}{#if canInstallUpdate}<button onclick={installUpdate}>Update and restart</button>{:else}<button onclick={() => openUrl('https://github.com/reaperhound/scentkeep/releases/latest')}>Open release page</button>{/if}{/if}</div>
    {/if}
    {#if view === 'form'}
      <div class="page-content">
        <div class="heading"><div><div class="eyebrow">YOUR FRAGRANCE LIBRARY</div><h1>{fragrance.id === undefined ? 'Add a fragrance' : 'Edit fragrance'}<span class="heading-dot">.</span></h1><p>Capture the details that make each scent unforgettable.</p></div><div class="step-count">STEP 0{step + 1} <span>/ 03</span></div></div>
        <div class="progress" aria-label="Form progress">{#each steps as label, index}<button class:current={step === index} class:done={step > index} onclick={() => { if (index < step) step = index; }}><span class="step-number">{step > index ? '✓' : `0${index + 1}`}</span><span>{label}</span></button>{#if index < 2}<div class:filled={step > index} class="connector"></div>{/if}{/each}</div>
        {#if error}<div class="message error" role="alert">{error}</div>{/if}
        <div class="form-card">
          {#if step === 0}
            <div class="card-heading"><span class="card-icon">✦</span><div><h2>The essentials</h2><p>Start with the fragrance’s identity.</p></div></div>
            <div class="form-grid"><label class="field wide"><span class="field-label-row"><span class="field-label-text">Fragrance name <b>*</b></span></span><input placeholder="e.g. Odéon Sunset" bind:value={fragrance.name} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Brand / house <b>*</b></span></span><input placeholder="e.g. Memo Paris" bind:value={fragrance.brand} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Collection</span><span class="field-hint">Optional</span></span><input placeholder="e.g. Fleurs Bohèmes" bind:value={fragrance.collection} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Release year</span></span><input type="number" min="1800" max="2100" bind:value={fragrance.year} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Perfumer</span><span class="field-hint">Optional</span></span><input placeholder="e.g. Aliénor Massenet" bind:value={fragrance.perfumer} /></label><div class="field concentration-field"><div class="field-label-row"><span class="field-label-text">Concentration</span><span class="field-hint">Optional</span></div><div class="category-picker"><button type="button" class="category-trigger" aria-haspopup="listbox" aria-expanded={concentrationMenuOpen} onclick={() => { concentrationMenuOpen = !concentrationMenuOpen; categoryMenuOpen = false; }}>{fragrance.concentration || 'Select concentration'}<span class:rotated={concentrationMenuOpen} class="category-chevron"></span></button>{#if concentrationMenuOpen}<div class="category-options concentration-options" role="listbox" aria-label="Concentration"><button type="button" role="option" aria-selected={fragrance.concentration === ''} class:selected={fragrance.concentration === ''} onclick={() => { fragrance.concentration = ''; concentrationMenuOpen = false; }}>Select concentration</button>{#each concentrations as concentration}<button type="button" role="option" aria-selected={fragrance.concentration === concentration} class:selected={fragrance.concentration === concentration} onclick={() => { fragrance.concentration = concentration; concentrationMenuOpen = false; }}>{concentration}{#if fragrance.concentration === concentration}<span>✓</span>{/if}</button>{/each}</div>{/if}</div></div></div>
            <div class="section-divider"></div><div class="field category-field"><div class="field-label-row"><span class="field-label-text">Status</span></div><div class="category-picker"><button type="button" class="category-trigger" aria-haspopup="listbox" aria-expanded={categoryMenuOpen} onclick={() => categoryMenuOpen = !categoryMenuOpen}>{fragrance.shelf}<span class:rotated={categoryMenuOpen} class="category-chevron"></span></button>{#if categoryMenuOpen}<div class="category-options" role="listbox" aria-label="Status">{#each ['Collection', 'Wishlist', 'Empties'] as shelf}<button type="button" role="option" aria-selected={fragrance.shelf === shelf} class:selected={fragrance.shelf === shelf} onclick={() => { fragrance.shelf = shelf as typeof fragrance.shelf; categoryMenuOpen = false; }}>{shelf}{#if fragrance.shelf === shelf}<span>✓</span>{/if}</button>{/each}</div>{/if}</div></div><div class="field-title">Who is this fragrance for?</div><div class="choice-grid">{#each categories as category}<button type="button" class:selected={fragrance.audience === category.value} onclick={() => fragrance.audience = category.value}><span class="choice-icon"><img src={category.icon} alt="" /></span><strong>{category.label}</strong><span class="radio-dot"></span></button>{/each}</div>
          {:else if step === 1}
            <div class="card-heading"><span class="card-icon">◈</span><div><h2>Fragrance pyramid</h2><p>Build the scent, layer by layer. Add each note separately.</p></div></div>
            {#each [{ key: 'topNotes', title: 'Top notes', hint: 'The first impression', icon: '✧', example: 'Candied apple' }, { key: 'heartNotes', title: 'Heart notes', hint: 'The heart of the scent', icon: '❀', example: 'Caramel' }, { key: 'baseNotes', title: 'Base notes', hint: 'The lasting impression', icon: '◆', example: 'Sandalwood' }] as group}
              {@const kind = group.key as 'topNotes' | 'heartNotes' | 'baseNotes'}
              <div class="note-section">
                <div class="note-title"><span class="note-icon">{group.icon}</span><div><h3>{group.title}</h3><p>{group.hint}</p></div></div>
                <div class="note-entry">
                  <div class="note-input-wrap">
                    <input
                      aria-label={`Add ${group.title.toLowerCase()}`}
                      role="combobox"
                      aria-autocomplete="list"
                      aria-expanded={activeNoteInput === kind && suggestionsFor(kind).length > 0}
                      aria-controls={activeNoteInput === kind && suggestionsFor(kind).length > 0 ? `suggestions-${kind}` : undefined}
                      aria-activedescendant={activeNoteInput === kind && highlightedNoteIndex >= 0 ? `suggestion-${kind}-${highlightedNoteIndex}` : undefined}
                      autocomplete="off"
                      placeholder={`e.g. ${group.example}`}
                      bind:value={noteInputs[kind]}
                      onfocus={() => { activeNoteInput = kind; highlightedNoteIndex = -1; }}
                      oninput={() => { activeNoteInput = kind; highlightedNoteIndex = -1; duplicateNoteLayer = null; }}
                      onblur={() => activeNoteInput = null}
                      onkeydown={(event) => handleNoteKeydown(event, kind)}
                    />
                    {#if activeNoteInput === kind && suggestionsFor(kind).length > 0}
                      <div class="note-suggestions" id={`suggestions-${kind}`} role="listbox" aria-label="Saved notes">
                        <div class="suggestions-heading">SAVED NOTES</div>
                        {#each suggestionsFor(kind) as suggestion, index}
                          <button
                            type="button"
                            id={`suggestion-${kind}-${index}`}
                            role="option"
                            aria-selected={highlightedNoteIndex === index}
                            class:highlighted={highlightedNoteIndex === index}
                            tabindex="-1"
                            onpointerdown={(event) => event.preventDefault()}
                            onmouseenter={() => highlightedNoteIndex = index}
                            onclick={() => addNote(kind, suggestion)}
                          >{suggestion}</button>
                        {/each}
                      </div>
                    {/if}
                  </div>
                  <button type="button" onclick={() => addNote(kind)}>+ Add note</button>
                </div>
                {#if duplicateNoteLayer === kind}<div class="note-error" role="alert">This note is already in the {group.title.toLowerCase()}.</div>{/if}
                <div class="note-chips">{#each fragrance[kind] as note, index}<button type="button" title={`Remove ${note}`} onclick={() => removeNote(kind, index)}>{note} <span>×</span></button>{/each}{#if fragrance[kind].length === 0}<span class="empty-hint">No notes added yet</span>{/if}</div>
              </div>
            {/each}
          {:else}
            <div class="card-heading"><span class="card-icon">☷</span><div><h2>Finishing touches</h2><p>Add context and a little more character.</p></div></div>
            <label class="field"><span class="field-label-row"><span class="field-label-text">Description</span><span class="field-hint">Optional</span></span><textarea rows="5" placeholder="What makes this fragrance special?" bind:value={fragrance.description}></textarea></label>
            <div class="form-grid details"><label class="field"><span class="field-label-row"><span class="field-label-text">Bottle rating</span><span class="field-hint">0–10</span></span><input type="number" min="0" max="10" step="0.1" placeholder="e.g. 7.6" bind:value={fragrance.bottleRating} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Number of ratings</span><span class="field-hint">Optional</span></span><input type="number" min="0" placeholder="e.g. 5" bind:value={fragrance.ratingCount} /></label></div>
            <div class="form-grid details"><label class="field"><span class="field-label-row"><span class="field-label-text">Bottles owned</span></span><input type="number" min="0" step="1" bind:value={fragrance.bottlesOwned} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Bottles finished</span></span><input type="number" min="0" step="1" bind:value={fragrance.bottlesFinished} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Price bought</span><span class="field-hint">Optional</span></span><input type="number" min="0" step="0.01" bind:value={fragrance.priceBought} /></label>{#if fragrance.shelf === 'Wishlist'}<label class="field"><span class="field-label-row"><span class="field-label-text">Current price</span><span class="field-hint">Optional</span></span><input type="number" min="0" step="0.01" bind:value={fragrance.currentPrice} /></label>{/if}</div><label class="favourite-field"><input type="checkbox" bind:checked={fragrance.favourite} /> Favourite</label><label class="field"><span class="field-label-row"><span class="field-label-text">Image URL</span><span class="field-hint">Optional</span></span><input type="url" placeholder="https://example.com/fragrance.jpg" bind:value={fragrance.imageUrl} /><small>A link to the bottle image. Leave blank to use a placeholder.</small></label>
            <div class="summary"><div class="summary-icon">✦</div><div><strong>Ready for your collection</strong><p>{fragrance.name || 'Your fragrance'} by {fragrance.brand || 'your chosen house'} · {fragrance.year} · {fragrance.audience}</p></div></div>
          {/if}
        </div>
        <div class="form-footer"><span>Fields marked <b>*</b> are required</span><div>{#if step > 0}<button class="back-button" onclick={() => { step -= 1; error = ''; }}>← Back</button>{/if}{#if step < 2}<button class="primary-button" onclick={next}>Continue <span>→</span></button>{:else}<button class="primary-button" disabled={saving} onclick={save}>{saving ? 'Saving…' : (fragrance.id === undefined ? 'Save fragrance' : 'Save changes')} <span>↗</span></button>{/if}</div></div>
      </div>
    {:else if view === 'detail' && selectedFragrance}
      <div class="page-content detail-page">
        <button class="detail-back" onclick={() => view = 'library'}>← Back to collection</button>
        <div class="detail-layout">
          <div class="detail-left"><div class="detail-image">{#if selectedFragrance.imageUrl}<img src={selectedFragrance.imageUrl} alt={selectedFragrance.name} />{:else}<div class="detail-placeholder"><span>✳</span><small>NO BOTTLE IMAGE</small></div>{/if}</div><div class="detail-image-caption">{selectedFragrance.brand} · {selectedFragrance.year}</div><button class="detail-edit" onclick={() => startEdit(selectedFragrance)}>✎ Edit fragrance</button></div>
          <div class="detail-main"><div class="detail-kicker">{selectedFragrance.collection || 'FRAGRANCE COLLECTION'}</div><h1>{selectedFragrance.name}<span class="heading-dot">.</span></h1><div class="detail-brand">{selectedFragrance.brand}<span class="year-pill">{selectedFragrance.year}</span></div><div class="audience-pill">{selectedFragrance.audience}</div>{#if selectedFragrance.concentration}<div class="audience-pill concentration-pill">{selectedFragrance.concentration}</div>{/if}<div class="ownership-summary"><span>{selectedFragrance.shelf}</span><span>Owned: {selectedFragrance.bottlesOwned}</span><span>Finished: {selectedFragrance.bottlesFinished}</span>{#if selectedFragrance.favourite}<span>★ Favourite</span>{/if}{#if selectedFragrance.priceBought !== null}<span>Price bought: {selectedFragrance.priceBought.toFixed(2)}</span>{/if}{#if selectedFragrance.shelf === 'Wishlist' && selectedFragrance.currentPrice !== null}<span>Current price: {selectedFragrance.currentPrice.toFixed(2)}</span>{/if}</div>{#if selectedFragrance.description}<p class="detail-description">{selectedFragrance.description}</p>{/if}
            <section class="detail-section"><div class="detail-section-title"><span>◈</span><h2>Fragrance pyramid</h2></div>
              {#each [{ title: 'Top notes', notes: selectedFragrance.topNotes, icon: '✧' }, { title: 'Heart notes', notes: selectedFragrance.heartNotes, icon: '❀' }, { title: 'Base notes', notes: selectedFragrance.baseNotes, icon: '◆' }] as group}
                <div class="pyramid-row"><div class="pyramid-label"><span>{group.icon}</span>{group.title}</div><div class="pyramid-notes">{#each group.notes as note}<div class="pyramid-note"><div class="pyramid-note-image">{#if imageFor(note)}<img src={imageFor(note)} alt="" />{:else}<span>✦</span>{/if}</div><span>{note}</span></div>{/each}{#if group.notes.length === 0}<span class="no-notes">No notes recorded</span>{/if}</div></div>
              {/each}
            </section>
            <div class="detail-meta"><section><div class="detail-section-title"><span>✦</span><h2>Perfumer</h2></div><p>{selectedFragrance.perfumer || 'Not recorded'}</p></section><section><div class="detail-section-title"><span>★</span><h2>Bottle rating</h2></div>{#if selectedFragrance.bottleRating !== null}<div class="detail-rating"><strong>{selectedFragrance.bottleRating.toFixed(1)}</strong><span>/ 10</span></div><div class="rating-bar"><div style={`width: ${selectedFragrance.bottleRating * 10}%`}></div></div>{#if selectedFragrance.ratingCount !== null}<small>{selectedFragrance.ratingCount} {selectedFragrance.ratingCount === 1 ? 'rating' : 'ratings'}</small>{/if}{:else}<p>Not rated yet</p>{/if}</section></div>
          </div>
        </div>
      </div>
    {:else if view === 'notes'}
      <div class="page-content">
        <div class="heading"><div><div class="eyebrow">YOUR SCENT INDEX</div><h1>Perfume notes<span class="heading-dot">.</span></h1><p>See every note in your collection and give it an image.</p></div>
          <div class="csv-actions"><button class="back-button" disabled={csvBusy} onclick={() => noteCsvInput?.click()}><img class="action-icon" src="/import.svg" alt="" />Import CSV</button><button class="back-button" disabled={csvBusy} onclick={() => exportCsv('notes')}><img class="action-icon" src="/export.svg" alt="" />Export CSV</button><input type="file" accept=".csv,.xlsx,.xls,text/csv" bind:this={noteCsvInput} onchange={(event) => importCsv(event, 'notes')} hidden /></div>
        </div>
        {#if success}<div class="message success">{success}</div>{/if}{#if error}<div class="message error" role="alert">{error}</div>{/if}
        {#if csvReport?.kind === 'notes'}<div class="message import-result" role="status"><strong>Imported {csvReport.result.imported} {csvReport.result.imported === 1 ? 'note' : 'notes'}.</strong>{#if csvReport.result.duplicates.length}<p>Skipped {csvReport.result.duplicates.length} duplicate {csvReport.result.duplicates.length === 1 ? 'note' : 'notes'}:</p><ul>{#each csvReport.result.duplicates as duplicate}<li>{duplicate}</li>{/each}</ul>{/if}</div>{/if}
        <form class="new-note-form" onsubmit={(event) => { event.preventDefault(); createNote(); }}><input aria-label="New note name" placeholder="Add a new perfume note..." bind:value={newNoteName} /><button type="submit" disabled={!newNoteName.trim()}>+ Add note</button></form>
        <div class="library-toolbar"><div class="search-box">⌕ <input aria-label="Search notes" placeholder="Search perfume notes..." bind:value={noteSearch} /></div><span>{visibleNotes.length} {visibleNotes.length === 1 ? 'note' : 'notes'}</span></div>
        {#if visibleNotes.length === 0}
          <div class="empty-state"><div>◈</div><h2>{allNotes.length === 0 ? 'No notes in your collection yet' : 'No notes found'}</h2><p>{allNotes.length === 0 ? 'Add a note above, or add notes to a fragrance.' : 'Try another search term.'}</p>{#if allNotes.length === 0}<button class="primary-button" onclick={startNew}>Add a fragrance →</button>{/if}</div>
        {:else}
          <div class="notes-grid">{#each visibleNotes as note}<article class="note-card"><div class="note-art">{#if imageFor(note)}<img src={imageFor(note)} alt={note} />{:else}<span>✧</span>{/if}</div><div class="note-card-body">{#if editingNote === note}<form class="rename-note-form" onsubmit={(event) => { event.preventDefault(); renameNote(note); }}><input aria-label={`Rename ${note}`} bind:value={editedNoteName} /><button type="submit" disabled={!editedNoteName.trim()}>Save</button><button type="button" onclick={() => editingNote = ''}>Cancel</button></form>{:else}<h2>{note}</h2>{/if}<p>Used in {noteImages.find((image) => image.name === note)?.usageCount ?? 0} fragrances</p><form class="note-image-form" onsubmit={(event) => { event.preventDefault(); setNoteImage(note); }}><input type="url" aria-label={`Image link for ${note}`} placeholder="https://example.com/note.jpg" value={noteImageDrafts[note] ?? ''} oninput={(event) => noteImageDrafts[note] = event.currentTarget.value} /><div><button type="submit" disabled={savingNoteImage === note || !noteImageDrafts[note]?.trim()}>{savingNoteImage === note ? 'Saving…' : imageFor(note) ? 'Update image link' : '+ Add image link'}</button>{#if imageFor(note)}<button type="button" onclick={() => removeNoteImage(note)}>Remove image</button>{/if}</div></form><div class="note-manage-actions"><button onclick={() => { editingNote = note; editedNoteName = note; }}>Rename</button><button class="danger" onclick={() => deleteNote(note)}>Delete note</button></div></div></article>{/each}</div>
        {/if}
      </div>
    {:else}
      <div class="page-content collection-page">
        <div class="heading collection-heading"><div><div class="eyebrow">YOUR FRAGRANCE LIBRARY</div><h1>{activeShelf}</h1><p>A home for every scent worth remembering.</p></div></div>
        {#if success}<div class="message success">{success}</div>{/if}{#if error}<div class="message error">{error}</div>{/if}
        {#if csvReport?.kind === 'fragrances'}<div class="message import-result" role="status"><strong>Imported {csvReport.result.imported} {csvReport.result.imported === 1 ? 'fragrance' : 'fragrances'}.</strong>{#if csvReport.result.duplicates.length}<p>Skipped {csvReport.result.duplicates.length} duplicate {csvReport.result.duplicates.length === 1 ? 'fragrance' : 'fragrances'}:</p><ul>{#each csvReport.result.duplicates as duplicate}<li>{duplicate}</li>{/each}</ul>{/if}</div>{/if}
        <div class="collection-controls"><div class="collection-search"><label class="search-box"><span aria-hidden="true">⌕</span><input aria-label="Search fragrances" placeholder="Search fragrances or brands..." bind:value={search} /></label><button class="filter-button" type="button" aria-label="Show owned fragrances" title="Show owned fragrances" onclick={() => collectionFilter = collectionFilter === 'Owned' ? 'All' : 'Owned'}>☷</button></div><div class="csv-actions"><button class="back-button" disabled={csvBusy} onclick={() => fragranceCsvInput?.click()}><img class="action-icon" src="/import.svg" alt="" /><span>Import CSV</span></button><button class="back-button" disabled={csvBusy} onclick={() => exportCsv('fragrances')}><img class="action-icon" src="/export.svg" alt="" /><span>Export CSV</span></button><button class="primary-button" onclick={startNew}><img class="action-icon" src="/add-circle.svg" alt="" /><span>Add Fragrance</span></button><input type="file" accept=".csv,.xlsx,.xls,text/csv" bind:this={fragranceCsvInput} onchange={(event) => importCsv(event, 'fragrances')} hidden /></div></div>
        <div class="collection-subbar"><div class="shelf-filters">{#if activeShelf === 'Collection'}{#each ['All', 'Owned', 'Wishlist', 'Empties'] as filter}<button type="button" class:active={collectionFilter === filter} onclick={() => collectionFilter = filter as typeof collectionFilter}><span>{#if filter === 'Wishlist'}<img class="filter-icon" src="/wallet.svg" alt="" />{:else if filter === 'All'}<img class="filter-icon" src="/app.svg" alt="" />{:else if filter === 'Owned'}<img class="filter-icon" src="/coin-collecting.svg" alt="" />{:else}♻{/if}</span>{filter} ({filter === 'All' ? entries.length : filter === 'Owned' ? entries.filter((item) => item.shelf === 'Collection').length : entries.filter((item) => item.shelf === filter).length})</button>{/each}{/if}</div><div class="collection-view"><span>{visibleEntries.length} {visibleEntries.length === 1 ? 'fragrance' : 'fragrances'}</span><div class="layout-switch"><button type="button" class:active={collectionLayout === 'grid'} aria-label="Grid view" onclick={() => collectionLayout = 'grid'}>⌘</button><button type="button" class:active={collectionLayout === 'list'} aria-label="List view" onclick={() => collectionLayout = 'list'}>☰</button></div></div></div>
        {#if visibleEntries.length === 0}<div class="empty-state"><div>✳</div><h2>{entries.length === 0 ? 'Your collection is waiting' : 'No fragrances found'}</h2><p>{entries.length === 0 ? 'Add your first fragrance to start your scent journal.' : 'Try another search or filter.'}</p>{#if entries.length === 0}<button class="primary-button" onclick={startNew}>Add your first fragrance →</button>{/if}</div>
        {:else}<div class="fragrance-grid" class:list-layout={collectionLayout === 'list'}>{#each visibleEntries as item}<article class="fragrance-card" role="link" tabindex="0" aria-label={`View details for ${item.name}`} onclick={() => openDetail(item)} onkeydown={(event) => { if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); openDetail(item); } }}><div class="fragrance-art">{#if item.imageUrl}<img src={item.imageUrl} alt={item.name} />{:else}<span class="bottle-placeholder">✳</span>{/if}<div class="card-menu-wrap"><button class="card-menu" type="button" aria-label={`Actions for ${item.name}`} aria-expanded={openCardMenuId === item.id} onclick={(event) => { event.stopPropagation(); openCardMenuId = openCardMenuId === item.id ? null : (item.id ?? null); }}>···</button>{#if openCardMenuId === item.id}<div class="card-dropdown" role="menu" aria-label={`Actions for ${item.name}`} onclick={(event) => event.stopPropagation()}><button type="button" role="menuitem" onclick={() => { openCardMenuId = null; startEdit(item); }}>✎ Edit</button><button type="button" role="menuitem" class="delete-action" onclick={() => { openCardMenuId = null; remove(item); }}>♜ Delete</button></div>{/if}</div></div><div class="fragrance-info"><h2>{item.name}</h2><div class="listing-meta"><img class="sex-icon" src={sexIconFor(item.audience)} alt={item.audience} title={item.audience} /><span class="concentration-label">{item.concentration || 'Concentration not set'}</span></div></div></article>{/each}</div>{/if}
      </div>
    {/if}
  </main>
</div>

{#if profileDialogOpen}<div class="dialog-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) profileDialogOpen = false; }}><dialog open class="profile-dialog" aria-labelledby="profile-title"><button class="dialog-close" type="button" aria-label="Close" onclick={() => profileDialogOpen = false}>×</button><div class="eyebrow">YOUR ACCOUNT</div><h2 id="profile-title">Edit profile</h2><div class="profile-picture-preview">{#if draftProfilePicture}<img src={draftProfilePicture} alt="Profile preview" />{:else}<span>{draftProfileName.trim().charAt(0).toUpperCase() || 'S'}</span>{/if}</div><label class="field"><span class="field-label-row"><span class="field-label-text">Your name</span></span><input maxlength="60" bind:value={draftProfileName} placeholder="Your name" /></label><div class="profile-url-row"><input type="url" aria-label="Profile picture URL" placeholder="Or paste an image URL" bind:value={profilePictureUrl} /><button type="button" disabled={!profilePictureUrl.trim()} onclick={() => { draftProfilePicture = profilePictureUrl.trim(); }}>Use link</button></div>{#if draftProfilePicture}<button type="button" class="clear-picture" onclick={() => draftProfilePicture = ''}>Remove picture</button>{/if}<div class="dialog-actions"><button class="back-button" type="button" onclick={() => profileDialogOpen = false}>Cancel</button><button class="primary-button" type="button" onclick={saveProfile}>Save profile</button></div></dialog></div>{/if}

<style>
  :global(*){box-sizing:border-box} :global(body){margin:0;background:#fff;color:#182941;font-family:Inter,ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;font-size:14px} :global(button),:global(input),:global(textarea){font:inherit} :global(button),:global(a[href]),:global(select),:global([role="button"]){cursor:pointer} :global(button:disabled){cursor:not-allowed}
  .app-shell{display:flex;min-height:100vh}.sidebar{width:248px;flex:none;background:#fff;border-right:1px solid #e9eff7;padding:30px 18px;display:flex;flex-direction:column}.brand-mark{display:flex;align-items:center;gap:10px;padding:0 12px 53px;color:#2563ad;font-size:22px;font-weight:800;letter-spacing:-1px}.brand-icon{width:39px;height:39px;border-radius:12px;background:#438feb;color:white;display:grid;place-items:center;font-size:25px}.brand-mark small{display:block;font-size:8px;letter-spacing:2px;color:#8ca1bd;font-weight:700;margin-top:2px}.sidebar-label{font-size:10px;font-weight:800;color:#a8b7cb;letter-spacing:1.6px;padding:0 15px 13px}.nav-link{display:flex;align-items:center;gap:14px;width:100%;border:0;background:transparent;color:#71829b;text-align:left;padding:14px 16px;border-radius:11px;font-weight:650;margin-bottom:5px}.nav-link>span{font-size:19px;width:20px;text-align:center}.nav-link.active{background:#eaf3ff;color:#3989e9}.nav-link em{font-size:11px;font-style:normal;margin-left:auto;color:#87a5ca}.side-bottom{margin-top:auto;background:#eaf4ff;border-radius:15px;padding:20px;color:#345577}.side-orb{width:35px;height:35px;border-radius:50%;background:#d4e9ff;display:grid;place-items:center;color:#408be5;font-size:20px;margin-bottom:13px}.side-bottom strong{font-size:13px}.side-bottom p{font-size:11px;line-height:1.7;color:#8199b5;margin-bottom:0}main{min-width:0;flex:1}.topbar{height:76px;background:#fff;border-bottom:1px solid #e9eff7;display:flex;align-items:center;justify-content:space-between;padding:0 48px}.breadcrumb{color:#7d91ab;font-size:12px;font-weight:650}.breadcrumb span{padding:0 12px;color:#c5d0dd}.top-actions{display:flex;align-items:center;gap:20px}.local-badge{color:#7890a6;font-size:11px}.local-badge span{display:inline-block;width:7px;height:7px;border-radius:50%;background:#58c99e;margin-right:5px}.avatar{width:32px;height:32px;background:#eaf2ff;color:#428ce7;border-radius:50%;display:grid;place-items:center;font-weight:800}.page-content{max-width:990px;margin:0 auto;padding:47px 38px 80px}.heading{display:flex;align-items:end;justify-content:space-between;gap:20px;margin-bottom:38px}.eyebrow{color:#4e9bf1;font-size:10px;letter-spacing:2px;font-weight:800;margin-bottom:11px}h1{font-family:Georgia,serif;font-size:39px;font-weight:500;letter-spacing:-1.5px;margin:0 0 9px}.heading-dot{color:#438ff0}.heading p{margin:0;color:#8a9db4}.step-count{font-size:11px;color:#438ff0;font-weight:800;letter-spacing:1.5px;padding-bottom:4px}.step-count span{color:#b4c4d6}.progress{display:flex;align-items:center;margin-bottom:27px}.progress button{border:0;background:transparent;color:#9cb2ca;display:flex;align-items:center;gap:9px;white-space:nowrap;font-size:11px;font-weight:700;padding:0}.progress button.current{color:#3187ee}.progress button.done{color:#50a99a}.step-number{width:32px;height:32px;border:1px solid #d9e8f8;border-radius:50%;display:grid;place-items:center;background:#fff;color:#98b5d4}.current .step-number{background:#448ff0;border-color:#448ff0;color:white;box-shadow:0 3px 10px #448ff050}.done .step-number{background:#e5f9f1;border-color:#c6ebde;color:#43ae8c}.connector{height:1px;background:#dbe8f6;flex:1;margin:0 14px}.connector.filled{background:#82c9b3}.form-card{background:white;border:1px solid #e6edf7;border-radius:17px;box-shadow:0 12px 40px #2b6dae0a;padding:34px 39px 38px}.card-heading{display:flex;align-items:center;gap:15px;margin-bottom:31px}.card-icon{width:42px;height:42px;border-radius:12px;background:#eaf3ff;color:#438fec;display:grid;place-items:center;font-size:22px}.card-heading h2{font-family:Georgia,serif;font-size:23px;font-weight:500;margin:0 0 4px}.card-heading p{margin:0;color:#9aabc0;font-size:12px}.form-grid{display:grid;grid-template-columns:1fr 1fr;gap:23px 25px}.field{display:flex;flex-direction:column;gap:9px;font-size:12px;font-weight:700;color:#34445c}.field.wide{grid-column:span 2}.field b,.form-footer b{color:#fa8c9d}.field>span{float:right;color:#b7c5d4;font-size:10px;font-weight:500;margin-left:auto}.field input,.field textarea,.note-entry input{width:100%;border:1px solid #dae5f1;border-radius:7px;padding:12px 14px;outline:none;background:#fff;color:#263b58;font-weight:400}.field input:focus,.field textarea:focus,.note-entry input:focus,.search-box input:focus{border-color:#4d97ee;box-shadow:0 0 0 3px #4d97ee1c}.field input::placeholder,.field textarea::placeholder,.note-entry input::placeholder{color:#b4c2d1}.field textarea{resize:vertical}.field small{font-size:10px;font-weight:400;color:#a0aec0}.section-divider{height:1px;background:#edf2f7;margin:32px 0 26px}.field-title{font-size:12px;font-weight:700;margin-bottom:15px}.choice-grid{display:grid;grid-template-columns:repeat(3,1fr);gap:13px}.choice-grid button{position:relative;border:1px solid #dfeaf5;background:white;border-radius:10px;padding:18px 15px;display:flex;align-items:center;gap:12px;color:#7188a3}.choice-grid button.selected{border-color:#4b9af3;background:#f0f7ff;box-shadow:0 0 0 2px #4b9af318;color:#2f7cd8}.choice-icon{width:32px;height:32px;background:#ecf4ff;color:#438eec;border-radius:50%;display:grid;place-items:center;font-size:19px}.choice-grid strong{font-size:12px}.radio-dot{margin-left:auto;width:14px;height:14px;border:1px solid #c8d9ec;border-radius:50%}.selected .radio-dot{border:4px solid #478fed}.form-footer{display:flex;justify-content:space-between;align-items:center;margin-top:23px;color:#a6b5c7;font-size:11px}.form-footer>div{display:flex;gap:11px}.primary-button,.back-button{border:0;border-radius:8px;padding:13px 24px;font-weight:750;font-size:12px}.primary-button{background:#458ff0;color:white;box-shadow:0 5px 13px #458ff03d}.primary-button:hover{background:#2c7bdd}.primary-button:disabled{opacity:.6;cursor:wait}.primary-button span{margin-left:19px}.back-button{background:white;border:1px solid #dce8f5;color:#7e91aa}.message{padding:12px 16px;border-radius:8px;margin-bottom:18px;font-size:12px}.message.error{background:#fff1f2;color:#bd405a}.message.success{background:#e9f9ef;color:#27865a}.note-section{border:1px solid #e4edf7;border-radius:12px;padding:20px;margin-bottom:17px}.note-title{display:flex;align-items:center;gap:11px;margin-bottom:15px}.note-icon{width:36px;height:36px;border-radius:50%;background:#eaf4ff;color:#4d97e9;display:grid;place-items:center;font-size:19px}.note-title h3{margin:0 0 3px;font-size:13px}.note-title p{margin:0;font-size:11px;color:#a0b0c4}.note-entry{display:flex;gap:10px}.note-entry > button{white-space:nowrap;border:0;border-radius:7px;background:#448ff0;color:white;padding:0 17px;font-size:11px;font-weight:750}.note-chips{display:flex;flex-wrap:wrap;gap:8px;margin-top:13px}.note-chips button,.mini-notes span{border:0;border-radius:20px;background:#e8f3ff;color:#4e91dd;padding:6px 10px;font-size:11px}.note-chips button span{font-size:15px;margin-left:5px}.empty-hint{font-size:11px;color:#becbda}.details{margin:23px 0}.summary{display:flex;align-items:center;gap:14px;margin-top:28px;padding:18px;border:1px solid #d9eafa;border-radius:10px;background:#f4f9ff}.summary-icon{font-size:23px;color:#468ee7}.summary strong{font-size:12px}.summary p{font-size:11px;color:#8ba0b8;margin:4px 0 0}.library-toolbar{display:flex;align-items:center;justify-content:space-between;margin-bottom:24px;color:#9cadbf;font-size:11px}.search-box{background:white;border:1px solid #e2eaf3;border-radius:9px;padding:0 13px;color:#9db4d0;font-size:21px;display:flex;align-items:center;width:330px}.search-box input{border:0;outline:none;padding:12px 8px;width:100%;font-size:12px}.empty-state{background:white;border:1px solid #e6edf7;border-radius:16px;text-align:center;padding:80px 20px}.empty-state>div{font-size:50px;color:#96c5f7}.empty-state h2{font-family:Georgia,serif;font-size:25px;font-weight:500;margin:10px}.empty-state p{color:#92a5b9;font-size:12px;margin-bottom:24px}.fragrance-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(245px,1fr));gap:18px}.fragrance-card{background:white;border-radius:13px;overflow:hidden}.fragrance-art{height:170px;background:#fff;display:grid;place-items:center;color:#a5c7ed;font-size:65px}.fragrance-art img{max-width:100%;max-height:100%;object-fit:contain}.fragrance-info{padding:19px}.fragrance-kicker{font-size:9px;letter-spacing:1.3px;text-transform:uppercase;color:#7ca7d5;font-weight:800}.fragrance-info h2{font-family:Georgia,serif;font-size:21px;font-weight:500;margin:8px 0 2px}.fragrance-info p{color:#95a7ba;font-size:12px;margin:0}.mini-notes{display:flex;flex-wrap:wrap;gap:5px;margin-top:15px}@media(max-width:800px){.sidebar{width:64px;padding:25px 8px}.brand-mark{padding:0 4px 45px}.brand-mark>span:last-child,.sidebar-label,.nav-link:not(.active),.nav-link.active{font-size:0}.nav-link{justify-content:center;padding:12px}.nav-link>span{font-size:21px}.nav-link em,.side-bottom{display:none}.topbar{padding:0 22px}.page-content{padding:35px 20px}.progress button>span:last-child{display:none}.progress{justify-content:center}.connector{max-width:75px}.choice-grid{gap:7px}.choice-grid button{padding:12px 7px;gap:5px;flex-direction:column}.radio-dot{display:none}}@media(max-width:550px){.sidebar{display:none}.heading h1{font-size:31px}.step-count{display:none}.form-card{padding:25px 20px}.form-grid{grid-template-columns:1fr}.field.wide{grid-column:auto}.topbar{height:60px}.search-box{width:220px}}
  
  .notes-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(190px,1fr));gap:18px}.note-card{background:white;border-radius:13px;overflow:hidden}.note-art{height:155px;background:#fff;display:grid;place-items:center;color:#a5c7ed;font-size:56px}.note-art img{width:100%;height:100%;object-fit:cover}.note-card-body{padding:16px}.note-card-body h2{font-family:Georgia,serif;font-size:19px;font-weight:500;margin:0 0 4px}.note-card-body p{color:#9aaec3;font-size:11px;margin:0 0 15px}.note-image-form{border-top:1px solid #edf2f7;padding-top:12px;display:flex;flex-direction:column;gap:9px}.note-image-form input{border:1px solid #dae5f1;border-radius:6px;padding:9px 10px;font-size:11px;width:100%;min-width:0;outline:none}.note-image-form input:focus{border-color:#4d97ee}.note-image-form>div{display:flex;align-items:center;justify-content:space-between;gap:6px}.note-image-form button{border:0;background:none;color:#3789e9;font-size:10px;font-weight:750;padding:0}.note-image-form button:last-child:not(:first-child){color:#d56877}.note-image-form button:disabled{opacity:.55}
  .detail-page{max-width:1120px}.detail-back{border:0;background:none;color:#658fbd;font-size:12px;font-weight:700;padding:0;margin-bottom:27px}.detail-layout{display:grid;grid-template-columns:minmax(250px,350px) minmax(0,1fr);gap:48px}.detail-left{min-width:0}.detail-image{height:390px;border:1px solid #e5edf6;border-radius:16px;background:#fff;display:grid;place-items:center;overflow:hidden}.detail-image img{width:100%;height:100%;object-fit:contain;padding:20px}.detail-placeholder{display:flex;flex-direction:column;align-items:center;gap:9px;color:#aac7e9}.detail-placeholder span{font-size:110px;line-height:1}.detail-placeholder small{letter-spacing:2px;font-size:9px;font-weight:800}.detail-image-caption{text-align:center;color:#9aabc0;font-size:11px;margin:13px 0 22px}.detail-edit{width:100%;border:1px solid #d9e8f8;background:#fff;color:#4c90df;border-radius:9px;padding:12px;font-weight:700;font-size:12px}.detail-main{min-width:0;padding-top:8px}.detail-kicker{font-size:11px;color:#5798e0;font-weight:800;letter-spacing:1.8px;text-transform:uppercase;margin-bottom:8px}.detail-main h1{font-size:42px;margin-bottom:4px}.detail-brand{font-size:19px;color:#899fb9}.year-pill{display:inline-block;margin-left:10px;background:#edf3f9;color:#8498af;border-radius:6px;padding:4px 7px;vertical-align:middle;font-size:11px}.audience-pill{display:inline-block;border-radius:20px;background:#eaf4ff;color:#498ddd;padding:6px 12px;margin-top:16px;font-size:11px;font-weight:750}.detail-description{line-height:1.75;color:#71869f;margin:24px 0 30px;white-space:pre-wrap}.detail-section{border-top:1px solid #e6eef7;padding-top:23px;margin-top:26px}.detail-section-title{display:flex;align-items:center;gap:10px;margin-bottom:19px}.detail-section-title>span{width:29px;height:29px;border-radius:8px;background:#eaf4ff;color:#4b91e8;display:grid;place-items:center;font-size:17px}.detail-section-title h2{font-family:Georgia,serif;font-size:21px;font-weight:500;margin:0}.pyramid-row{padding:13px 0 20px;border-bottom:1px solid #edf2f7}.pyramid-label{font-size:10px;text-transform:uppercase;letter-spacing:1.4px;font-weight:800;color:#8498ae;display:flex;align-items:center;gap:8px;margin-bottom:12px}.pyramid-label>span{color:#5ca0ee;font-size:18px}.pyramid-notes{display:flex;flex-wrap:wrap;gap:15px}.pyramid-note{display:flex;align-items:center;gap:7px;color:#438ad7;font-size:12px;font-weight:650}.pyramid-note-image{width:30px;height:30px;border-radius:50%;background:#e9f3ff;display:grid;place-items:center;overflow:hidden;color:#8dbcec}.pyramid-note-image img{width:100%;height:100%;object-fit:cover}.no-notes{font-size:11px;color:#bdcad8}.detail-meta{display:grid;grid-template-columns:1fr 1fr;gap:25px;margin-top:32px}.detail-meta section{border:1px solid #e6edf7;border-radius:12px;background:white;padding:19px}.detail-meta .detail-section-title{margin-bottom:10px}.detail-meta .detail-section-title h2{font-size:18px}.detail-meta p{color:#6797cc;margin:0;font-size:13px}.detail-rating strong{font-size:30px;color:#48ad8b}.detail-rating span{font-size:12px;color:#a5b5c5;margin-left:4px}.detail-meta small{color:#a0afbf;font-size:10px}@media(max-width:850px){.detail-layout{grid-template-columns:1fr;gap:25px}.detail-image{height:300px}.detail-left{max-width:500px;width:100%;margin:auto}.detail-meta{grid-template-columns:1fr 1fr}}@media(max-width:550px){.detail-meta{grid-template-columns:1fr}.detail-main h1{font-size:34px}}
  .new-note-form{display:flex;gap:10px;background:#fff;border:1px solid #e3edf7;border-radius:12px;padding:13px;margin-bottom:23px}.new-note-form input,.rename-note-form input{border:1px solid #dae5f1;border-radius:7px;padding:11px 12px;outline:none;min-width:0;flex:1}.new-note-form input:focus,.rename-note-form input:focus{border-color:#4d97ee}.new-note-form button,.rename-note-form button[type=submit]{border:0;border-radius:7px;background:#448ff0;color:white;font-size:11px;font-weight:750;padding:0 17px}.new-note-form button:disabled,.rename-note-form button:disabled{opacity:.55}.rename-note-form{display:flex;gap:5px;flex-wrap:wrap;margin-bottom:8px}.rename-note-form input{width:100%;flex-basis:100%}.rename-note-form button{border:0;background:transparent;color:#7f96ae;font-size:11px;padding:6px}.note-manage-actions{display:flex;gap:12px;flex-wrap:wrap;border-top:1px solid #edf2f7;margin-top:12px;padding-top:11px;align-items:center}.note-manage-actions button{border:0;background:transparent;padding:0;color:#6f92b8;font-size:11px;font-weight:700}.note-manage-actions button.danger{color:#d56877}
  .note-input-wrap{position:relative;flex:1;min-width:0}
  .note-suggestions{position:absolute;top:calc(100% + 6px);left:0;right:0;z-index:20;max-height:236px;overflow-y:auto;background:#fff;border:1px solid #dce8f5;border-radius:9px;box-shadow:0 12px 28px #234e8424;padding:6px}
  .suggestions-heading{padding:7px 10px 5px;color:#9aacc0;font-size:9px;font-weight:800;letter-spacing:1.3px}
  .note-suggestions button{display:block;width:100%;border:0;border-radius:6px;background:transparent;color:#344c69;text-align:left;padding:9px 10px;font-size:12px;font-weight:550;white-space:normal}
  .note-suggestions button:hover,.note-suggestions button.highlighted{background:#eaf3ff;color:#2f80da}
  .note-error{margin-top:9px;color:#bd405a;font-size:11px}
  .csv-actions{display:flex;align-items:center;gap:9px;flex-wrap:wrap;justify-content:flex-end}
  .csv-actions .back-button,.csv-actions .primary-button{white-space:nowrap;padding:11px 15px}
  .csv-actions button:disabled{opacity:.55}
  .import-result{background:#eaf4ff;color:#345577}
  .import-result p{margin:8px 0 4px}
  .import-result ul{margin:0;padding-left:20px;max-height:160px;overflow:auto}
  .profile-avatar{border:0;padding:0;overflow:hidden}.profile-avatar img{width:100%;height:100%;object-fit:cover;border-radius:50%}.dialog-backdrop{position:fixed;inset:0;z-index:100;display:grid;place-items:center;padding:20px;background:#172b4380;backdrop-filter:blur(3px)}.profile-dialog{position:relative;margin:0;width:min(100%,420px);background:#fff;border:1px solid #e4edf7;border-radius:18px;padding:30px;box-shadow:0 24px 70px #102e502e}.profile-dialog h2{font-family:Georgia,serif;font-size:27px;font-weight:500;margin:0 0 22px}.dialog-close{position:absolute;top:16px;right:18px;border:0;background:none;color:#8297af;font-size:25px}.profile-picture-preview{width:84px;height:84px;margin:0 auto 22px;border-radius:50%;overflow:hidden;background:#eaf3ff;color:#428ce7;display:grid;place-items:center;font-size:32px;font-weight:750}.profile-picture-preview img{width:100%;height:100%;object-fit:cover}.profile-url-row{display:flex;gap:8px;margin-top:11px}.profile-url-row input{flex:1;min-width:0;border:1px solid #d9e5f1;border-radius:8px;padding:10px 11px;font-size:12px;outline:none}.profile-url-row input:focus{border-color:#4d97ee;box-shadow:0 0 0 3px #4d97ee1c}.profile-url-row button{border:0;border-radius:8px;background:#eaf3ff;color:#4389d5;padding:0 12px;font-size:11px;font-weight:750}.profile-url-row button:disabled{opacity:.5}.clear-picture{display:block;margin:11px auto 0;border:0;background:none;color:#d56877;font-size:12px}.dialog-actions{display:flex;justify-content:flex-end;gap:10px;margin-top:25px}.dialog-actions .primary-button,.dialog-actions .back-button{padding:11px 16px}.sidebar-toggle{align-self:flex-end;border:0;background:#eef5ff;color:#397bc6;border-radius:7px;padding:6px 10px;margin-bottom:12px}.sidebar-collapsed .sidebar{width:68px;padding-left:8px;padding-right:8px}.sidebar-collapsed .brand-mark>span:last-child,.sidebar-collapsed .sidebar-label,.sidebar-collapsed .nav-text,.sidebar-collapsed .nav-link em,.sidebar-collapsed .side-bottom{display:none}.sidebar-collapsed .nav-link{justify-content:center;padding:12px}.breadcrumb{display:flex;align-items:center;gap:8px;min-width:0;overflow:hidden;white-space:nowrap;text-overflow:ellipsis}.breadcrumb button{border:0;background:none;color:#4b8fdc;padding:0;font-size:inherit}.breadcrumb [aria-current="page"]{overflow:hidden;text-overflow:ellipsis}.mobile-menu{display:none;border:0;background:none;color:#347fcf;font-size:22px}.ownership-summary{display:flex;flex-wrap:wrap;gap:8px;margin:20px 0}.ownership-summary span{background:#eaf4ff;border-radius:20px;padding:6px 10px;color:#467db3;font-size:12px}.bottle-counts{font-size:11px;color:#7693b0;margin-top:10px}.favourite-field{display:flex;align-items:center;gap:8px;margin:14px 0;font-weight:700}.category-picker{position:relative;margin-top:8px}.category-trigger{width:100%;height:48px;border:1px solid #dce8f5;border-radius:8px;background:#fff;color:#263b55;padding:0 15px;text-align:left;display:flex;align-items:center;justify-content:space-between;box-shadow:0 1px 2px #17395b08;transition:border-color .15s,box-shadow .15s}.category-trigger:hover,.category-trigger[aria-expanded="true"]{border-color:#8bbcf2}.category-trigger:focus-visible{outline:0;border-color:#5a9cec;box-shadow:0 0 0 3px #4b9aff25}.category-chevron{width:8px;height:8px;border-right:1.7px solid #7290ad;border-bottom:1.7px solid #7290ad;transform:rotate(45deg);margin:-4px 3px 0 10px;transition:transform .15s}.category-chevron.rotated{transform:rotate(225deg);margin-top:4px}.category-options{position:absolute;z-index:30;top:calc(100% + 6px);left:0;right:0;background:#fff;border:1px solid #e2ebf5;border-radius:9px;padding:5px;box-shadow:0 12px 28px #254b7224}.category-options button{display:flex;align-items:center;justify-content:space-between;width:100%;border:0;border-radius:6px;background:transparent;padding:10px 11px;color:#405772;text-align:left;font-size:13px}.category-options button:hover,.category-options button.selected{background:#edf5ff;color:#337fcf}.category-options button span{color:#458fe3;font-size:12px}.field{position:relative;min-width:0}.field>span{position:absolute;top:0;right:0;float:none;margin:0;line-height:1.2}.field.category-field>span{position:static;align-self:flex-start;margin:0;line-height:1.2}.form-grid{align-items:start;row-gap:26px}.field-title{margin-top:27px;margin-bottom:15px}.section-divider{margin:31px 0 25px}.category-picker{margin-top:9px}.category-field{margin-bottom:0}
  @media(max-width:800px){.sidebar{width:68px!important;padding:20px 8px!important}.sidebar .brand-mark>span:last-child,.sidebar .sidebar-label,.sidebar .nav-text,.sidebar .nav-link em,.sidebar .side-bottom{display:none}.sidebar .nav-link{font-size:14px!important;justify-content:center}.sidebar .nav-link>span:first-child{font-size:21px}.top-actions .local-badge,.top-actions .avatar{display:none}}
  @media(max-width:550px){.sidebar{display:flex!important;position:fixed;top:0;bottom:0;left:0;z-index:50;transform:translateX(-100%);transition:transform .2s;width:240px!important;padding:20px!important;box-shadow:10px 0 30px #193a5b20}.mobile-sidebar-open .sidebar{transform:translateX(0)}.mobile-sidebar-open .sidebar .brand-mark>span:last-child,.mobile-sidebar-open .sidebar .sidebar-label,.mobile-sidebar-open .sidebar .nav-text,.mobile-sidebar-open .sidebar .nav-link em{display:block}.mobile-sidebar-open .sidebar .nav-link{justify-content:flex-start}.mobile-menu{display:block}.topbar{gap:8px;padding:0 12px}.top-actions{margin-left:auto}.update-check{font-size:0!important;padding:7px!important}.update-check .action-icon{width:19px;height:19px}.search-box{width:min(100%,300px)}.library-toolbar{flex-wrap:wrap}.note-entry{flex-wrap:wrap}.note-entry>button{width:100%}.new-note-form{flex-wrap:wrap}.new-note-form button{min-height:40px}.fragrance-grid,.notes-grid{grid-template-columns:1fr}.page-content{padding:24px 15px}.detail-layout{grid-template-columns:minmax(0,1fr)}}
  @media(max-width:700px){.heading{align-items:start;flex-direction:column}.csv-actions{justify-content:flex-start}}
  .update-check{border:1px solid #d6e5f4;border-radius:7px;background:#fff;color:#3d81ca;padding:8px 12px;font-size:11px;font-weight:700;cursor:pointer}
  .update-check:disabled{opacity:.6;cursor:wait}
  .update-notice{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:12px 26px;background:#eaf3ff;color:#315b84;font-size:12px}
  .update-notice button{border:0;border-radius:6px;background:#448ff0;color:#fff;padding:8px 12px;font-size:11px;font-weight:700;cursor:pointer}
  /* Collection visual system */
  :global(body){color:#171417;background:#fff;font-family:Inter,ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
  .sidebar{width:242px;border-right:1px solid #e8e5e5;padding:28px 17px 18px}
  .sidebar-toggle{display:none}
  .brand-mark{gap:12px;padding:0 12px 40px;color:#111;font-family:Georgia,serif;font-size:22px;font-weight:700;letter-spacing:-1px;line-height:1}
  .brand-icon{width:31px;height:38px;object-fit:contain;flex:none}
  .brand-mark small{margin-top:7px;color:#171417;letter-spacing:2px;font-family:Inter,ui-sans-serif,system-ui,sans-serif;font-size:8px;line-height:1.5}
  .sidebar-label{color:#555153;font-size:10px;letter-spacing:2.4px;padding:0 11px 10px}
  .nav-link{color:#171417;padding:9px 13px;margin-bottom:4px;font-size:12px;font-weight:550;gap:14px;min-height:44px}
  .nav-link>span:first-child{font-size:22px;width:25px;line-height:1}
  .nav-icon{display:block;width:25px;height:25px;object-fit:contain}
  .nav-link.active{color:#171417;background:#eee9e9}
  .nav-link em{color:#292629;font-size:11px;font-weight:400}
  .nav-link.active em{background:#f1ded1;border-radius:50%;padding:3px 7px}
  .side-bottom{border:1px solid #eee9e9;background:#fcfbfb;color:#171417;padding:16px 18px;border-radius:14px}
  .side-orb{background:transparent;color:#171417;width:auto;height:auto;display:block;text-align:left;font-size:19px;margin-bottom:6px}
  .side-bottom strong{display:block;font-family:Georgia,serif;font-size:18px;font-style:italic;font-weight:400;line-height:1.2}
  .side-bottom p{font-size:11px;color:#595557;line-height:1.5}
  .topbar{height:75px;border-bottom:0;padding:0 24px;justify-content:flex-start;gap:26px}
  .top-back{width:38px;height:38px;border:0;border-radius:9px;background:#f5f4f4;font-size:22px;color:#171417}
  .breadcrumb{font-size:15px;color:#171417;font-weight:650}
  .breadcrumb span{color:#171417;padding:0 4px}
  .breadcrumb button{color:#171417}
  .top-actions{margin-left:auto;gap:25px}
  .update-check{border-color:#e8e5e5;color:#171417;border-radius:12px;padding:12px 19px;font-size:12px;font-weight:550}
  .update-check{display:inline-flex;align-items:center;gap:10px}
  .action-icon{display:inline-block;width:19px;height:19px;object-fit:contain;flex:none}
  .primary-button .action-icon{filter:invert(1)}
  .local-badge{color:#171417;font-size:13px}
  .local-badge span{background:#16a34a;width:11px;height:11px;margin-right:9px}
  .avatar{width:45px;height:45px;background:#f1eef1;color:#171417;font-family:Georgia,serif;font-size:20px;font-weight:400}
  .page-content{max-width:none;padding:23px 22px 70px}
  .collection-heading{border-left:2px solid #bd8a13;margin:0 0 27px 17px;padding:7px 0 5px 35px;min-height:120px;align-items:center}
  .eyebrow{color:#4d494c;letter-spacing:4px;font-size:11px;margin-bottom:10px}
  .collection-heading h1{color:#111;font-size:47px;margin:0 0 4px;letter-spacing:-2px}
  .heading-dot{color:#ad7d11}
  .heading p{color:#5e5a5d;font-size:15px}
  .collection-controls{display:flex;align-items:center;justify-content:space-between;gap:22px;margin-bottom:29px}
  .collection-search{display:flex;align-items:center;gap:16px;min-width:0}
  .collection-search .search-box{width:min(40vw,492px);height:47px;background:#faf9f9;border-color:#e4e1e1;border-radius:12px;color:#171417;font-size:30px;padding:0 19px}
  .collection-search .search-box input{background:transparent;font-size:14px;color:#171417;padding-left:14px}
  .filter-button{height:47px;width:55px;border:1px solid #e7e3e3;border-radius:12px;background:#fff;color:#171417;font-size:26px}
  .collection-controls .csv-actions{gap:17px}
  .collection-controls .csv-actions button{display:inline-flex;align-items:center;justify-content:center;gap:9px;height:47px;border-radius:11px;font-size:13px;font-weight:600;padding:0 18px}
  .collection-controls .csv-actions .back-button{color:#171417;border-color:#e7e3e3}
  .collection-controls .csv-actions .back-button:first-letter{font-size:22px}
  .primary-button{background:#552044;color:#fff;box-shadow:none}
  .primary-button:hover{background:#6c2b59}
  .collection-controls .primary-button{min-width:194px}
  .collection-controls .primary-button span{margin:0}
  .collection-subbar{display:flex;align-items:center;justify-content:space-between;gap:16px;margin-bottom:25px}
  .shelf-filters{display:flex;align-items:center;gap:15px;flex-wrap:wrap}
  .shelf-filters button{border:1px solid #e8e5e5;border-radius:22px;background:#fff;color:#171417;min-height:41px;padding:0 23px;font-size:13px;font-weight:550}
  .shelf-filters button span{font-size:22px;margin-right:8px;vertical-align:-2px}
  .filter-icon{display:inline-block;width:22px;height:22px;object-fit:contain;vertical-align:-5px}
  .shelf-filters button.active{background:#552044;color:#fff;border-color:#552044}
  .shelf-filters button.active .filter-icon{filter:invert(1)}
  .collection-view{display:flex;align-items:center;gap:25px;white-space:nowrap;color:#3e4153;font-size:14px}
  .layout-switch{display:flex;border:1px solid #e7e3e3;border-radius:12px;overflow:hidden}
  .layout-switch button{border:0;background:#fff;width:59px;height:39px;color:#171417;font-size:23px}
  .layout-switch button.active{background:#552044;color:#fff;border-radius:10px}
  .fragrance-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,360px));gap:23px}
  .fragrance-card{border-radius:0;overflow:visible;max-width:360px}
  .fragrance-art{height:300px;position:relative;background:#fff;color:#c9a768}
  .fragrance-art img{height:100%;width:100%;object-fit:contain}
  .bottle-placeholder{font-size:120px;color:#d1ac6c}
  .shelf-badge{position:absolute;top:12px;left:20px;z-index:1;background:#dff7e7;color:#118041;border-radius:20px;padding:7px 12px;font-size:12px;font-weight:600}
  .card-menu{position:absolute;top:12px;right:0;width:39px;height:39px;border:0;border-radius:50%;background:#f7f6f6;color:#171417;font-size:24px;line-height:0}
  .fragrance-card{cursor:pointer}
  .fragrance-card:focus-visible{outline:2px solid #552044;outline-offset:5px}
  .card-menu-wrap{position:absolute;top:12px;right:0;z-index:3}
  .card-menu-wrap .card-menu{position:static}
  .card-dropdown{position:absolute;top:45px;right:0;min-width:135px;padding:5px;background:#fff;border:1px solid #e8e5e5;border-radius:10px;box-shadow:0 12px 30px #281d2726}
  .card-dropdown button{display:block;width:100%;border:0;border-radius:6px;background:transparent;padding:10px 12px;text-align:left;color:#292629;font-size:13px}
  .card-dropdown button:hover{background:#f6f3f5}
  .card-dropdown .delete-action{color:#d82929}
  .fragrance-info{padding:0 6px}
  .fragrance-kicker{color:#171417;font-size:10px;letter-spacing:2px}
  .fragrance-info h2{font-size:26px;font-weight:600;color:#171417;margin:6px 0 2px}
  .fragrance-info p{font-size:14px;color:#5e5a5d}
  .bottle-counts{font-size:13px;color:#3e4153;margin-top:9px}
  .bottle-counts span{padding:0 8px}
  .mini-notes{margin-top:14px;gap:8px}
  .mini-notes span{background:#eef0ff;color:#172563;padding:6px 14px;font-size:12px}
  .list-layout{grid-template-columns:1fr}
  .list-layout .fragrance-card{max-width:none;display:grid;grid-template-columns:170px minmax(0,1fr) auto;align-items:center;border-bottom:1px solid #eee;padding-bottom:16px}
  .list-layout .fragrance-art{height:150px}
  .list-layout .fragrance-info{padding:0 20px}
  @media(max-width:1100px){.collection-controls{align-items:flex-start;flex-direction:column}.collection-search .search-box{width:min(60vw,492px)}.collection-subbar{align-items:flex-start;flex-direction:column}.collection-view{align-self:flex-end}}
  @media(max-width:800px){.sidebar{width:68px!important}.page-content{padding:23px 20px 60px}}
  @media(max-width:550px){.top-back{display:none}.collection-heading{margin-left:0;padding-left:18px}.collection-heading h1{font-size:42px}.collection-controls .csv-actions{gap:8px}.collection-controls .csv-actions button{font-size:12px;padding:0 10px}.collection-controls .primary-button{min-width:0}.collection-search{width:100%}.collection-search .search-box{width:100%}.shelf-filters{gap:7px}.shelf-filters button{font-size:12px;padding:0 12px}}
  .choice-icon img{width:23px;height:23px;object-fit:contain}
  .concentration-field .category-picker{margin-top:0}
  .concentration-options{max-height:250px;overflow-y:auto}
  .concentration-pill{margin-left:8px}
  .fragrance-grid{grid-template-columns:repeat(auto-fill,minmax(155px,1fr));gap:28px 18px}
  .fragrance-card{width:100%;max-width:none;text-align:center}
  .fragrance-art{height:175px}
  .fragrance-art img{width:100%;height:100%;max-width:100%;max-height:100%;object-fit:contain;padding:9px 17px}
  .bottle-placeholder{font-size:65px}
  .card-menu-wrap{top:2px;right:2px}
  .card-menu{width:29px;height:29px;font-size:19px}
  .fragrance-info{padding:7px 5px 5px}
  .fragrance-info h2{font-family:Inter,ui-sans-serif,system-ui,sans-serif;font-size:14px;font-weight:650;color:#317fcb;line-height:1.3;margin:0 0 6px}
  .listing-meta{display:flex;align-items:center;justify-content:center;gap:5px;flex-wrap:wrap}
  .listing-meta span{font-size:10px;color:#708098;background:#f1f3f8;border-radius:5px;padding:3px 6px;text-transform:uppercase;line-height:1.3}
  .listing-meta .sex-icon{display:block;width:20px;height:20px;object-fit:contain;flex:none}
  .listing-meta .concentration-label{font-family:Georgia,serif;font-size:12px;font-style:italic;font-weight:500;letter-spacing:.2px;text-transform:none;background:transparent;color:#6f5970}
  .list-layout .fragrance-card{grid-template-columns:105px minmax(0,1fr);padding-bottom:10px;text-align:left}
  .list-layout .fragrance-art{height:100px}
  .list-layout .fragrance-info{padding:0 15px}
  .list-layout .listing-meta{justify-content:flex-start}
  @media(max-width:550px){.fragrance-grid{grid-template-columns:repeat(2,minmax(0,1fr));gap:22px 10px}.fragrance-art{height:145px}.list-layout{grid-template-columns:1fr}.list-layout .fragrance-card{grid-template-columns:90px minmax(0,1fr)}}

  /* Shared form and navigation sizing */
  .app-shell{--border:#d8d1d8;--accent:#552044;--accent-soft:#5520441c;--focus-ring:#55204430;--muted:#6b6475}
  .sidebar{width:264px}
  .nav-link{gap:10px;padding:10px 12px;white-space:nowrap}
  .nav-link .nav-icon{display:grid;place-items:center;width:20px;height:20px;flex:none;color:var(--muted)}
  .nav-link .nav-text{white-space:nowrap}
  .nav-link em{margin-left:auto;flex:none}
  .nav-link.active{background:var(--accent-soft);color:var(--accent)}
  .nav-link.active .nav-icon,.nav-link.active em{color:var(--accent)}
  .nav-link.active em{background:transparent}
  .topbar{gap:16px}
  .top-back,.update-check,.profile-avatar{height:40px;flex:none}
  .breadcrumb{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .breadcrumb [aria-current="page"]{min-width:0}
  .top-actions{margin-left:0;gap:14px;flex:none}
  .update-check{white-space:nowrap;height:40px}
  .local-badge{display:flex;align-items:center;gap:8px;height:40px;white-space:nowrap}
  .local-badge .local-dot{width:11px;height:11px;margin:0;flex:none}
  .local-badge .local-text{white-space:nowrap}
  .progress{min-width:0}
  .progress button{flex:none;color:var(--muted);font-size:12px}
  .progress button.current{color:var(--accent)}
  .current .step-number{background:var(--accent);border-color:var(--accent);box-shadow:0 3px 10px var(--focus-ring)}
  .connector{flex:1;min-width:24px;margin:0 12px}
  .card-heading p{font-size:13px;color:var(--muted)}
  .field{min-width:0;gap:0;color:#171417}
  .field>.field-label-row{position:static;display:flex;align-items:baseline;justify-content:space-between;gap:6px;height:20px;min-height:20px;margin:0 0 8px;float:none;line-height:20px;width:100%;color:#171417;font-size:13px;font-weight:600}
  .field-label-text{min-width:0;white-space:nowrap}
  .field-label-text b{display:inline;margin-left:2px;color:#ba3150}
  .field .field-hint{position:static;display:inline;margin-left:auto;float:none;color:var(--muted);font-size:12px;font-weight:400;white-space:nowrap}
  .form-grid{align-items:start}
  .field input:not([type="checkbox"]),:global(.field select),.category-trigger,.note-entry input{height:44px;min-height:44px;padding:0 14px;border:1px solid var(--border);border-radius:12px;background:#fff;color:#171417;color-scheme:light;box-shadow:none}
  :global(.field select){appearance:none;padding-right:40px;background-image:url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12' fill='none'%3E%3Cpath d='m2 4 4 4 4-4' stroke='%236b6475' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");background-repeat:no-repeat;background-position:right 15px center}
  .field input[type="number"]{appearance:textfield}
  .field input[type="number"]::-webkit-inner-spin-button,.field input[type="number"]::-webkit-outer-spin-button{appearance:none;margin:0}
  .field input:focus,:global(.field select:focus),.category-trigger:focus-visible,.note-entry input:focus{outline:0;border-color:var(--accent);box-shadow:0 0 0 3px var(--focus-ring)}
  .category-picker{margin-top:0}
  .category-trigger{padding-right:14px}
  .category-trigger:hover,.category-trigger[aria-expanded="true"]{border-color:var(--accent)}
  .category-options button:hover,.category-options button.selected{background:var(--accent-soft);color:var(--accent)}
  .field small,.note-title p,.empty-hint,.summary p{font-size:12px;color:var(--muted)}
  .page-content .eyebrow,.step-count{color:var(--accent)}
  .card-icon,.note-icon{background:var(--accent-soft);color:var(--accent)}
  .choice-grid button.selected{border-color:var(--accent);background:var(--accent-soft);box-shadow:0 0 0 2px var(--focus-ring);color:var(--accent)}
  .choice-icon{background:var(--accent-soft);color:var(--accent)}
  .selected .radio-dot{border-color:var(--accent)}
  .note-entry>button{background:var(--accent)}
  .note-chips button{background:var(--accent-soft);color:var(--accent)}
  @media(max-width:1100px){
    .update-check{width:40px;padding:0;justify-content:center;font-size:0}
    .update-check .action-icon{width:19px;height:19px}
    .local-badge .local-text{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}
  }
  @media(max-width:900px){
    .sidebar{width:68px;padding:20px 8px}
    .sidebar .brand-mark>span:last-child,.sidebar .sidebar-label,.sidebar .nav-text,.sidebar .nav-link em,.sidebar .side-bottom{display:none}
    .sidebar .nav-link{justify-content:center;padding:12px}
    .progress button:not(.current)>span:last-child{display:none}
  }
  @media(max-width:800px){.sidebar{width:68px!important}.top-actions .local-badge{display:flex}.progress button.current>span:last-child{display:inline}}
  @media(max-width:550px){
    .sidebar{width:240px!important}
    .mobile-sidebar-open .sidebar .nav-text,.mobile-sidebar-open .sidebar .nav-link em{display:inline}
    .mobile-sidebar-open .sidebar .nav-link{justify-content:flex-start}
    .topbar{gap:8px;padding:0 12px}
    .top-actions{gap:8px}
    .breadcrumb{font-size:12px}
    .connector{min-width:16px;margin:0 6px}
  }
</style>
