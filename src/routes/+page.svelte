<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { isTauri } from '@tauri-apps/api/core';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { check, type Update } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { activeBackendTasks, backend, type AccordBar, type BackupStatus, type CsvImportReport, type Fragrance, type NoteImage, type ParfumoNote, type ParfumoStatus, type WebdavBackup } from '$lib/backend';
  import AccordBars from '$lib/AccordBars.svelte';
  import NavIcon from '$lib/NavIcon.svelte';
  import './redesign.css';

  const blank = (): Fragrance => ({ collection: '', name: '', brand: '', year: new Date().getFullYear(), audience: 'Unisex', concentration: '', description: '', topNotes: [], heartNotes: [], baseNotes: [], perfumer: '', bottleRating: null, ratings: { scent: null, longevity: null, sillage: null, bottle: null, valueForMoney: null }, ratingCount: null, imageUrl: '', shelf: 'Collection', bottlesOwned: 0, bottlesFinished: 0, favourite: false, priceBought: null, currentPrice: null });
  const ratingFields = [{ key: 'scent', label: 'Scent' }, { key: 'longevity', label: 'Longevity' }, { key: 'sillage', label: 'Sillage' }, { key: 'bottle', label: 'Bottle' }, { key: 'valueForMoney', label: 'Value for money' }] as const;
  function overallRating(ratings: Fragrance['ratings']): number | null {
    const values = ratingFields.map(({ key }) => ratings[key]).filter((value): value is number => value !== null && Number.isFinite(value));
    return values.length ? Math.round(values.reduce((sum, value) => sum + value, 0) / values.length * 10) / 10 : null;
  }
  let fragrance = $state<Fragrance>(blank());
  let entries = $state<Fragrance[]>([]);
  let step = $state(0);
  let view = $state<'form' | 'library' | 'notes' | 'detail' | 'settings'>('library');
  let activeShelf = $state<'Collection' | 'Wishlist' | 'Empties'>('Collection');
  let categoryMenuOpen = $state(false);
  let concentrationMenuOpen = $state(false);
  let brandSuggestionsOpen = $state(false);
  let highlightedBrandIndex = $state(-1);
  let sidebarCollapsed = $state(false);
  let profileName = $state('Scentkeeper');
  let profilePicture = $state('');
  let draftProfileName = $state('');
  let draftProfilePicture = $state('');
  let profilePictureUrl = $state('');
  let profileDialogOpen = $state(false);
  let mobileSidebarOpen = $state(false);
  let selectedId = $state<number | null>(null);
  let fragranceAccords = $state<AccordBar[]>([]);
  let fragranceAccordsLoading = $state(false);
  let fragranceAccordsError = $state('');
  let accordRequestVersion = 0;
  let noteImages = $state<NoteImage[]>([]);
  let noteSearch = $state('');
  let newNoteName = $state('');
  let parfumoNotes = $state<ParfumoNote[]>([]);
  let searchingParfumoNotes = $state(false);
  let parfumoNoteError = $state('');
  let savingParfumoNote = $state(false);
  let parfumoResultsOpen = $state(false);
  let highlightedParfumoIndex = $state(-1);
  let noteSearchVersion = 0;
  let editingNote = $state('');
  let editedNoteName = $state('');
  let savingNoteImage = $state('');
  let noteImageDrafts = $state<Record<string, string>>({});
  let editingNoteCategories = $state('');
  let noteCategoryDraft = $state('');
  let refreshingNoteMetadata = $state(false);
  let error = $state('');
  let success = $state('');
  let saving = $state(false);
  let visibleBackendTask = $state('');
  let backupStatus = $state<BackupStatus>({ state: 'idle', error: null });
  let showBackupProgress = $state(false);
  let closeBackupDialogOpen = $state(false);
  let allowCloseWithPendingBackup = false;
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
  let backupFolder = $state<string | null>(null);
  let backupDialogOpen = $state(false);
  let choosingBackupFolder = $state(false);
  let webdavBackup = $state<WebdavBackup | null>(null);
  let webdavUrl = $state('');
  let webdavUsername = $state('');
  let webdavPassword = $state('');
  let webdavBusy = $state(false);
  let webdavSyncing = $state(false);
  let parfumoStatus = $state<ParfumoStatus | null>(null);
  let parfumoBusy = $state(false);
  let parfumoLoginOpen = $state(false);
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
  const noteImagesByName = $derived(new Map(noteImages.map((note) => [note.name.toLocaleLowerCase(), note])));
  const brandSuggestions = $derived.by(() => {
    const query = fragrance.brand.trim().toLocaleLowerCase();
    if (!query) return [];
    const brands = new Map<string, string>();
    for (const item of entries) {
      const brand = item.brand.trim();
      const key = brand.toLocaleLowerCase();
      if (brand && !brands.has(key)) brands.set(key, brand);
    }
    return [...brands.values()]
      .filter((brand) => {
        const name = brand.toLocaleLowerCase();
        return name.includes(query) && name !== query;
      })
      .sort((a, b) => Number(!a.toLocaleLowerCase().startsWith(query)) - Number(!b.toLocaleLowerCase().startsWith(query)) || a.localeCompare(b))
      .slice(0, 6);
  });
  const visibleNotes = $derived(allNotes.filter((note) => note.toLocaleLowerCase().includes(noteSearch.toLocaleLowerCase())));
  const selectedFragrance = $derived(entries.find((item) => item.id === selectedId));
  const visibleEntries = $derived(entries.filter((item) => (activeShelf === 'Collection' ? (collectionFilter === 'All' || (collectionFilter === 'Owned' ? item.shelf === 'Collection' : item.shelf === collectionFilter)) : item.shelf === activeShelf) && `${item.name} ${item.brand} ${item.collection}`.toLowerCase().includes(search.toLowerCase())));

  $effect(() => {
    const tasks = $activeBackendTasks;
    if (tasks.length === 0) {
      visibleBackendTask = '';
      return;
    }
    const timer = setTimeout(() => visibleBackendTask = tasks[tasks.length - 1], 350);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (backupStatus.state !== 'running' && backupStatus.state !== 'queued') {
      showBackupProgress = false;
      return;
    }
    const timer = setTimeout(() => showBackupProgress = true, 350);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const query = newNoteName.trim();
    const version = ++noteSearchVersion;
    parfumoNotes = [];
    highlightedParfumoIndex = -1;
    parfumoNoteError = '';
    searchingParfumoNotes = false;
    if (view !== 'notes' || !parfumoStatus?.connected || query.length < 2) return;
    const timer = setTimeout(async () => {
      searchingParfumoNotes = true;
      try {
        const results = await backend.searchParfumoNotes(query);
        if (version === noteSearchVersion) {
          parfumoNotes = results;
          highlightedParfumoIndex = -1;
        }
      } catch (e) {
        if (version === noteSearchVersion) parfumoNoteError = String(e);
      } finally {
        if (version === noteSearchVersion) searchingParfumoNotes = false;
      }
    }, 300);
    return () => clearTimeout(timer);
  });

  onMount(async () => {
    profileName = localStorage.getItem('scentkeep.profile.name') || 'Scentkeeper';
    profilePicture = localStorage.getItem('scentkeep.profile.picture') || '';

    let refreshedNoteImagesLoaded = false;
    const collectionRequest = Promise.allSettled([backend.listFragrances(), backend.listNoteImages()]).then(([fragrancesResult, noteImagesResult]) => {
      if (fragrancesResult.status === 'fulfilled') entries = fragrancesResult.value;
      else error = String(fragrancesResult.reason);
      if (noteImagesResult.status === 'fulfilled') {
        if (!refreshedNoteImagesLoaded) noteImages = noteImagesResult.value;
      } else error = String(noteImagesResult.reason);
    });
    if (!isTauri()) {
      await collectionRequest;
      return;
    }

    const backupSettingsRequest = Promise.allSettled([backend.getBackupFolder(), backend.getWebdavBackup()]).then(([backupFolderResult, webdavBackupResult]) => {
      if (backupFolderResult.status === 'fulfilled') backupFolder = backupFolderResult.value;
      if (webdavBackupResult.status === 'fulfilled') {
        webdavBackup = webdavBackupResult.value;
        if (webdavBackup) { webdavUrl = webdavBackup.url; webdavUsername = webdavBackup.username; }
      }
      if (backupFolderResult.status === 'fulfilled' && webdavBackupResult.status === 'fulfilled') backupDialogOpen = !backupFolder && !webdavBackup;
      else {
        const cause = backupFolderResult.status === 'rejected' ? backupFolderResult.reason : webdavBackupResult.status === 'rejected' ? webdavBackupResult.reason : '';
        error = `Could not load backup settings: ${String(cause)}`;
      }
    });
    const updateSupportRequest = invoke<boolean>('can_install_update').then((supported) => canInstallUpdate = supported);
    const parfumoRequest = backend.parfumoStatus().then(async (status) => {
      parfumoStatus = status;
      if (!status.connected) return;
      try {
        if (await backend.refreshParfumoNoteMetadata()) {
          const refreshedNoteImages = await backend.listNoteImages();
          refreshedNoteImagesLoaded = true;
          noteImages = refreshedNoteImages;
        }
      } catch { /* Metadata can be retried the next time Parfumo is connected. */ }
    }).catch((cause) => error = `Could not load Parfumo connection: ${String(cause)}`);

    await Promise.allSettled([collectionRequest, backupSettingsRequest, updateSupportRequest, parfumoRequest]);
    void checkForUpdates(true);
  });

  onMount(() => {
    if (!isTauri()) return;
    let disposed = false;
    let backupEventCount = 0;
    let stopBackupEvents: (() => void) | undefined;
    let stopCloseEvents: (() => void) | undefined;
    void (async () => {
      stopBackupEvents = await listen<BackupStatus>('backup-status', (event) => {
        backupEventCount += 1;
        backupStatus = event.payload;
        if (backupStatus.state !== 'running' && backupStatus.state !== 'queued') closeBackupDialogOpen = false;
      });
      if (disposed) { stopBackupEvents(); return; }
      const eventsBeforeStatus = backupEventCount;
      const initialBackupStatus = await backend.getBackupStatus();
      if (backupEventCount === eventsBeforeStatus) backupStatus = initialBackupStatus;
      stopCloseEvents = await getCurrentWindow().onCloseRequested((event) => {
        if (allowCloseWithPendingBackup) return;
        if (backupStatus.state === 'running' || backupStatus.state === 'queued') {
          event.preventDefault();
          closeBackupDialogOpen = true;
        }
      });
      if (disposed) stopCloseEvents();
    })().catch((cause) => error = `Could not monitor backups: ${String(cause)}`);
    return () => { disposed = true; stopBackupEvents?.(); stopCloseEvents?.(); };
  });

  async function closeWithPendingBackup() {
    allowCloseWithPendingBackup = true;
    try { await getCurrentWindow().close(); }
    catch (cause) { allowCloseWithPendingBackup = false; error = `Could not close the app: ${String(cause)}`; }
  }

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
  async function chooseBackupFolder() {
    choosingBackupFolder = true;
    try {
      const folder = await openDialog({ directory: true, multiple: false, title: 'Choose a CSV backup folder' });
      if (!folder || Array.isArray(folder)) return;
      await backend.setBackupFolder(folder);
      backupFolder = folder;
      backupDialogOpen = false;
      success = 'Automatic CSV backups are enabled.';
      error = '';
    } catch (e) { error = `Could not set backup folder: ${String(e)}`; }
    finally { choosingBackupFolder = false; }
  }
  async function saveWebdavBackup(event: SubmitEvent) {
    event.preventDefault();
    webdavBusy = true;
    try {
      webdavBackup = await backend.setWebdavBackup(webdavUrl, webdavUsername, webdavPassword);
      webdavUrl = webdavBackup.url;
      webdavUsername = webdavBackup.username;
      webdavPassword = '';
      backupDialogOpen = false;
      success = webdavBackup.needsSync
        ? 'WebDAV connected. Sync from WebDAV on the collection page before making changes.'
        : 'WebDAV connected. CSV backups will upload after changes.';
      error = '';
    } catch (e) { error = `Could not connect WebDAV backup: ${String(e)}`; }
    finally { webdavBusy = false; }
  }
  async function disconnectWebdavBackup() {
    webdavBusy = true;
    try {
      await backend.disconnectWebdavBackup();
      webdavBackup = null;
      webdavPassword = '';
      success = 'WebDAV backup disconnected.';
      error = '';
    } catch (e) { error = `Could not disconnect WebDAV backup: ${String(e)}`; }
    finally { webdavBusy = false; }
  }
  async function syncFromWebdav() {
    if (webdavSyncing) return;
    webdavSyncing = true;
    error = ''; success = '';
    try {
      const report = await backend.syncFromWebdav();
      if (webdavBackup) webdavBackup = { ...webdavBackup, needsSync: false };
      await refreshNotesAndFragrances();
      success = `Synced from WebDAV: ${report.fragrancesAdded} fragrances and ${report.notesAdded} notes added; ${report.fragrancesUpdated} fragrances and ${report.notesUpdated} notes updated.`;
    } catch (e) { error = `Could not sync from WebDAV: ${String(e)}`; }
    finally { webdavSyncing = false; }
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
    if (savingParfumoNote) return;
    error = ''; success = '';
    try {
      await backend.createNote(newNoteName);
      noteImages = await backend.listNoteImages();
      success = `${newNoteName.trim()} added to your notes.`;
      newNoteName = '';
    } catch (e) { error = String(e); }
  }
  function handleParfumoNoteKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      parfumoResultsOpen = false;
      highlightedParfumoIndex = -1;
    } else if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && parfumoStatus?.connected && parfumoNotes.length && !searchingParfumoNotes && !savingParfumoNote) {
      event.preventDefault();
      parfumoResultsOpen = true;
      highlightedParfumoIndex = event.key === 'ArrowDown'
        ? (highlightedParfumoIndex + 1) % parfumoNotes.length
        : highlightedParfumoIndex < 0 ? parfumoNotes.length - 1 : (highlightedParfumoIndex - 1 + parfumoNotes.length) % parfumoNotes.length;
      const index = highlightedParfumoIndex;
      void tick().then(() => {
        if (parfumoResultsOpen && highlightedParfumoIndex === index)
          document.getElementById(`parfumo-note-option-${index}`)?.scrollIntoView({ block: 'nearest' });
      });
    } else if (event.key === 'Enter' && parfumoResultsOpen && highlightedParfumoIndex >= 0 && parfumoNotes[highlightedParfumoIndex] && !searchingParfumoNotes && !savingParfumoNote) {
      event.preventDefault();
      void selectParfumoNote(parfumoNotes[highlightedParfumoIndex]);
    }
  }
  async function selectParfumoNote(note: ParfumoNote) {
    if (savingParfumoNote) return;
    error = ''; success = ''; savingParfumoNote = true; parfumoResultsOpen = false; highlightedParfumoIndex = -1;
    try {
      await backend.saveParfumoNote(note);
      noteImages = await backend.listNoteImages();
      newNoteName = '';
      success = `${note.name} saved from Parfumo.`;
    } catch (e) { error = String(e); }
    finally { savingParfumoNote = false; }
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
    return noteImagesByName.get(note.toLocaleLowerCase())?.imageUrl;
  }
  function parfumoThumbnail(imageUrl: string, width = 64) {
    if (!imageUrl.startsWith('https://media.parfumo.com/notes_png/')) return imageUrl;
    const thumbnail = new URL(imageUrl);
    thumbnail.searchParams.set('width', String(width));
    thumbnail.searchParams.set('aspect_ratio', '1:1');
    return thumbnail.toString();
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
  async function saveNoteCategories(note: string) {
    error = ''; success = '';
    try {
      const categories = noteCategoryDraft.split(',').map((category) => category.trim()).filter(Boolean);
      await backend.setNoteCategories(note, categories);
      noteImages = await backend.listNoteImages();
      editingNoteCategories = '';
      success = `Categories saved for ${note}.`;
    } catch (e) { error = String(e); }
  }
  async function refreshNoteMetadata() {
    if (refreshingNoteMetadata) return;
    error = ''; success = ''; refreshingNoteMetadata = true;
    try {
      const updated = await backend.refreshParfumoNoteMetadata();
      noteImages = await backend.listNoteImages();
      success = updated ? `Filled metadata for ${updated} ${updated === 1 ? 'note' : 'notes'}.` : 'No additional Parfumo metadata was found.';
    } catch (e) { error = String(e); }
    finally { refreshingNoteMetadata = false; }
  }
  function suggestionsFor(kind: NoteLayer) {
    const query = noteInputs[kind].trim().toLocaleLowerCase();
    return allNotes.filter((name) =>
      name.toLocaleLowerCase().includes(query) &&
      !fragrance[kind].some((added) => added.toLocaleLowerCase() === name.toLocaleLowerCase())
    ).slice(0, 6);
  }
  function selectBrand(brand: string) {
    fragrance.brand = brand;
    brandSuggestionsOpen = false;
    highlightedBrandIndex = -1;
  }
  function handleBrandKeydown(event: KeyboardEvent) {
    if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && brandSuggestions.length) {
      event.preventDefault();
      brandSuggestionsOpen = true;
      highlightedBrandIndex = event.key === 'ArrowDown'
        ? (highlightedBrandIndex + 1) % brandSuggestions.length
        : (highlightedBrandIndex - 1 + brandSuggestions.length) % brandSuggestions.length;
    } else if (event.key === 'Enter' && brandSuggestionsOpen && highlightedBrandIndex >= 0) {
      event.preventDefault();
      selectBrand(brandSuggestions[highlightedBrandIndex]);
    } else if (event.key === 'Escape') {
      brandSuggestionsOpen = false;
      highlightedBrandIndex = -1;
    }
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
  function showShelf(shelf: 'Collection' | 'Wishlist' | 'Empties') { activeShelf = shelf; view = 'library'; error = ''; success = ''; mobileSidebarOpen = false; accordRequestVersion += 1; fragranceAccordsLoading = false; }
  function showSettings() { view = 'settings'; error = ''; success = ''; mobileSidebarOpen = false; }
  async function openParfumoLogin() {
    parfumoBusy = true; error = ''; success = '';
    try { await backend.openParfumoLogin(); parfumoLoginOpen = true; }
    catch (e) { error = `Could not open Parfumo: ${String(e)}`; }
    finally { parfumoBusy = false; }
  }
  async function saveParfumoConnection() {
    parfumoBusy = true; error = ''; success = '';
    try {
      parfumoStatus = await backend.saveParfumoSession();
      parfumoLoginOpen = false;
      success = 'Parfumo browser session saved in the system keyring.';
    } catch (e) { error = `Could not save Parfumo session: ${String(e)}`; }
    finally { parfumoBusy = false; }
  }
  async function disconnectParfumo() {
    parfumoBusy = true; error = ''; success = '';
    try {
      await backend.disconnectParfumo();
      parfumoStatus = { connected: false, cookieCount: 0 };
      parfumoLoginOpen = false;
      success = 'Parfumo connection removed.';
    } catch (e) { error = `Could not disconnect Parfumo: ${String(e)}`; }
    finally { parfumoBusy = false; }
  }
  async function resetParfumoBrowser() {
    if (!window.confirm('Clear the embedded Parfumo browser data and close its window?')) return;
    parfumoBusy = true; error = ''; success = '';
    try {
      await backend.resetParfumoBrowser();
      parfumoLoginOpen = false;
      parfumoStatus = { connected: false, cookieCount: 0 };
      success = 'Parfumo browser data cleared. Open Parfumo again to retry.';
    } catch (cause) { error = `Could not reset Parfumo browser: ${String(cause)}`; }
    finally { parfumoBusy = false; }
  }
  function startNew() {
    fragrance = { ...blank(), shelf: activeShelf };
    mobileSidebarOpen = false;
    noteInputs = { topNotes: '', heartNotes: '', baseNotes: '' };
    activeNoteInput = null;
    duplicateNoteLayer = null;
    step = 0; concentrationMenuOpen = false; view = 'form'; error = ''; success = '';
  }
  async function loadFragranceAccords(fragranceId: number) {
    const version = ++accordRequestVersion;
    fragranceAccords = [];
    fragranceAccordsError = '';
    fragranceAccordsLoading = true;
    try {
      const accords = await backend.getFragranceAccords(fragranceId);
      if (version === accordRequestVersion) fragranceAccords = accords;
    } catch (cause) {
      if (version === accordRequestVersion) fragranceAccordsError = String(cause);
    } finally {
      if (version === accordRequestVersion) fragranceAccordsLoading = false;
    }
  }
  function openDetail(item: Fragrance) {
    openCardMenuId = null; selectedId = item.id ?? null; activeShelf = item.shelf; view = 'detail'; error = ''; success = '';
    fragranceAccords = [];
    fragranceAccordsError = '';
    if (item.id !== undefined) void loadFragranceAccords(item.id);
  }
  function startEdit(item: Fragrance) {
    fragrance = { ...item, ratings: { ...blank().ratings, ...item.ratings }, topNotes: [...item.topNotes], heartNotes: [...item.heartNotes], baseNotes: [...item.baseNotes] };
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
    if (saving) return;
    error = ''; success = '';
    if (!fragrance.name.trim() || !fragrance.brand.trim()) {
      error = 'A fragrance name and brand are required.'; step = 0; return;
    }
    saving = true;
    try {
      fragrance.bottleRating = overallRating(fragrance.ratings) ?? fragrance.bottleRating;
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
  <title>Scentkeep: Fragrance collection</title>
  <meta name="description" content="A personal fragrance catalogue" />
</svelte:head>

<div class="app-shell" class:sidebar-collapsed={sidebarCollapsed} class:mobile-sidebar-open={mobileSidebarOpen}>
  {#if showBackupProgress}
    <div class="background-task" role="status"><span class="save-spinner" aria-hidden="true"></span>Backing up in the background…</div>
  {:else if backupStatus.state === 'error'}
    <div class="background-task backup-task-error" role="alert"><span>Backup failed: {backupStatus.error}</span><button type="button" onclick={() => backend.retryBackup().catch((cause) => error = String(cause))}>Retry</button></div>
  {:else if visibleBackendTask}
    <div class="background-task" role="status"><span class="save-spinner" aria-hidden="true"></span>{visibleBackendTask}</div>
  {/if}
  <aside class="sidebar" aria-label="Main navigation">
    <button class="drawer-close" type="button" aria-label="Close navigation" onclick={() => mobileSidebarOpen = false}><NavIcon name="back" /></button>
    <button class="sidebar-toggle" type="button" aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"} aria-expanded={!sidebarCollapsed} onclick={() => sidebarCollapsed = !sidebarCollapsed}><NavIcon name={sidebarCollapsed ? 'forward' : 'back'} /></button>
    <div class="brand-mark"><img class="brand-icon" src="/aroma.svg" alt="" /><span>Scentkeep<small>The fragrance journal</small></span></div>
    {#each ['Collection', 'Wishlist', 'Empties'] as shelf}
      <button title={shelf} class:active={view === 'library' && activeShelf === shelf} class="nav-link" onclick={() => showShelf(shelf as typeof activeShelf)}><span class="nav-icon"><NavIcon name={shelf === 'Wishlist' ? 'wishlist' : shelf === 'Collection' ? 'collection' : 'empties'} size={18} /></span><span class="nav-text">{shelf}</span><em>{entries.filter((item) => item.shelf === shelf).length}</em></button>
    {/each}
    <button title="Add fragrance" class:active={view === 'form'} class="nav-link" onclick={startNew}><span class="nav-icon"><NavIcon name="add" size={18} /></span><span class="nav-text">Add fragrance</span></button>
    <button title="Perfume notes" class:active={view === 'notes'} class="nav-link" onclick={() => { view = 'notes'; error = ''; success = ''; mobileSidebarOpen = false; }}><span class="nav-icon"><NavIcon name="notes" size={18} /></span><span class="nav-text">Perfume notes</span><em>{allNotes.length}</em></button>
    <button title="Settings" class:active={view === 'settings'} class="nav-link" onclick={showSettings}><span class="nav-icon"><NavIcon name="settings" size={18} /></span><span class="nav-text">Settings</span></button>
    {#if entries.length === 0}<div class="side-bottom"><strong>Your scent story starts here.</strong><p>Keep every note, memory and discovery in one beautiful place.</p></div>{/if}
  </aside>

  <main>
    <header class="topbar"><div class="topbar-inner"><button class="mobile-menu" type="button" aria-label="Toggle navigation" aria-expanded={mobileSidebarOpen} onclick={() => mobileSidebarOpen = !mobileSidebarOpen}><NavIcon name="menu" /></button><button class="top-back" type="button" aria-label="Back to collection" onclick={() => showShelf('Collection')}><NavIcon name="back" /></button><nav class="breadcrumb" aria-label="Breadcrumb">{#if view === 'library'}<span aria-current="page">{activeShelf}</span>{:else if view === 'form' || view === 'detail'}{@const shelf = view === 'detail' ? (selectedFragrance?.shelf ?? 'Collection') : fragrance.shelf}<button onclick={() => showShelf(shelf)}>{shelf}</button><span>/</span><span aria-current="page">{view === 'form' ? (fragrance.id === undefined ? 'Add fragrance' : 'Edit fragrance') : (selectedFragrance?.name ?? 'Fragrance')}</span>{:else}<button onclick={() => showShelf('Collection')}>Collection</button><span>/</span><span aria-current="page">{view === 'settings' ? 'Settings' : 'Perfume notes'}</span>{/if}</nav><div class="top-actions"><button class="update-check" aria-label={checkingUpdate ? 'Checking for updates' : 'Check for updates'} title={checkingUpdate ? 'Checking for updates' : 'Check for updates'} onclick={() => checkForUpdates()} disabled={checkingUpdate || installingUpdate}><NavIcon name="refresh" size={18} />{checkingUpdate ? 'Checking…' : 'Check for updates'}</button><span class="local-badge" title="Saved locally"><span class="local-text">Saved locally</span></span><button class="avatar profile-avatar" type="button" aria-label="Edit profile" title="Edit profile" onclick={editProfile}>{#if profilePicture}<img src={profilePicture} alt="" />{:else}{profileName.trim().charAt(0).toUpperCase() || 'S'}{/if}</button></div></div></header>
    {#if updateMessage}
      <div class="update-notice" role="status"><span>{updateMessage}</span>{#if availableUpdate && !installingUpdate}{#if canInstallUpdate}<button onclick={installUpdate}>Update and restart</button>{:else}<button onclick={() => openUrl('https://github.com/reaperhound/scentkeep/releases/latest')}>Open release page</button>{/if}{/if}</div>
    {/if}
    {#if view === 'form'}
      <div class="page-content">
        <div class="heading"><div><h1>{fragrance.id === undefined ? 'Add a fragrance' : 'Edit fragrance'}</h1><p>Capture the details that make each scent unforgettable.</p></div><div class="step-count">STEP 0{step + 1} <span>/ 03</span></div></div>
        <div class="progress" aria-label="Form progress">{#each steps as label, index}<button class:current={step === index} class:done={step > index} onclick={() => { if (index < step) step = index; }}><span class="step-number">{step > index ? '✓' : `0${index + 1}`}</span><span>{label}</span></button>{#if index < 2}<div class:filled={step > index} class="connector"></div>{/if}{/each}</div>
        {#if error}<div class="message error" role="alert">{error}</div>{/if}
        <div class="form-card">
          {#if step === 0}
            <div class="card-heading"><div><h2>The essentials</h2><p>Start with the fragrance’s identity.</p></div></div>
            <div class="form-grid"><label class="field wide"><span class="field-label-row"><span class="field-label-text">Fragrance name <b>*</b></span></span><input placeholder="e.g. Odéon Sunset" bind:value={fragrance.name} /></label><div class="field brand-field">
              <label class="field-label-row" for="fragrance-brand"><span class="field-label-text">Brand / house <b>*</b></span></label>
              <div class="brand-input-wrap">
                <input
                  id="fragrance-brand"
                  role="combobox"
                  aria-autocomplete="list"
                  aria-expanded={brandSuggestionsOpen && brandSuggestions.length > 0}
                  aria-controls={brandSuggestionsOpen && brandSuggestions.length > 0 ? 'brand-suggestions' : undefined}
                  aria-activedescendant={brandSuggestionsOpen && highlightedBrandIndex >= 0 ? `brand-option-${highlightedBrandIndex}` : undefined}
                  autocomplete="off"
                  placeholder="e.g. Memo Paris"
                  bind:value={fragrance.brand}
                  onfocus={() => { brandSuggestionsOpen = true; highlightedBrandIndex = -1; }}
                  oninput={() => { brandSuggestionsOpen = true; highlightedBrandIndex = -1; }}
                  onblur={() => { brandSuggestionsOpen = false; highlightedBrandIndex = -1; }}
                  onkeydown={handleBrandKeydown}
                />
                {#if brandSuggestionsOpen && brandSuggestions.length > 0}
                  <div class="note-suggestions brand-suggestions" id="brand-suggestions" role="listbox" aria-label="Saved brands">
                    {#each brandSuggestions as suggestion, index}
                      <button
                        type="button"
                        id={`brand-option-${index}`}
                        role="option"
                        aria-selected={highlightedBrandIndex === index}
                        class:highlighted={highlightedBrandIndex === index}
                        tabindex="-1"
                        onpointerdown={(event) => event.preventDefault()}
                        onmouseenter={() => highlightedBrandIndex = index}
                        onclick={() => selectBrand(suggestion)}
                      >{suggestion}</button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div><label class="field"><span class="field-label-row"><span class="field-label-text">Collection</span><span class="field-hint">Optional</span></span><input placeholder="e.g. Fleurs Bohèmes" bind:value={fragrance.collection} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Release year</span></span><input type="number" min="1800" max="2100" bind:value={fragrance.year} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Perfumer</span><span class="field-hint">Optional</span></span><input placeholder="e.g. Aliénor Massenet" bind:value={fragrance.perfumer} /></label><div class="field concentration-field"><div class="field-label-row"><span class="field-label-text">Concentration</span><span class="field-hint">Optional</span></div><div class="category-picker"><button type="button" class="category-trigger" aria-haspopup="listbox" aria-expanded={concentrationMenuOpen} onclick={() => { concentrationMenuOpen = !concentrationMenuOpen; categoryMenuOpen = false; }}>{fragrance.concentration || 'Select concentration'}<span class:rotated={concentrationMenuOpen} class="category-chevron"></span></button>{#if concentrationMenuOpen}<div class="category-options concentration-options" role="listbox" aria-label="Concentration"><button type="button" role="option" aria-selected={fragrance.concentration === ''} class:selected={fragrance.concentration === ''} onclick={() => { fragrance.concentration = ''; concentrationMenuOpen = false; }}>Select concentration</button>{#each concentrations as concentration}<button type="button" role="option" aria-selected={fragrance.concentration === concentration} class:selected={fragrance.concentration === concentration} onclick={() => { fragrance.concentration = concentration; concentrationMenuOpen = false; }}>{concentration}{#if fragrance.concentration === concentration}<span>✓</span>{/if}</button>{/each}</div>{/if}</div></div></div>
            <div class="section-divider"></div><div class="field category-field"><div class="field-label-row"><span class="field-label-text">Status</span></div><div class="category-picker"><button type="button" class="category-trigger" aria-haspopup="listbox" aria-expanded={categoryMenuOpen} onclick={() => categoryMenuOpen = !categoryMenuOpen}>{fragrance.shelf}<span class:rotated={categoryMenuOpen} class="category-chevron"></span></button>{#if categoryMenuOpen}<div class="category-options" role="listbox" aria-label="Status">{#each ['Collection', 'Wishlist', 'Empties'] as shelf}<button type="button" role="option" aria-selected={fragrance.shelf === shelf} class:selected={fragrance.shelf === shelf} onclick={() => { fragrance.shelf = shelf as typeof fragrance.shelf; categoryMenuOpen = false; }}>{shelf}{#if fragrance.shelf === shelf}<span>✓</span>{/if}</button>{/each}</div>{/if}</div></div><div class="field-title">Who is this fragrance for?</div><div class="choice-grid">{#each categories as category}<button type="button" class:selected={fragrance.audience === category.value} onclick={() => fragrance.audience = category.value}><span class="choice-icon"><img src={category.icon} alt="" /></span><strong>{category.label}</strong><span class="radio-dot"></span></button>{/each}</div>
          {:else if step === 1}
            <div class="card-heading"><div><h2>Fragrance pyramid</h2><p>Build the scent, layer by layer. Add each note separately.</p></div></div>
            {#each [{ key: 'topNotes', title: 'Top notes', hint: 'The first impression', icon: '✧', example: 'Candied apple' }, { key: 'heartNotes', title: 'Heart notes', hint: 'The heart of the scent', icon: '❀', example: 'Caramel' }, { key: 'baseNotes', title: 'Base notes', hint: 'The lasting impression', icon: '◆', example: 'Sandalwood' }] as group}
              {@const kind = group.key as 'topNotes' | 'heartNotes' | 'baseNotes'}
              <div class="note-section">
                <div class="note-title"><div><h3>{group.title}</h3><p>{group.hint}</p></div></div>
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
                        <div class="suggestions-heading">Saved notes</div>
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
            <div class="card-heading"><div><h2>Finishing touches</h2><p>Add context and a little more character.</p></div></div>
            <label class="field"><span class="field-label-row"><span class="field-label-text">Description</span><span class="field-hint">Optional</span></span><textarea rows="5" placeholder="What makes this fragrance special?" bind:value={fragrance.description}></textarea></label>
            <div class="form-grid details">
              {#each ratingFields as { key, label }}
                <label class="field"><span class="field-label-row"><span class="field-label-text">{label}</span><span class="field-hint">0 to 10</span></span><input type="number" min="0" max="10" step="0.1" bind:value={fragrance.ratings[key]} /></label>
              {/each}
            </div>
            <div class="detail-rating"><strong>Overall rating: {overallRating(fragrance.ratings)?.toFixed(1) ?? 'Not rated yet'}</strong></div>
            <div class="form-grid details"><label class="field"><span class="field-label-row"><span class="field-label-text">Bottles owned</span></span><input type="number" min="0" step="1" bind:value={fragrance.bottlesOwned} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Bottles finished</span></span><input type="number" min="0" step="1" bind:value={fragrance.bottlesFinished} /></label><label class="field"><span class="field-label-row"><span class="field-label-text">Price bought</span><span class="field-hint">Optional</span></span><input type="number" min="0" step="0.01" bind:value={fragrance.priceBought} /></label>{#if fragrance.shelf === 'Wishlist'}<label class="field"><span class="field-label-row"><span class="field-label-text">Current price</span><span class="field-hint">Optional</span></span><input type="number" min="0" step="0.01" bind:value={fragrance.currentPrice} /></label>{/if}</div><label class="favourite-field"><input type="checkbox" bind:checked={fragrance.favourite} /> Favourite</label><label class="field"><span class="field-label-row"><span class="field-label-text">Image URL</span><span class="field-hint">Optional</span></span><input type="url" placeholder="https://example.com/fragrance.jpg" bind:value={fragrance.imageUrl} /><small>A link to the bottle image. Leave blank to use a placeholder.</small></label>
            <div class="summary"><div><strong>Ready for your collection</strong><p>{fragrance.name || 'Your fragrance'} by {fragrance.brand || 'your chosen house'}, {fragrance.year}, {fragrance.audience}</p></div></div>
          {/if}
        </div>
        <div class="form-footer"><span>Fields marked <b>*</b> are required</span><div>{#if step > 0}<button class="back-button" disabled={saving} onclick={() => { step -= 1; error = ''; }}>← Back</button>{/if}{#if step < 2}<button class="primary-button" onclick={next}>Continue</button>{:else}<button class="primary-button" disabled={saving} onclick={save}>{#if saving}<span class="save-spinner" aria-hidden="true"></span>Saving…{:else}{fragrance.id === undefined ? 'Save fragrance' : 'Save changes'}{/if}</button>{/if}</div></div>
        {#if saving}<p class="save-status" role="status">Saving your fragrance…</p>{/if}
      </div>
    {:else if view === 'detail' && selectedFragrance}
      <div class="page-content detail-page">
        <button class="detail-back" onclick={() => view = 'library'}>← Back to collection</button>
        <div class="detail-layout">
          <div class="detail-left"><div class="detail-image">{#if selectedFragrance.imageUrl}<img src={selectedFragrance.imageUrl} alt={selectedFragrance.name} />{:else}<div class="detail-placeholder"><NavIcon name="bottle" size={72} /><small>No bottle image</small></div>{/if}</div><div class="detail-image-caption">{selectedFragrance.brand}, {selectedFragrance.year}</div><button class="detail-edit" onclick={() => startEdit(selectedFragrance)}>✎ Edit fragrance</button></div>
          <div class="detail-main"><h1>{selectedFragrance.name}</h1><div class="detail-kicker">{selectedFragrance.collection || 'Fragrance collection'}</div><div class="detail-brand">{selectedFragrance.brand}<span class="year-pill">{selectedFragrance.year}</span></div><div class="audience-pill">{selectedFragrance.audience}</div>{#if selectedFragrance.concentration}<div class="audience-pill concentration-pill">{selectedFragrance.concentration}</div>{/if}<div class="ownership-summary"><span>{selectedFragrance.shelf}</span><span>Owned: {selectedFragrance.bottlesOwned}</span><span>Finished: {selectedFragrance.bottlesFinished}</span>{#if selectedFragrance.favourite}<span>★ Favourite</span>{/if}{#if selectedFragrance.priceBought !== null}<span>Price bought: {selectedFragrance.priceBought.toFixed(2)}</span>{/if}{#if selectedFragrance.shelf === 'Wishlist' && selectedFragrance.currentPrice !== null}<span>Current price: {selectedFragrance.currentPrice.toFixed(2)}</span>{/if}</div>{#if selectedFragrance.description}<p class="detail-description">{selectedFragrance.description}</p>{/if}
            <AccordBars accords={fragranceAccords} loading={fragranceAccordsLoading} error={fragranceAccordsError} />
            <section class="detail-section"><div class="detail-section-title"><h2>Fragrance pyramid</h2></div>
              {#each [{ title: 'Top notes', notes: selectedFragrance.topNotes, icon: '✧' }, { title: 'Heart notes', notes: selectedFragrance.heartNotes, icon: '❀' }, { title: 'Base notes', notes: selectedFragrance.baseNotes, icon: '◆' }] as group}
                <div class="pyramid-row"><div class="pyramid-label">{group.title}</div><div class="pyramid-notes">{#each group.notes as note}{@const noteImage = imageFor(note)}<div class="pyramid-note"><div class="pyramid-note-image">{#if noteImage}<img src={parfumoThumbnail(noteImage, 50)} alt="" />{:else}<NavIcon name="notes" size={18} />{/if}</div><span>{note}</span></div>{/each}{#if group.notes.length === 0}<span class="no-notes">No notes recorded</span>{/if}</div></div>
              {/each}
            </section>
            <div class="detail-meta"><section><div class="detail-section-title"><h2>Perfumer</h2></div><p>{selectedFragrance.perfumer || 'Not recorded'}</p></section><section><div class="detail-section-title"><h2>Overall rating</h2></div>{#if selectedFragrance.bottleRating !== null}<div class="detail-rating"><strong>{selectedFragrance.bottleRating.toFixed(1)}</strong><span>/ 10</span></div><div class="rating-bar"><div style={`width: ${selectedFragrance.bottleRating * 10}%`}></div></div>{#if selectedFragrance.ratings && ratingFields.some(({ key }) => selectedFragrance.ratings[key] !== null)}<div class="rating-breakdown">{#each ratingFields as { key, label }}{#if selectedFragrance.ratings[key] !== null}<small>{label}: {selectedFragrance.ratings[key]?.toFixed(1)} / 10</small>{/if}{/each}</div>{/if}{:else}<p>Not rated yet</p>{/if}</section></div>
          </div>
        </div>
      </div>
    {:else if view === 'notes'}
      <div class="page-content">
        <div class="heading"><div><h1>Perfume notes</h1><p>See every note in your collection and give it an image.</p><p class="note-source-tip">Image tip: When adding a note image link, try Parfumo first if it has a suitable photo; its images are often clearer than Fragrantica’s.</p></div>
          <div class="csv-actions"><button class="back-button" disabled={csvBusy} onclick={() => noteCsvInput?.click()}><NavIcon name="import" size={18} /><span>Import CSV</span></button><button class="back-button" disabled={csvBusy} onclick={() => exportCsv('notes')}><NavIcon name="export" size={18} /><span>Export CSV</span></button><input type="file" accept=".csv,.xlsx,.xls,text/csv" bind:this={noteCsvInput} onchange={(event) => importCsv(event, 'notes')} hidden /></div>
        </div>
        {#if success}<div class="message success">{success}</div>{/if}{#if error}<div class="message error" role="alert">{error}</div>{/if}
        {#if csvReport?.kind === 'notes'}<div class="message import-result" role="status"><strong>Imported {csvReport.result.imported} {csvReport.result.imported === 1 ? 'note' : 'notes'}.</strong>{#if csvReport.result.duplicates.length}<p>Skipped {csvReport.result.duplicates.length} duplicate {csvReport.result.duplicates.length === 1 ? 'note' : 'notes'}:</p><ul>{#each csvReport.result.duplicates as duplicate}<li>{duplicate}</li>{/each}</ul>{/if}</div>{/if}
        <div class="parfumo-note-search">
          <form class="new-note-form" onsubmit={(event) => { event.preventDefault(); createNote(); parfumoResultsOpen = false; }}>
            <input aria-label="Search Parfumo or enter a note name" role="combobox" aria-autocomplete="list" aria-expanded={parfumoResultsOpen && parfumoStatus?.connected && newNoteName.trim().length >= 2} aria-controls={parfumoResultsOpen && parfumoStatus?.connected && newNoteName.trim().length >= 2 ? 'parfumo-note-results' : undefined} aria-activedescendant={parfumoResultsOpen && highlightedParfumoIndex >= 0 && !searchingParfumoNotes ? `parfumo-note-option-${highlightedParfumoIndex}` : undefined} autocomplete="off" placeholder="Search Parfumo notes or add your own..." disabled={savingParfumoNote} bind:value={newNoteName} onfocus={() => { parfumoResultsOpen = true; highlightedParfumoIndex = -1; }} oninput={() => { parfumoResultsOpen = true; highlightedParfumoIndex = -1; }} onkeydown={handleParfumoNoteKeydown} />
            <button type="submit" disabled={!newNoteName.trim() || savingParfumoNote}>+ Add note</button>
          </form>
          {#if savingParfumoNote}<p class="parfumo-save-status" role="status"><span class="save-spinner" aria-hidden="true"></span>Saving note…</p>{/if}
          {#if parfumoStatus?.connected && newNoteName.trim().length >= 2 && parfumoResultsOpen}
            <div class="parfumo-note-results" id="parfumo-note-results" role="listbox" aria-label="Parfumo note results">
              <div class="suggestions-heading">Parfumo notes</div>
              {#if searchingParfumoNotes}<p>Searching Parfumo…</p>
              {:else if parfumoNoteError}<p class="parfumo-result-error">{parfumoNoteError}</p>
              {:else if parfumoNotes.length === 0}<p>No matching Parfumo notes.</p>
              {:else}{#each parfumoNotes as note, index}
                <button type="button" id={`parfumo-note-option-${index}`} role="option" aria-selected={highlightedParfumoIndex === index} class:highlighted={highlightedParfumoIndex === index} tabindex="-1" disabled={savingParfumoNote} onclick={() => selectParfumoNote(note)}>
                  {#if note.imageUrl}<img src={parfumoThumbnail(note.imageUrl)} alt="" loading="lazy" decoding="async" />{:else}<span class="parfumo-result-placeholder"><NavIcon name="notes" size={18} /></span>{/if}
                  <span>{note.name}<small>Save Parfumo note</small></span>
                </button>
              {/each}{/if}
            </div>
          {/if}
        </div>
        {#if !parfumoStatus?.connected}<p class="note-source-tip">Connect Parfumo in Settings to search its notes. You can still add a note by name.</p>{/if}
        <div class="library-toolbar"><div class="search-box">⌕ <input aria-label="Search notes" placeholder="Search perfume notes..." bind:value={noteSearch} /></div><div class="note-toolbar-actions">{#if parfumoStatus?.connected}<button type="button" disabled={refreshingNoteMetadata} onclick={refreshNoteMetadata}>{refreshingNoteMetadata ? 'Filling metadata…' : 'Fill missing metadata'}</button>{/if}<span>{visibleNotes.length} {visibleNotes.length === 1 ? 'note' : 'notes'}</span></div></div>
        {#if visibleNotes.length === 0}
          <div class="empty-state"><div><NavIcon name="notes" size={48} /></div><h2>{allNotes.length === 0 ? 'No notes in your collection yet' : 'No notes found'}</h2><p>{allNotes.length === 0 ? 'Add a note above, or add notes to a fragrance.' : 'Try another search term.'}</p>{#if allNotes.length === 0}<button class="primary-button" onclick={startNew}>Add a fragrance</button>{/if}</div>
        {:else}
          <div class="notes-grid">
            {#each visibleNotes as note (note)}
              {@const noteInfo = noteImagesByName.get(note.toLocaleLowerCase())}
              <article class="note-card">
                <div class="note-art">{#if noteInfo?.imageUrl}<img src={noteInfo.imageUrl} alt={note} loading="lazy" decoding="async" />{:else}<NavIcon name="notes" size={48} />{/if}</div>
                <div class="note-card-body">
                  {#if editingNote === note}
                    <form class="rename-note-form" onsubmit={(event) => { event.preventDefault(); renameNote(note); }}><input aria-label={`Rename ${note}`} bind:value={editedNoteName} /><button type="submit" disabled={!editedNoteName.trim()}>Save</button><button type="button" onclick={() => editingNote = ''}>Cancel</button></form>
                  {:else}<h2>{note}</h2>{/if}
                  <p>Used in {noteInfo?.usageCount ?? 0} fragrances</p>
                  {#if editingNoteCategories === note}
                    <form class="note-category-form" onsubmit={(event) => { event.preventDefault(); saveNoteCategories(note); }}><label for={`categories-${note}`}>Categories, separated by commas</label><input id={`categories-${note}`} bind:value={noteCategoryDraft} placeholder="Fruits, Green notes" /><div><button type="submit">Save categories</button><button type="button" onclick={() => editingNoteCategories = ''}>Cancel</button></div></form>
                  {:else}
                    {#if noteInfo?.categories.length}<div class="note-categories">{#each noteInfo.categories as category}<span>{category}</span>{/each}</div>{/if}
                    <button class="edit-categories-button" type="button" onclick={() => { editingNoteCategories = note; noteCategoryDraft = noteInfo?.categories.join(', ') ?? ''; }}>Edit categories</button>
                  {/if}
                  <div class="note-source-slot">{#if noteInfo?.parfumoUrl}<a class="note-source-link" href={noteInfo.parfumoUrl} target="_blank" rel="noreferrer">View on Parfumo</a>{/if}</div>
                  <form class="note-image-form" onsubmit={(event) => { event.preventDefault(); setNoteImage(note); }}>
                    <input type="url" aria-label={`Image link for ${note}`} placeholder="https://example.com/note.jpg" value={noteImageDrafts[note] ?? ''} oninput={(event) => noteImageDrafts[note] = event.currentTarget.value} />
                    <div><button type="submit" disabled={savingNoteImage === note || !noteImageDrafts[note]?.trim()}>{savingNoteImage === note ? 'Saving…' : noteInfo?.imageUrl ? 'Update image link' : '+ Add image link'}</button>{#if noteInfo?.imageUrl}<button type="button" onclick={() => removeNoteImage(note)}>Remove image</button>{/if}</div>
                  </form>
                  <div class="note-manage-actions"><button onclick={() => { editingNote = note; editedNoteName = note; }}>Rename</button><button class="danger" onclick={() => deleteNote(note)}>Delete note</button></div>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {:else if view === 'settings'}
      <div class="page-content settings-page">
        <div class="heading"><div><h1>Settings</h1><p>Manage how Scentkeep keeps your collection safe.</p></div></div>
        {#if success}<div class="message success" role="status">{success}</div>{/if}
        {#if error}<div class="message error" role="alert">{error}</div>{/if}
        <section class="settings-card" aria-labelledby="backup-settings-title">
          <div class="settings-card-header"><div><h2 id="backup-settings-title">Automatic backups</h2><p>Fragrances and notes are backed up as CSV files in the background after each change.</p></div><span class:configured={!!backupFolder} class="backup-status">{backupFolder ? 'On' : 'Not set'}</span></div>
          <div class="settings-field"><span>Backup folder</span><code>{backupFolder ?? 'No folder selected'}</code></div>
          <p class="settings-hint">{#if backupFolder}Files: scentkeep-fragrances.csv and scentkeep-notes.csv{:else}Choose a folder to start automatic backups.{/if}</p>
          <button class="primary-button" type="button" disabled={choosingBackupFolder || !isTauri()} onclick={chooseBackupFolder}>{choosingBackupFolder ? 'Choosing…' : backupFolder ? 'Change folder' : 'Choose folder'}</button>
        </section>
        <section class="settings-card webdav-card" aria-labelledby="webdav-settings-title">
          <div class="settings-card-header"><div><h2 id="webdav-settings-title">WebDAV backup</h2><p>Upload the same two CSV files to an HTTPS WebDAV folder in the background.</p></div><span class:configured={!!webdavBackup} class="backup-status">{webdavBackup ? 'Connected' : 'Not connected'}</span></div>
          {#if webdavBackup?.needsSync}<p class="settings-hint">This folder already has backups. Open your collection and select Sync from WebDAV before making changes; uploads are paused until then.</p>{/if}
          <form class="webdav-form" onsubmit={saveWebdavBackup}>
            <label class="field">WebDAV folder URL<input type="url" required placeholder="https://cloud.example.com/remote.php/dav/files/you/Scentkeep/" bind:value={webdavUrl} disabled={webdavBusy || !isTauri()} /></label>
            <label class="field">Username<input required autocomplete="username" bind:value={webdavUsername} disabled={webdavBusy || !isTauri()} /></label>
            <label class="field">{webdavBackup ? 'New app password' : 'App password'}<input type="password" required autocomplete="new-password" bind:value={webdavPassword} disabled={webdavBusy || !isTauri()} /></label>
            <p class="settings-hint">Use an existing folder and an app password from your WebDAV provider. The password is stored in your system keyring.</p>
            <div class="parfumo-actions"><button class="primary-button" type="submit" disabled={webdavBusy || !isTauri()}>{webdavBusy ? 'Connecting…' : webdavBackup ? 'Update connection' : 'Connect WebDAV'}</button>{#if webdavBackup}<button class="parfumo-disconnect-button" type="button" disabled={webdavBusy} onclick={disconnectWebdavBackup}>Disconnect</button>{/if}</div>
          </form>
        </section>
        <section class="settings-card parfumo-card" aria-labelledby="parfumo-settings-title">
          <div class="settings-card-header"><div><h2 id="parfumo-settings-title">Parfumo account</h2><p>Sign in on Parfumo and save the browser session to search its notes.</p></div><span class:configured={parfumoStatus?.connected} class="backup-status">{parfumoStatus?.connected ? 'Session saved' : 'Not connected'}</span></div>
          <p class="settings-hint">{#if parfumoLoginOpen}After signing in on Parfumo, return here and save the session.{:else if parfumoStatus?.connected}Your session is saved in the system keyring. Open Parfumo to refresh it, or disconnect to remove it.{:else}Open Parfumo and sign in there to connect your account.{/if}</p>
          <p class="settings-hint">If Cloudflare verification stays stuck, reset this browser's data and try again. If it still fails, try another network or contact Parfumo with the Ray ID shown on the page. You can still add notes by name.</p>
          <div class="parfumo-actions">
            <button class="primary-button" type="button" disabled={parfumoBusy || !isTauri()} onclick={openParfumoLogin}>{parfumoStatus?.connected ? 'Open Parfumo' : 'Connect Parfumo'}</button>
            {#if parfumoLoginOpen}<button class="parfumo-save-button" type="button" disabled={parfumoBusy} onclick={saveParfumoConnection}>{parfumoStatus?.connected ? 'Update session' : 'Save session'}</button>{/if}
            {#if parfumoStatus?.connected}<button class="parfumo-disconnect-button" type="button" disabled={parfumoBusy} onclick={disconnectParfumo}>Disconnect</button>{/if}
            {#if parfumoStatus && !parfumoStatus.connected}<button class="back-button" type="button" disabled={parfumoBusy} onclick={resetParfumoBrowser}>Reset browser data</button>{/if}
          </div>
        </section>
      </div>
    {:else}
      <div class="page-content collection-page">
        <div class="heading collection-heading"><div><h1>{activeShelf}</h1><p>A home for every scent worth remembering.</p></div></div>
        {#if success}<div class="message success">{success}</div>{/if}{#if error}<div class="message error">{error}</div>{/if}
        {#if webdavBackup?.needsSync}<div class="message import-result" role="status">This WebDAV folder already has backups. Sync them before making changes on this device.</div>{/if}
        {#if csvReport?.kind === 'fragrances'}<div class="message import-result" role="status"><strong>Imported {csvReport.result.imported} {csvReport.result.imported === 1 ? 'fragrance' : 'fragrances'}.</strong>{#if csvReport.result.duplicates.length}<p>Skipped {csvReport.result.duplicates.length} duplicate {csvReport.result.duplicates.length === 1 ? 'fragrance' : 'fragrances'}:</p><ul>{#each csvReport.result.duplicates as duplicate}<li>{duplicate}</li>{/each}</ul>{/if}</div>{/if}
        <div class="collection-controls"><div class="collection-search"><label class="search-box"><NavIcon name="search" size={18} /><input aria-label="Search fragrances" placeholder="Search fragrances or brands..." bind:value={search} /></label><button class="filter-button" type="button" aria-label={collectionFilter === 'Owned' ? 'Show all fragrances' : 'Show owned fragrances'} title={collectionFilter === 'Owned' ? 'Show all fragrances' : 'Show owned fragrances'} onclick={() => collectionFilter = collectionFilter === 'Owned' ? 'All' : 'Owned'}><NavIcon name="filter" size={18} /><span>Filters</span></button></div><div class="csv-actions">{#if webdavBackup}<button class="back-button" type="button" disabled={webdavSyncing} onclick={syncFromWebdav}><NavIcon name="refresh" size={18} /><span>{webdavSyncing ? 'Syncing…' : 'Sync from WebDAV'}</span></button>{/if}<button class="back-button" disabled={csvBusy} onclick={() => fragranceCsvInput?.click()}><NavIcon name="import" size={18} /><span>Import CSV</span></button><button class="back-button" disabled={csvBusy} onclick={() => exportCsv('fragrances')}><NavIcon name="export" size={18} /><span>Export CSV</span></button><button class="primary-button" onclick={startNew}><NavIcon name="add" size={18} /><span>Add fragrance</span></button><input type="file" accept=".csv,.xlsx,.xls,text/csv" bind:this={fragranceCsvInput} onchange={(event) => importCsv(event, 'fragrances')} hidden /></div></div>
        <div class="collection-subbar"><div class="shelf-filters">{#if activeShelf === 'Collection'}{#each ['All', 'Owned', 'Wishlist', 'Empties'] as filter}<button type="button" class:active={collectionFilter === filter} onclick={() => collectionFilter = filter as typeof collectionFilter}><NavIcon name={filter === 'All' ? 'grid' : filter === 'Owned' ? 'library' : filter === 'Wishlist' ? 'wishlist' : 'empties'} size={18} /><span>{filter}</span><span class="filter-count">{filter === 'All' ? entries.length : filter === 'Owned' ? entries.filter((item) => item.shelf === 'Collection').length : entries.filter((item) => item.shelf === filter).length}</span></button>{/each}{/if}</div><div class="collection-view"><span>{visibleEntries.length} {visibleEntries.length === 1 ? 'fragrance' : 'fragrances'}</span><div class="layout-switch"><button type="button" class:active={collectionLayout === 'grid'} aria-label="Grid view" onclick={() => collectionLayout = 'grid'}><NavIcon name="grid" size={18} /></button><button type="button" class:active={collectionLayout === 'list'} aria-label="List view" onclick={() => collectionLayout = 'list'}><NavIcon name="list" size={18} /></button></div></div></div>
        {#if visibleEntries.length === 0}<div class="empty-state"><div><NavIcon name="bottle" size={48} /></div><h2>{entries.length === 0 ? 'Your collection is waiting' : 'No fragrances found'}</h2><p>{entries.length === 0 ? 'Add your first fragrance to start your scent journal.' : 'Try another search or filter.'}</p>{#if entries.length === 0}<button class="primary-button" onclick={startNew}>Add your first fragrance <NavIcon name="forward" size={18} /></button>{/if}</div>
        {:else}<div class="fragrance-grid" class:list-layout={collectionLayout === 'list'}>{#each visibleEntries as item}<article class="fragrance-card" role="link" tabindex="0" aria-label={`View details for ${item.name}`} onclick={() => openDetail(item)} onkeydown={(event) => { if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); openDetail(item); } }}><div class="fragrance-art">{#if item.imageUrl}<img src={item.imageUrl} alt={item.name} />{:else}<NavIcon name="bottle" size={72} />{/if}<div class="card-menu-wrap"><button class="card-menu" type="button" aria-label={`Actions for ${item.name}`} aria-expanded={openCardMenuId === item.id} onclick={(event) => { event.stopPropagation(); openCardMenuId = openCardMenuId === item.id ? null : (item.id ?? null); }}>···</button>{#if openCardMenuId === item.id}<div class="card-dropdown" role="menu" aria-label={`Actions for ${item.name}`} onclick={(event) => event.stopPropagation()}><button type="button" role="menuitem" onclick={() => { openCardMenuId = null; startEdit(item); }}><NavIcon name="edit" size={16} /> Edit</button><button type="button" role="menuitem" class="delete-action" onclick={() => { openCardMenuId = null; remove(item); }}><NavIcon name="trash" size={16} /> Delete</button></div>{/if}</div></div><div class="fragrance-info"><h2>{item.name}</h2><div class="listing-meta"><img class="sex-icon" src={sexIconFor(item.audience)} alt={item.audience} title={item.audience} /><span class="concentration-label">{item.concentration || 'Concentration not set'}</span></div></div></article>{/each}</div>{/if}
      </div>
    {/if}
  </main>
</div>

{#if profileDialogOpen}<div class="dialog-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) profileDialogOpen = false; }}><dialog open class="profile-dialog" aria-labelledby="profile-title"><button class="dialog-close" type="button" aria-label="Close" onclick={() => profileDialogOpen = false}>×</button><h2 id="profile-title">Edit profile</h2><div class="profile-picture-preview">{#if draftProfilePicture}<img src={draftProfilePicture} alt="Profile preview" />{:else}<span>{draftProfileName.trim().charAt(0).toUpperCase() || 'S'}</span>{/if}</div><label class="field"><span class="field-label-row"><span class="field-label-text">Your name</span></span><input maxlength="60" bind:value={draftProfileName} placeholder="Your name" /></label><div class="profile-url-row"><input type="url" aria-label="Profile picture URL" placeholder="Or paste an image URL" bind:value={profilePictureUrl} /><button type="button" disabled={!profilePictureUrl.trim()} onclick={() => { draftProfilePicture = profilePictureUrl.trim(); }}>Use link</button></div>{#if draftProfilePicture}<button type="button" class="clear-picture" onclick={() => draftProfilePicture = ''}>Remove picture</button>{/if}<div class="dialog-actions"><button class="back-button" type="button" onclick={() => profileDialogOpen = false}>Cancel</button><button class="primary-button" type="button" onclick={saveProfile}>Save profile</button></div></dialog></div>{/if}

{#if backupDialogOpen}
  <div class="dialog-backdrop" role="presentation">
    <dialog open class="profile-dialog" aria-labelledby="backup-title">
      <div class="eyebrow">AUTOMATIC BACKUPS</div>
      <h2 id="backup-title">Set up automatic backups</h2>
      <p>After every change, Scentkeep backs up your fragrances and notes as two CSV files in a local folder or WebDAV folder without delaying your save.</p>
      {#if backupFolder}<p class="backup-path">Current folder: {backupFolder}</p>{/if}
      {#if error}<div class="message error" role="alert">{error}</div>{/if}
      <div class="dialog-actions">
        <button class="back-button" type="button" onclick={() => backupDialogOpen = false}>Later</button>
        <button class="back-button" type="button" onclick={() => { backupDialogOpen = false; view = 'settings'; }}>Set up WebDAV</button>
        <button class="primary-button" type="button" disabled={choosingBackupFolder} onclick={chooseBackupFolder}>{choosingBackupFolder ? 'Choosing…' : 'Choose folder'}</button>
      </div>
    </dialog>
  </div>
{/if}

{#if closeBackupDialogOpen}
  <div class="dialog-backdrop" role="presentation">
    <dialog open class="profile-dialog" aria-labelledby="backup-close-title">
      <h2 id="backup-close-title">Backup still running</h2>
      <p>Your changes are saved locally. Wait for the backup to finish, or quit now and retry it when Scentkeep next opens.</p>
      <div class="dialog-actions">
        <button class="back-button" type="button" onclick={() => closeBackupDialogOpen = false}>Wait for backup</button>
        <button class="primary-button" type="button" onclick={closeWithPendingBackup}>Quit and retry later</button>
      </div>
    </dialog>
  </div>
{/if}
