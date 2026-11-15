<script lang="ts">
  import { onMount } from 'svelte';
  import { backend, type Fragrance, type NoteImage } from '$lib/backend';

  const blank = (): Fragrance => ({ collection: '', name: '', brand: '', year: new Date().getFullYear(), audience: 'Unisex', description: '', topNotes: [], heartNotes: [], baseNotes: [], perfumer: '', bottleRating: null, ratingCount: null, imageUrl: '' });
  let fragrance = $state<Fragrance>(blank());
  let entries = $state<Fragrance[]>([]);
  let step = $state(0);
  let view = $state<'form' | 'library' | 'notes' | 'detail'>('form');
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
  let noteInputs = $state({ topNotes: '', heartNotes: '', baseNotes: '' });
  const steps = ['General information', 'Fragrance pyramid', 'Additional details'];
  const categories = [
    { value: 'Women', icon: '♀', label: 'Women' },
    { value: 'Men', icon: '♂', label: 'Men' },
    { value: 'Unisex', icon: '⚥', label: 'Unisex' }
  ];
  const allNotes = $derived(noteImages.map((note) => note.name).sort((a, b) => a.localeCompare(b)));
  const visibleNotes = $derived(allNotes.filter((note) => note.toLocaleLowerCase().includes(noteSearch.toLocaleLowerCase())));
  const selectedFragrance = $derived(entries.find((item) => item.id === selectedId));
  const visibleEntries = $derived(entries.filter((item) => `${item.name} ${item.brand} ${item.collection}`.toLowerCase().includes(search.toLowerCase())));

  onMount(async () => {
    try { entries = await backend.listFragrances();
      noteImages = await backend.listNoteImages(); }
    catch (e) { error = String(e); }
  });

  async function refreshNotesAndFragrances() {
    entries = await backend.listFragrances();
    noteImages = await backend.listNoteImages();
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
  function addNote(kind: 'topNotes' | 'heartNotes' | 'baseNotes') {
    const value = noteInputs[kind].trim();
    if (!value) return;
    const note = allNotes.find((name) => name.toLocaleLowerCase() === value.toLocaleLowerCase()) ?? value;
    if (fragrance[kind].some((name) => name.toLocaleLowerCase() === note.toLocaleLowerCase())) {
      noteInputs[kind] = '';
      return;
    }
    fragrance[kind] = [...fragrance[kind], note];
    noteInputs[kind] = '';
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
    if (step < 2) step += 1;
  }
  function startNew() {
    fragrance = blank();
    noteInputs = { topNotes: '', heartNotes: '', baseNotes: '' };
    step = 0; view = 'form'; error = ''; success = '';
  }
  function openDetail(item: Fragrance) {
    selectedId = item.id ?? null; view = 'detail'; error = ''; success = '';
  }
  function startEdit(item: Fragrance) {
    fragrance = { ...item, topNotes: [...item.topNotes], heartNotes: [...item.heartNotes], baseNotes: [...item.baseNotes] };
    noteInputs = { topNotes: '', heartNotes: '', baseNotes: '' };
    step = 0; view = 'form'; error = ''; success = '';
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
      fragrance = blank(); noteInputs = { topNotes: '', heartNotes: '', baseNotes: '' };
      step = 0; view = 'library'; success = wasEditing ? 'Fragrance updated.' : 'Fragrance saved to your collection.';
    } catch (e) { error = String(e); }
    finally { saving = false; }
  }
</script>

<svelte:head>
  <title>Scentkeep — Fragrance collection</title>
  <meta name="description" content="A personal fragrance catalogue" />
</svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand-mark"><span class="brand-icon">✳</span><span>Scentkeep<small>THE FRAGRANCE JOURNAL</small></span></div>
    <div class="sidebar-label">WORKSPACE</div>
    <button class:active={view === 'library'} class="nav-link" onclick={() => { view = 'library'; error = ''; }}><span>▦</span> My collection <em>{entries.length}</em></button>
    <button class:active={view === 'form'} class="nav-link" onclick={startNew}><span>＋</span> Add fragrance</button>
    <button class:active={view === 'notes'} class="nav-link" onclick={() => { view = 'notes'; error = ''; success = ''; }}><span>◈</span> Perfume notes <em>{allNotes.length}</em></button>
    <div class="side-bottom"><div class="side-orb">✦</div><strong>Your scent story starts here.</strong><p>Keep every note, memory and discovery in one beautiful place.</p></div>
  </aside>

  <main>
    <header class="topbar"><div class="breadcrumb">Collection <span>/</span> {view === 'form' ? (fragrance.id === undefined ? 'Add fragrance' : 'Edit fragrance') : view === 'notes' ? 'Perfume notes' : view === 'detail' ? (selectedFragrance?.name ?? 'Fragrance') : 'All fragrances'}</div><div class="top-actions"><span class="local-badge"><span></span> Saved locally</span><div class="avatar">S</div></div></header>
    {#if view === 'form'}
      <div class="page-content">
        <div class="heading"><div><div class="eyebrow">YOUR FRAGRANCE LIBRARY</div><h1>{fragrance.id === undefined ? 'Add a fragrance' : 'Edit fragrance'}<span class="heading-dot">.</span></h1><p>Capture the details that make each scent unforgettable.</p></div><div class="step-count">STEP 0{step + 1} <span>/ 03</span></div></div>
        <div class="progress" aria-label="Form progress">{#each steps as label, index}<button class:current={step === index} class:done={step > index} onclick={() => { if (index < step) step = index; }}><span class="step-number">{step > index ? '✓' : `0${index + 1}`}</span><span>{label}</span></button>{#if index < 2}<div class:filled={step > index} class="connector"></div>{/if}{/each}</div>
        {#if error}<div class="message error" role="alert">{error}</div>{/if}
        <div class="form-card">
          {#if step === 0}
            <div class="card-heading"><span class="card-icon">✦</span><div><h2>The essentials</h2><p>Start with the fragrance’s identity.</p></div></div>
            <div class="form-grid"><label class="field wide">Fragrance name <b>*</b><input placeholder="e.g. Odéon Sunset" bind:value={fragrance.name} /></label><label class="field">Brand / house <b>*</b><input placeholder="e.g. Memo Paris" bind:value={fragrance.brand} /></label><label class="field">Collection <span>Optional</span><input placeholder="e.g. Fleurs Bohèmes" bind:value={fragrance.collection} /></label><label class="field">Release year<input type="number" min="1800" max="2100" bind:value={fragrance.year} /></label><label class="field">Perfumer <span>Optional</span><input placeholder="e.g. Aliénor Massenet" bind:value={fragrance.perfumer} /></label></div>
            <div class="section-divider"></div><div class="field-title">Who is this fragrance for?</div><div class="choice-grid">{#each categories as category}<button type="button" class:selected={fragrance.audience === category.value} onclick={() => fragrance.audience = category.value}><span class="choice-icon">{category.icon}</span><strong>{category.label}</strong><span class="radio-dot"></span></button>{/each}</div>
          {:else if step === 1}
            <div class="card-heading"><span class="card-icon">◈</span><div><h2>Fragrance pyramid</h2><p>Build the scent, layer by layer. Add each note separately.</p></div></div>
            <datalist id="saved-note-suggestions">{#each allNotes as note}<option value={note}></option>{/each}</datalist>
            {#each [{ key: 'topNotes', title: 'Top notes', hint: 'The first impression', icon: '✧', example: 'Candied apple' }, { key: 'heartNotes', title: 'Heart notes', hint: 'The heart of the scent', icon: '❀', example: 'Caramel' }, { key: 'baseNotes', title: 'Base notes', hint: 'The lasting impression', icon: '◆', example: 'Sandalwood' }] as group}
              {@const kind = group.key as 'topNotes' | 'heartNotes' | 'baseNotes'}
              <div class="note-section"><div class="note-title"><span class="note-icon">{group.icon}</span><div><h3>{group.title}</h3><p>{group.hint}</p></div></div><div class="note-entry"><input aria-label={`Add ${group.title.toLowerCase()}`} list="saved-note-suggestions" placeholder={`e.g. ${group.example}`} bind:value={noteInputs[kind]} onkeydown={(event) => { if (event.key === 'Enter') { event.preventDefault(); addNote(kind); } }} /><button type="button" onclick={() => addNote(kind)}>+ Add note</button></div><div class="note-chips">{#each fragrance[kind] as note, index}<button type="button" title={`Remove ${note}`} onclick={() => removeNote(kind, index)}>{note} <span>×</span></button>{/each}{#if fragrance[kind].length === 0}<span class="empty-hint">No notes added yet</span>{/if}</div></div>
            {/each}
          {:else}
            <div class="card-heading"><span class="card-icon">☷</span><div><h2>Finishing touches</h2><p>Add context and a little more character.</p></div></div>
            <label class="field">Description <span>Optional</span><textarea rows="5" placeholder="What makes this fragrance special?" bind:value={fragrance.description}></textarea></label>
            <div class="form-grid details"><label class="field">Bottle rating <span>0–10</span><input type="number" min="0" max="10" step="0.1" placeholder="e.g. 7.6" bind:value={fragrance.bottleRating} /></label><label class="field">Number of ratings <span>Optional</span><input type="number" min="0" placeholder="e.g. 5" bind:value={fragrance.ratingCount} /></label></div>
            <label class="field">Image URL <span>Optional</span><input type="url" placeholder="https://example.com/fragrance.jpg" bind:value={fragrance.imageUrl} /><small>A link to the bottle image. Leave blank to use a placeholder.</small></label>
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
          <div class="detail-main"><div class="detail-kicker">{selectedFragrance.collection || 'FRAGRANCE COLLECTION'}</div><h1>{selectedFragrance.name}<span class="heading-dot">.</span></h1><div class="detail-brand">{selectedFragrance.brand}<span class="year-pill">{selectedFragrance.year}</span></div><div class="audience-pill">{selectedFragrance.audience}</div>{#if selectedFragrance.description}<p class="detail-description">{selectedFragrance.description}</p>{/if}
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
        <div class="heading"><div><div class="eyebrow">YOUR SCENT INDEX</div><h1>Perfume notes<span class="heading-dot">.</span></h1><p>See every note in your collection and give it an image.</p></div><div class="step-count">{allNotes.length} NOTES</div></div>
        {#if success}<div class="message success">{success}</div>{/if}{#if error}<div class="message error" role="alert">{error}</div>{/if}
        <form class="new-note-form" onsubmit={(event) => { event.preventDefault(); createNote(); }}><input aria-label="New note name" placeholder="Add a new perfume note..." bind:value={newNoteName} /><button type="submit" disabled={!newNoteName.trim()}>+ Add note</button></form>
        <div class="library-toolbar"><div class="search-box">⌕ <input aria-label="Search notes" placeholder="Search perfume notes..." bind:value={noteSearch} /></div><span>{visibleNotes.length} {visibleNotes.length === 1 ? 'note' : 'notes'}</span></div>
        {#if visibleNotes.length === 0}
          <div class="empty-state"><div>◈</div><h2>{allNotes.length === 0 ? 'No notes in your collection yet' : 'No notes found'}</h2><p>{allNotes.length === 0 ? 'Add a note above, or add notes to a fragrance.' : 'Try another search term.'}</p>{#if allNotes.length === 0}<button class="primary-button" onclick={startNew}>Add a fragrance →</button>{/if}</div>
        {:else}
          <div class="notes-grid">{#each visibleNotes as note}<article class="note-card"><div class="note-art">{#if imageFor(note)}<img src={imageFor(note)} alt={note} />{:else}<span>✧</span>{/if}</div><div class="note-card-body">{#if editingNote === note}<form class="rename-note-form" onsubmit={(event) => { event.preventDefault(); renameNote(note); }}><input aria-label={`Rename ${note}`} bind:value={editedNoteName} /><button type="submit" disabled={!editedNoteName.trim()}>Save</button><button type="button" onclick={() => editingNote = ''}>Cancel</button></form>{:else}<h2>{note}</h2>{/if}<p>Used in {noteImages.find((image) => image.name === note)?.usageCount ?? 0} fragrances</p><form class="note-image-form" onsubmit={(event) => { event.preventDefault(); setNoteImage(note); }}><input type="url" aria-label={`Image link for ${note}`} placeholder="https://example.com/note.jpg" value={noteImageDrafts[note] ?? ''} oninput={(event) => noteImageDrafts[note] = event.currentTarget.value} /><div><button type="submit" disabled={savingNoteImage === note || !noteImageDrafts[note]?.trim()}>{savingNoteImage === note ? 'Saving…' : imageFor(note) ? 'Update image link' : '+ Add image link'}</button>{#if imageFor(note)}<button type="button" onclick={() => removeNoteImage(note)}>Remove image</button>{/if}</div></form><div class="note-manage-actions"><button onclick={() => { editingNote = note; editedNoteName = note; }}>Rename</button><button class="danger" onclick={() => deleteNote(note)}>Delete note</button></div></div></article>{/each}</div>
        {/if}
      </div>
    {:else}
      <div class="page-content"><div class="heading"><div><div class="eyebrow">YOUR FRAGRANCE LIBRARY</div><h1>My collection<span class="heading-dot">.</span></h1><p>A home for every scent worth remembering.</p></div><button class="primary-button" onclick={startNew}>+ Add fragrance</button></div>{#if success}<div class="message success">{success}</div>{/if}{#if error}<div class="message error">{error}</div>{/if}<div class="library-toolbar"><div class="search-box">⌕ <input aria-label="Search fragrances" placeholder="Search fragrances or brands..." bind:value={search} /></div><span>{visibleEntries.length} {visibleEntries.length === 1 ? 'fragrance' : 'fragrances'}</span></div>{#if visibleEntries.length === 0}<div class="empty-state"><div>✳</div><h2>{entries.length === 0 ? 'Your collection is waiting' : 'No fragrances found'}</h2><p>{entries.length === 0 ? 'Add your first fragrance to start your scent journal.' : 'Try another search term.'}</p>{#if entries.length === 0}<button class="primary-button" onclick={startNew}>Add your first fragrance →</button>{/if}</div>{:else}<div class="fragrance-grid">{#each visibleEntries as item}<article class="fragrance-card"><div class="fragrance-art">{#if item.imageUrl}<img src={item.imageUrl} alt={item.name} />{:else}<span>✳</span>{/if}</div><div class="fragrance-info"><div class="fragrance-kicker">{item.collection || item.audience} · {item.year}</div><h2>{item.name}</h2><p>{item.brand}</p><div class="mini-notes">{#each [...item.topNotes, ...item.heartNotes, ...item.baseNotes].slice(0, 3) as note}<span>{note}</span>{/each}</div>{#if item.bottleRating !== null}<div class="rating">★ {item.bottleRating.toFixed(1)} <span>Bottle rating</span></div>{/if}</div><div class="card-actions"><button type="button" class="view-action" onclick={() => openDetail(item)}>View details →</button><button type="button" class="edit-action" onclick={() => startEdit(item)}>✎ Edit</button><button type="button" class="delete-action" onclick={() => remove(item)}>Delete</button></div></article>{/each}</div>{/if}</div>
    {/if}
  </main>
</div>

<style>
  :global(*){box-sizing:border-box} :global(body){margin:0;background:#fff;color:#182941;font-family:Inter,ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;font-size:14px} :global(button),:global(input),:global(textarea){font:inherit} :global(button),:global(a[href]),:global(select),:global([role="button"]){cursor:pointer} :global(button:disabled){cursor:not-allowed}
  .app-shell{display:flex;min-height:100vh}.sidebar{width:248px;flex:none;background:#fff;border-right:1px solid #e9eff7;padding:30px 18px;display:flex;flex-direction:column}.brand-mark{display:flex;align-items:center;gap:10px;padding:0 12px 53px;color:#2563ad;font-size:22px;font-weight:800;letter-spacing:-1px}.brand-icon{width:39px;height:39px;border-radius:12px;background:#438feb;color:white;display:grid;place-items:center;font-size:25px}.brand-mark small{display:block;font-size:8px;letter-spacing:2px;color:#8ca1bd;font-weight:700;margin-top:2px}.sidebar-label{font-size:10px;font-weight:800;color:#a8b7cb;letter-spacing:1.6px;padding:0 15px 13px}.nav-link{display:flex;align-items:center;gap:14px;width:100%;border:0;background:transparent;color:#71829b;text-align:left;padding:14px 16px;border-radius:11px;font-weight:650;margin-bottom:5px}.nav-link>span{font-size:19px;width:20px;text-align:center}.nav-link.active{background:#eaf3ff;color:#3989e9}.nav-link em{font-size:11px;font-style:normal;margin-left:auto;color:#87a5ca}.side-bottom{margin-top:auto;background:#eaf4ff;border-radius:15px;padding:20px;color:#345577}.side-orb{width:35px;height:35px;border-radius:50%;background:#d4e9ff;display:grid;place-items:center;color:#408be5;font-size:20px;margin-bottom:13px}.side-bottom strong{font-size:13px}.side-bottom p{font-size:11px;line-height:1.7;color:#8199b5;margin-bottom:0}main{min-width:0;flex:1}.topbar{height:76px;background:#fff;border-bottom:1px solid #e9eff7;display:flex;align-items:center;justify-content:space-between;padding:0 48px}.breadcrumb{color:#7d91ab;font-size:12px;font-weight:650}.breadcrumb span{padding:0 12px;color:#c5d0dd}.top-actions{display:flex;align-items:center;gap:20px}.local-badge{color:#7890a6;font-size:11px}.local-badge span{display:inline-block;width:7px;height:7px;border-radius:50%;background:#58c99e;margin-right:5px}.avatar{width:32px;height:32px;background:#eaf2ff;color:#428ce7;border-radius:50%;display:grid;place-items:center;font-weight:800}.page-content{max-width:990px;margin:0 auto;padding:47px 38px 80px}.heading{display:flex;align-items:end;justify-content:space-between;gap:20px;margin-bottom:38px}.eyebrow{color:#4e9bf1;font-size:10px;letter-spacing:2px;font-weight:800;margin-bottom:11px}h1{font-family:Georgia,serif;font-size:39px;font-weight:500;letter-spacing:-1.5px;margin:0 0 9px}.heading-dot{color:#438ff0}.heading p{margin:0;color:#8a9db4}.step-count{font-size:11px;color:#438ff0;font-weight:800;letter-spacing:1.5px;padding-bottom:4px}.step-count span{color:#b4c4d6}.progress{display:flex;align-items:center;margin-bottom:27px}.progress button{border:0;background:transparent;color:#9cb2ca;display:flex;align-items:center;gap:9px;white-space:nowrap;font-size:11px;font-weight:700;padding:0}.progress button.current{color:#3187ee}.progress button.done{color:#50a99a}.step-number{width:32px;height:32px;border:1px solid #d9e8f8;border-radius:50%;display:grid;place-items:center;background:#fff;color:#98b5d4}.current .step-number{background:#448ff0;border-color:#448ff0;color:white;box-shadow:0 3px 10px #448ff050}.done .step-number{background:#e5f9f1;border-color:#c6ebde;color:#43ae8c}.connector{height:1px;background:#dbe8f6;flex:1;margin:0 14px}.connector.filled{background:#82c9b3}.form-card{background:white;border:1px solid #e6edf7;border-radius:17px;box-shadow:0 12px 40px #2b6dae0a;padding:34px 39px 38px}.card-heading{display:flex;align-items:center;gap:15px;margin-bottom:31px}.card-icon{width:42px;height:42px;border-radius:12px;background:#eaf3ff;color:#438fec;display:grid;place-items:center;font-size:22px}.card-heading h2{font-family:Georgia,serif;font-size:23px;font-weight:500;margin:0 0 4px}.card-heading p{margin:0;color:#9aabc0;font-size:12px}.form-grid{display:grid;grid-template-columns:1fr 1fr;gap:23px 25px}.field{display:flex;flex-direction:column;gap:9px;font-size:12px;font-weight:700;color:#34445c}.field.wide{grid-column:span 2}.field b,.form-footer b{color:#fa8c9d}.field>span{float:right;color:#b7c5d4;font-size:10px;font-weight:500;margin-left:auto}.field input,.field textarea,.note-entry input{width:100%;border:1px solid #dae5f1;border-radius:7px;padding:12px 14px;outline:none;background:#fff;color:#263b58;font-weight:400}.field input:focus,.field textarea:focus,.note-entry input:focus,.search-box input:focus{border-color:#4d97ee;box-shadow:0 0 0 3px #4d97ee1c}.field input::placeholder,.field textarea::placeholder,.note-entry input::placeholder{color:#b4c2d1}.field textarea{resize:vertical}.field small{font-size:10px;font-weight:400;color:#a0aec0}.section-divider{height:1px;background:#edf2f7;margin:32px 0 26px}.field-title{font-size:12px;font-weight:700;margin-bottom:15px}.choice-grid{display:grid;grid-template-columns:repeat(3,1fr);gap:13px}.choice-grid button{position:relative;border:1px solid #dfeaf5;background:white;border-radius:10px;padding:18px 15px;display:flex;align-items:center;gap:12px;color:#7188a3}.choice-grid button.selected{border-color:#4b9af3;background:#f0f7ff;box-shadow:0 0 0 2px #4b9af318;color:#2f7cd8}.choice-icon{width:32px;height:32px;background:#ecf4ff;color:#438eec;border-radius:50%;display:grid;place-items:center;font-size:19px}.choice-grid strong{font-size:12px}.radio-dot{margin-left:auto;width:14px;height:14px;border:1px solid #c8d9ec;border-radius:50%}.selected .radio-dot{border:4px solid #478fed}.form-footer{display:flex;justify-content:space-between;align-items:center;margin-top:23px;color:#a6b5c7;font-size:11px}.form-footer>div{display:flex;gap:11px}.primary-button,.back-button{border:0;border-radius:8px;padding:13px 24px;font-weight:750;font-size:12px}.primary-button{background:#458ff0;color:white;box-shadow:0 5px 13px #458ff03d}.primary-button:hover{background:#2c7bdd}.primary-button:disabled{opacity:.6;cursor:wait}.primary-button span{margin-left:19px}.back-button{background:white;border:1px solid #dce8f5;color:#7e91aa}.message{padding:12px 16px;border-radius:8px;margin-bottom:18px;font-size:12px}.message.error{background:#fff1f2;color:#bd405a}.message.success{background:#e9f9ef;color:#27865a}.note-section{border:1px solid #e4edf7;border-radius:12px;padding:20px;margin-bottom:17px}.note-title{display:flex;align-items:center;gap:11px;margin-bottom:15px}.note-icon{width:36px;height:36px;border-radius:50%;background:#eaf4ff;color:#4d97e9;display:grid;place-items:center;font-size:19px}.note-title h3{margin:0 0 3px;font-size:13px}.note-title p{margin:0;font-size:11px;color:#a0b0c4}.note-entry{display:flex;gap:10px}.note-entry button{white-space:nowrap;border:0;border-radius:7px;background:#448ff0;color:white;padding:0 17px;font-size:11px;font-weight:750}.note-chips{display:flex;flex-wrap:wrap;gap:8px;margin-top:13px}.note-chips button,.mini-notes span{border:0;border-radius:20px;background:#e8f3ff;color:#4e91dd;padding:6px 10px;font-size:11px}.note-chips button span{font-size:15px;margin-left:5px}.empty-hint{font-size:11px;color:#becbda}.details{margin:23px 0}.summary{display:flex;align-items:center;gap:14px;margin-top:28px;padding:18px;border:1px solid #d9eafa;border-radius:10px;background:#f4f9ff}.summary-icon{font-size:23px;color:#468ee7}.summary strong{font-size:12px}.summary p{font-size:11px;color:#8ba0b8;margin:4px 0 0}.library-toolbar{display:flex;align-items:center;justify-content:space-between;margin-bottom:24px;color:#9cadbf;font-size:11px}.search-box{background:white;border:1px solid #e2eaf3;border-radius:9px;padding:0 13px;color:#9db4d0;font-size:21px;display:flex;align-items:center;width:330px}.search-box input{border:0;outline:none;padding:12px 8px;width:100%;font-size:12px}.empty-state{background:white;border:1px solid #e6edf7;border-radius:16px;text-align:center;padding:80px 20px}.empty-state>div{font-size:50px;color:#96c5f7}.empty-state h2{font-family:Georgia,serif;font-size:25px;font-weight:500;margin:10px}.empty-state p{color:#92a5b9;font-size:12px;margin-bottom:24px}.fragrance-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(245px,1fr));gap:18px}.fragrance-card{background:white;border-radius:13px;overflow:hidden}.fragrance-art{height:170px;background:#fff;display:grid;place-items:center;color:#a5c7ed;font-size:65px}.fragrance-art img{max-width:100%;max-height:100%;object-fit:contain}.fragrance-info{padding:19px}.fragrance-kicker{font-size:9px;letter-spacing:1.3px;text-transform:uppercase;color:#7ca7d5;font-weight:800}.fragrance-info h2{font-family:Georgia,serif;font-size:21px;font-weight:500;margin:8px 0 2px}.fragrance-info p{color:#95a7ba;font-size:12px;margin:0}.mini-notes{display:flex;flex-wrap:wrap;gap:5px;margin-top:15px}.rating{border-top:1px solid #edf2f7;margin-top:17px;padding-top:13px;color:#48aa8e;font-weight:800}.rating span{font-weight:400;color:#a6b3c2;margin-left:5px;font-size:10px}@media(max-width:800px){.sidebar{width:64px;padding:25px 8px}.brand-mark{padding:0 4px 45px}.brand-mark>span:last-child,.sidebar-label,.nav-link:not(.active),.nav-link.active{font-size:0}.nav-link{justify-content:center;padding:12px}.nav-link>span{font-size:21px}.nav-link em,.side-bottom{display:none}.topbar{padding:0 22px}.page-content{padding:35px 20px}.progress button>span:last-child{display:none}.progress{justify-content:center}.connector{max-width:75px}.choice-grid{gap:7px}.choice-grid button{padding:12px 7px;gap:5px;flex-direction:column}.radio-dot{display:none}}@media(max-width:550px){.sidebar{display:none}.heading h1{font-size:31px}.step-count{display:none}.form-card{padding:25px 20px}.form-grid{grid-template-columns:1fr}.field.wide{grid-column:auto}.topbar{height:60px}.search-box{width:220px}}
  .card-actions{border-top:1px solid #edf2f7;padding:11px 19px;display:flex;align-items:center;gap:10px;flex-wrap:wrap}.card-actions button{border:0;background:transparent;color:#8295aa;font-size:11px;font-weight:700;padding:4px 0}.card-actions .view-action{color:#3789e9;margin-right:auto}.card-actions .edit-action{color:#6f88a5;margin-right:0}.card-actions .delete-action{color:#d96677}
  .notes-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(190px,1fr));gap:18px}.note-card{background:white;border-radius:13px;overflow:hidden}.note-art{height:155px;background:#fff;display:grid;place-items:center;color:#a5c7ed;font-size:56px}.note-art img{width:100%;height:100%;object-fit:cover}.note-card-body{padding:16px}.note-card-body h2{font-family:Georgia,serif;font-size:19px;font-weight:500;margin:0 0 4px}.note-card-body p{color:#9aaec3;font-size:11px;margin:0 0 15px}.note-image-form{border-top:1px solid #edf2f7;padding-top:12px;display:flex;flex-direction:column;gap:9px}.note-image-form input{border:1px solid #dae5f1;border-radius:6px;padding:9px 10px;font-size:11px;width:100%;min-width:0;outline:none}.note-image-form input:focus{border-color:#4d97ee}.note-image-form>div{display:flex;align-items:center;justify-content:space-between;gap:6px}.note-image-form button{border:0;background:none;color:#3789e9;font-size:10px;font-weight:750;padding:0}.note-image-form button:last-child:not(:first-child){color:#d56877}.note-image-form button:disabled{opacity:.55}
  .detail-page{max-width:1120px}.detail-back{border:0;background:none;color:#658fbd;font-size:12px;font-weight:700;padding:0;margin-bottom:27px}.detail-layout{display:grid;grid-template-columns:minmax(250px,350px) minmax(0,1fr);gap:48px}.detail-left{min-width:0}.detail-image{height:390px;border:1px solid #e5edf6;border-radius:16px;background:#fff;display:grid;place-items:center;overflow:hidden}.detail-image img{width:100%;height:100%;object-fit:contain;padding:20px}.detail-placeholder{display:flex;flex-direction:column;align-items:center;gap:9px;color:#aac7e9}.detail-placeholder span{font-size:110px;line-height:1}.detail-placeholder small{letter-spacing:2px;font-size:9px;font-weight:800}.detail-image-caption{text-align:center;color:#9aabc0;font-size:11px;margin:13px 0 22px}.detail-edit{width:100%;border:1px solid #d9e8f8;background:#fff;color:#4c90df;border-radius:9px;padding:12px;font-weight:700;font-size:12px}.detail-main{min-width:0;padding-top:8px}.detail-kicker{font-size:11px;color:#5798e0;font-weight:800;letter-spacing:1.8px;text-transform:uppercase;margin-bottom:8px}.detail-main h1{font-size:42px;margin-bottom:4px}.detail-brand{font-size:19px;color:#899fb9}.year-pill{display:inline-block;margin-left:10px;background:#edf3f9;color:#8498af;border-radius:6px;padding:4px 7px;vertical-align:middle;font-size:11px}.audience-pill{display:inline-block;border-radius:20px;background:#eaf4ff;color:#498ddd;padding:6px 12px;margin-top:16px;font-size:11px;font-weight:750}.detail-description{line-height:1.75;color:#71869f;margin:24px 0 30px;white-space:pre-wrap}.detail-section{border-top:1px solid #e6eef7;padding-top:23px;margin-top:26px}.detail-section-title{display:flex;align-items:center;gap:10px;margin-bottom:19px}.detail-section-title>span{width:29px;height:29px;border-radius:8px;background:#eaf4ff;color:#4b91e8;display:grid;place-items:center;font-size:17px}.detail-section-title h2{font-family:Georgia,serif;font-size:21px;font-weight:500;margin:0}.pyramid-row{padding:13px 0 20px;border-bottom:1px solid #edf2f7}.pyramid-label{font-size:10px;text-transform:uppercase;letter-spacing:1.4px;font-weight:800;color:#8498ae;display:flex;align-items:center;gap:8px;margin-bottom:12px}.pyramid-label>span{color:#5ca0ee;font-size:18px}.pyramid-notes{display:flex;flex-wrap:wrap;gap:15px}.pyramid-note{display:flex;align-items:center;gap:7px;color:#438ad7;font-size:12px;font-weight:650}.pyramid-note-image{width:30px;height:30px;border-radius:50%;background:#e9f3ff;display:grid;place-items:center;overflow:hidden;color:#8dbcec}.pyramid-note-image img{width:100%;height:100%;object-fit:cover}.no-notes{font-size:11px;color:#bdcad8}.detail-meta{display:grid;grid-template-columns:1fr 1fr;gap:25px;margin-top:32px}.detail-meta section{border:1px solid #e6edf7;border-radius:12px;background:white;padding:19px}.detail-meta .detail-section-title{margin-bottom:10px}.detail-meta .detail-section-title h2{font-size:18px}.detail-meta p{color:#6797cc;margin:0;font-size:13px}.detail-rating strong{font-size:30px;color:#48ad8b}.detail-rating span{font-size:12px;color:#a5b5c5;margin-left:4px}.rating-bar{height:6px;background:#e7f4f0;border-radius:5px;margin:10px 0 6px;overflow:hidden}.rating-bar div{height:100%;background:#63c6a6;border-radius:5px}.detail-meta small{color:#a0afbf;font-size:10px}@media(max-width:850px){.detail-layout{grid-template-columns:1fr;gap:25px}.detail-image{height:300px}.detail-left{max-width:500px;width:100%;margin:auto}.detail-meta{grid-template-columns:1fr 1fr}}@media(max-width:550px){.detail-meta{grid-template-columns:1fr}.detail-main h1{font-size:34px}}
  .new-note-form{display:flex;gap:10px;background:#fff;border:1px solid #e3edf7;border-radius:12px;padding:13px;margin-bottom:23px}.new-note-form input,.rename-note-form input{border:1px solid #dae5f1;border-radius:7px;padding:11px 12px;outline:none;min-width:0;flex:1}.new-note-form input:focus,.rename-note-form input:focus{border-color:#4d97ee}.new-note-form button,.rename-note-form button[type=submit]{border:0;border-radius:7px;background:#448ff0;color:white;font-size:11px;font-weight:750;padding:0 17px}.new-note-form button:disabled,.rename-note-form button:disabled{opacity:.55}.rename-note-form{display:flex;gap:5px;flex-wrap:wrap;margin-bottom:8px}.rename-note-form input{width:100%;flex-basis:100%}.rename-note-form button{border:0;background:transparent;color:#7f96ae;font-size:11px;padding:6px}.note-manage-actions{display:flex;gap:12px;flex-wrap:wrap;border-top:1px solid #edf2f7;margin-top:12px;padding-top:11px;align-items:center}.note-manage-actions button{border:0;background:transparent;padding:0;color:#6f92b8;font-size:11px;font-weight:700}.note-manage-actions button.danger{color:#d56877}
</style>
