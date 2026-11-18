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
  import NavIcon from '$lib/NavIcon.svelte';
  import AppDialogs from '$lib/screens/AppDialogs.svelte';
  import CollectionScreen from '$lib/screens/CollectionScreen.svelte';
  import DetailsScreen from '$lib/screens/DetailsScreen.svelte';
  import EditorScreen from '$lib/screens/EditorScreen.svelte';
  import NotesScreen from '$lib/screens/NotesScreen.svelte';
  import SettingsScreen from '$lib/screens/SettingsScreen.svelte';
  import './redesign.css';

  const blank = (): Fragrance => ({ collection: '', name: '', brand: '', year: new Date().getFullYear(), audience: 'Unisex', concentration: '', description: '', topNotes: [], heartNotes: [], baseNotes: [], perfumer: '', bottleRating: null, ratings: { scent: null, longevity: null, sillage: null, bottle: null, valueForMoney: null }, ratingCount: null, imageUrl: '', shelf: 'Collection', bottlesOwned: 0, bottlesFinished: 0, favourite: false, priceBought: null, currentPrice: null });
  const ratingFields = [{ key: 'scent', label: 'Scent' }, { key: 'longevity', label: 'Longevity' }, { key: 'sillage', label: 'Sillage' }, { key: 'bottle', label: 'Bottle' }, { key: 'valueForMoney', label: 'Value for money' }] as const;
  function overallRating(ratings: Fragrance['ratings']): number | null {
    const values = ratingFields.map(({ key }) => ratings[key]).filter((value): value is number => value !== null && Number.isFinite(value));
    return values.length ? Math.round(values.reduce((sum, value) => sum + value, 0) / values.length * 10) / 10 : null;
  }
  let fragrance = $state<Fragrance>(blank());
  let importedFragranceNotes = $state<ParfumoNote[]>([]);
  let entries = $state<Fragrance[]>([]);
  let editorVersion = $state(0);
  let view = $state<'form' | 'library' | 'notes' | 'detail' | 'settings'>('library');
  let activeShelf = $state<'Collection' | 'Wishlist' | 'Empties'>('Collection');
  // Capture the outgoing list before Svelte replaces its DOM, then restore
  // after the destination (including its pagination) has rendered.
  const listScrollPositions = new Map<string, number>();
  let previousScreen = '';
  $effect.pre(() => {
    const screen = view === 'library' ? `library:${activeShelf}` : view;
    if (screen === previousScreen) return;
    if (previousScreen === 'notes' || previousScreen.startsWith('library:')) {
      listScrollPositions.set(previousScreen, window.scrollY);
    }
    previousScreen = screen;
    void tick().then(() => {
      if (previousScreen === screen) {
        window.scrollTo({ top: listScrollPositions.get(screen) ?? 0, behavior: 'instant' });
      }
    });
  });

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
  let notePage = $state(1);
  type CsvKind = 'fragrances' | 'notes';
  let noteListElement = $state<HTMLDivElement>();
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
  const LIST_PAGE_SIZE = 12;
  const allNotes = $derived(noteImages.map((note) => note.name).sort((a, b) => a.localeCompare(b)));
  const noteImagesByName = $derived(new Map(noteImages.map((note) => [note.name.toLocaleLowerCase(), note])));
  const visibleNotes = $derived(allNotes.filter((note) => note.toLocaleLowerCase().includes(noteSearch.toLocaleLowerCase())));
  const selectedFragrance = $derived(entries.find((item) => item.id === selectedId));
  const notePageCount = $derived(Math.max(1, Math.ceil(visibleNotes.length / LIST_PAGE_SIZE)));
  const currentNotePage = $derived(Math.min(notePage, notePageCount));
  const pagedNotes = $derived(visibleNotes.slice((currentNotePage - 1) * LIST_PAGE_SIZE, currentNotePage * LIST_PAGE_SIZE));

  $effect(() => {
    noteSearch;
    notePage = 1;
  });

  async function changeNotePage(page: number) {
    notePage = page;
    await tick();
    noteListElement?.scrollIntoView({
      behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
      block: 'start'
    });
  }

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
  function replaceNoteLocally(updated: NoteImage, previousName = updated.name) {
    const key = previousName.toLocaleLowerCase();
    const index = noteImages.findIndex((note) => note.name.toLocaleLowerCase() === key);
    noteImages = index < 0
      ? [...noteImages, updated]
      : noteImages.map((note, noteIndex) => noteIndex === index ? updated : note);
  }
  function noteNamesFor(item?: Fragrance) {
    const names = new Map<string, string>();
    if (!item) return names;
    for (const name of [...item.topNotes, ...item.heartNotes, ...item.baseNotes]) {
      const trimmed = name.trim();
      if (trimmed) names.set(trimmed.toLocaleLowerCase(), trimmed);
    }
    return names;
  }
  function updateFragranceLocally(previous: Fragrance | undefined, updated: Fragrance | undefined) {
    if (updated) {
      const index = entries.findIndex((item) => item.id === updated.id);
      entries = index < 0
        ? [updated, ...entries]
        : entries.map((item, itemIndex) => itemIndex === index ? updated : item);
    } else if (previous?.id !== undefined) {
      entries = entries.filter((item) => item.id !== previous.id);
    }

    const before = noteNamesFor(previous);
    const after = noteNamesFor(updated);
    const deltas = new Map<string, number>();
    for (const key of before.keys()) if (!after.has(key)) deltas.set(key, -1);
    for (const key of after.keys()) if (!before.has(key)) deltas.set(key, 1);
    if (!deltas.size) return;

    const existing = new Set(noteImages.map((note) => note.name.toLocaleLowerCase()));
    noteImages = noteImages.map((note) => {
      const delta = deltas.get(note.name.toLocaleLowerCase());
      return delta ? { ...note, usageCount: Math.max(0, note.usageCount + delta) } : note;
    });
    for (const [key, delta] of deltas) {
      if (delta > 0 && !existing.has(key)) {
        noteImages = [...noteImages, { name: after.get(key)!, imageUrl: '', parfumoUrl: '', categories: [], usageCount: delta }];
      }
    }
  }
  function rewriteNoteInFragrances(oldName: string, newName?: string) {
    const key = oldName.toLocaleLowerCase();
    const rewriteLayer = (layer: string[]) => newName
      ? layer.map((note) => note.toLocaleLowerCase() === key ? newName : note)
      : layer.filter((note) => note.toLocaleLowerCase() !== key);
    entries = entries.map((item) => {
      const isAffected = [...item.topNotes, ...item.heartNotes, ...item.baseNotes]
        .some((note) => note.toLocaleLowerCase() === key);
      return isAffected ? {
        ...item,
        topNotes: rewriteLayer(item.topNotes),
        heartNotes: rewriteLayer(item.heartNotes),
        baseNotes: rewriteLayer(item.baseNotes)
      } : item;
    });
  }
  function normalizedFragrance(item: Fragrance, id: number) {
    const normalizeNotes = (notes: string[]) => notes.map((note) => note.trim()).filter(Boolean);
    return {
      ...item,
      id,
      collection: item.collection.trim(),
      name: item.name.trim(),
      brand: item.brand.trim(),
      concentration: item.concentration.trim(),
      description: item.description.trim(),
      topNotes: normalizeNotes(item.topNotes),
      heartNotes: normalizeNotes(item.heartNotes),
      baseNotes: normalizeNotes(item.baseNotes),
      perfumer: item.perfumer.trim(),
      imageUrl: item.imageUrl.trim(),
      ratings: { ...item.ratings }
    };
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
      const created = await backend.createNote(newNoteName);
      replaceNoteLocally(created);
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
      const saved = await backend.saveParfumoNote(note);
      replaceNoteLocally(saved);
      newNoteName = '';
      success = `${note.name} saved from Parfumo.`;
    } catch (e) { error = String(e); }
    finally { savingParfumoNote = false; }
  }
  async function renameNote(oldName: string) {
    error = ''; success = '';
    try {
      const renamed = await backend.renameNote(oldName, editedNoteName);
      replaceNoteLocally(renamed, oldName);
      rewriteNoteInFragrances(oldName, renamed.name);
      success = `${oldName} renamed to ${renamed.name}.`;
      editingNote = ''; editedNoteName = '';
    } catch (e) { error = String(e); }
  }
  async function deleteNote(name: string) {
    if (!window.confirm(`Are you sure you want to delete the note "${name}"? It will also be removed from all fragrances.`)) return;
    error = ''; success = '';
    try {
      await backend.deleteNote(name);
      noteImages = noteImages.filter((note) => note.name.toLocaleLowerCase() !== name.toLocaleLowerCase());
      rewriteNoteInFragrances(name);
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
      const updated = await backend.setNoteImage(note, imageUrl);
      replaceNoteLocally(updated);
      noteImageDrafts[note] = '';
      success = `Image link saved for ${note}.`;
    } catch (e) { error = String(e); }
    finally { savingNoteImage = ''; }
  }
  async function removeNoteImage(note: string) {
    error = ''; success = '';
    try {
      const updated = await backend.removeNoteImage(note);
      replaceNoteLocally(updated);
      success = `Image removed from ${note}.`;
    } catch (e) { error = String(e); }
  }
  async function saveNoteCategories(note: string) {
    error = ''; success = '';
    try {
      const categories = noteCategoryDraft.split(',').map((category) => category.trim()).filter(Boolean);
      const updated = await backend.setNoteCategories(note, categories);
      replaceNoteLocally(updated);
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
    importedFragranceNotes = [];
    editorVersion += 1;
    mobileSidebarOpen = false;
    view = 'form'; error = ''; success = '';
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
    selectedId = item.id ?? null; view = 'detail'; error = ''; success = '';
    fragranceAccords = [];
    fragranceAccordsError = '';
    if (item.id !== undefined) void loadFragranceAccords(item.id);
  }
  function startEdit(item: Fragrance) {
    fragrance = { ...item, ratings: { ...blank().ratings, ...item.ratings }, topNotes: [...item.topNotes], heartNotes: [...item.heartNotes], baseNotes: [...item.baseNotes] };
    importedFragranceNotes = [];
    editorVersion += 1;
    view = 'form'; error = ''; success = ''; mobileSidebarOpen = false;
  }
  async function remove(item: Fragrance) {
    if (item.id === undefined || !window.confirm(`Are you sure you want to delete "${item.name}"?`)) return;
    error = ''; success = '';
    try {
      await backend.deleteFragrance(item.id);
      updateFragranceLocally(item, undefined);
      success = `${item.name} was deleted.`;
    } catch (e) { error = String(e); }
  }
  async function save() {
    if (saving) return;
    error = ''; success = '';
    if (!fragrance.name.trim() || !fragrance.brand.trim()) {
      error = 'A fragrance name and brand are required.'; return;
    }
    saving = true;
    try {
      fragrance.bottleRating = overallRating(fragrance.ratings) ?? fragrance.bottleRating;
      const wasEditing = fragrance.id !== undefined;
      const previous = wasEditing ? entries.find((item) => item.id === fragrance.id) : undefined;
      const id = fragrance.id === undefined
        ? await backend.saveFragrance(fragrance, importedFragranceNotes)
        : (await backend.updateFragrance(fragrance, importedFragranceNotes), fragrance.id);
      updateFragranceLocally(previous, normalizedFragrance(fragrance, id));
      const savedShelf = fragrance.shelf; fragrance = blank(); activeShelf = savedShelf;
      view = 'library'; success = wasEditing ? 'Fragrance updated.' : 'Fragrance saved to your collection.';
      importedFragranceNotes = [];
      try { noteImages = await backend.listNoteImages(); }
      catch (cause) { error = `Fragrance saved. Could not reload the Notes library: ${String(cause)}`; }
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
    <header class="topbar"><div class="topbar-inner"><button class="mobile-menu" type="button" aria-label="Toggle navigation" aria-expanded={mobileSidebarOpen} onclick={() => mobileSidebarOpen = !mobileSidebarOpen}><NavIcon name="menu" /></button><button class="top-back" type="button" aria-label="Back to collection" onclick={() => showShelf(activeShelf)}><NavIcon name="back" /></button><nav class="breadcrumb" aria-label="Breadcrumb">{#if view === 'library'}<span aria-current="page">{activeShelf}</span>{:else if view === 'form' || view === 'detail'}{@const shelf = view === 'detail' ? activeShelf : fragrance.shelf}<button onclick={() => showShelf(shelf)}>{shelf}</button><span>/</span><span aria-current="page">{view === 'form' ? (fragrance.id === undefined ? 'Add fragrance' : 'Edit fragrance') : (selectedFragrance?.name ?? 'Fragrance')}</span>{:else}<button onclick={() => showShelf('Collection')}>Collection</button><span>/</span><span aria-current="page">{view === 'settings' ? 'Settings' : 'Perfume notes'}</span>{/if}</nav><div class="top-actions"><button class="update-check" aria-label={checkingUpdate ? 'Checking for updates' : 'Check for updates'} title={checkingUpdate ? 'Checking for updates' : 'Check for updates'} onclick={() => checkForUpdates()} disabled={checkingUpdate || installingUpdate}><NavIcon name="refresh" size={18} />{checkingUpdate ? 'Checking…' : 'Check for updates'}</button><span class="local-badge" title="Saved locally"><span class="local-text">Saved locally</span></span><button class="avatar profile-avatar" type="button" aria-label="Edit profile" title="Edit profile" onclick={editProfile}>{#if profilePicture}<img src={profilePicture} alt="" />{:else}{profileName.trim().charAt(0).toUpperCase() || 'S'}{/if}</button></div></div></header>
    {#if updateMessage}
      <div class="update-notice" role="status"><span>{updateMessage}</span>{#if availableUpdate && !installingUpdate}{#if canInstallUpdate}<button onclick={installUpdate}>Update and restart</button>{:else}<button onclick={() => openUrl('https://github.com/reaperhound/scentkeep/releases/latest')}>Open release page</button>{/if}{/if}</div>
    {/if}
    {#if view === 'form'}
      {#key editorVersion}<EditorScreen bind:fragrance bind:importedNotes={importedFragranceNotes} {entries} noteNames={allNotes} {parfumoStatus} {error} {saving} onSave={save} />{/key}
    {:else if view === 'detail' && selectedFragrance}
      <DetailsScreen fragrance={selectedFragrance} {noteImages} accords={fragranceAccords} accordsLoading={fragranceAccordsLoading} accordsError={fragranceAccordsError} onBack={() => showShelf(activeShelf)} onEdit={() => startEdit(selectedFragrance)} />
    {:else if view === 'notes'}
      <NotesScreen
        notes={allNotes} {visibleNotes} {pagedNotes} notesByName={noteImagesByName}
        page={currentNotePage} pageCount={notePageCount} pageSize={LIST_PAGE_SIZE}
        bind:search={noteSearch} bind:newNoteName results={parfumoNotes}
        bind:resultsOpen={parfumoResultsOpen} bind:highlightedResult={highlightedParfumoIndex}
        searching={searchingParfumoNotes} searchError={parfumoNoteError} savingResult={savingParfumoNote} status={parfumoStatus}
        bind:editingNote bind:editedNoteName savingImage={savingNoteImage} bind:imageDrafts={noteImageDrafts}
        bind:editingCategories={editingNoteCategories} bind:categoryDraft={noteCategoryDraft}
        refreshingMetadata={refreshingNoteMetadata} {success} {error} {csvReport} {csvBusy} bind:listElement={noteListElement}
        onImport={(event) => importCsv(event, 'notes')} onExport={() => exportCsv('notes')} onCreate={createNote}
        onResultKeydown={handleParfumoNoteKeydown} onSelectResult={selectParfumoNote} onRefreshMetadata={refreshNoteMetadata}
        onStartFragrance={startNew} onRename={renameNote} onDelete={deleteNote} onSaveCategories={saveNoteCategories}
        onSetImage={setNoteImage} onRemoveImage={removeNoteImage} onChangePage={changeNotePage}
      />
    {:else if view === 'settings'}
      <SettingsScreen
        {success} {error} {backupFolder} {choosingBackupFolder} {webdavBackup}
        bind:webdavUrl bind:webdavUsername bind:webdavPassword {webdavBusy}
        {parfumoStatus} {parfumoLoginOpen} {parfumoBusy}
        onChooseFolder={chooseBackupFolder} onSaveWebdav={saveWebdavBackup} onDisconnectWebdav={disconnectWebdavBackup}
        onOpenParfumo={openParfumoLogin} onSaveParfumo={saveParfumoConnection}
        onDisconnectParfumo={disconnectParfumo} onResetParfumo={resetParfumoBrowser}
      />
    {/if}
    <div hidden={view !== 'library'}>
      <CollectionScreen
        {entries} {activeShelf} {success} {error} {csvReport} {csvBusy} {webdavBackup} {webdavSyncing}
        onSync={syncFromWebdav} onImport={(event) => importCsv(event, 'fragrances')} onExport={() => exportCsv('fragrances')}
        onAdd={startNew} onOpen={openDetail} onEdit={startEdit} onDelete={remove}
      />
    </div>
  </main>
</div>

<AppDialogs
  bind:profileOpen={profileDialogOpen}
  bind:profileName={draftProfileName}
  bind:profilePicture={draftProfilePicture}
  bind:profilePictureUrl
  bind:backupOpen={backupDialogOpen}
  {backupFolder}
  {choosingBackupFolder}
  backupError={error}
  bind:closeBackupOpen={closeBackupDialogOpen}
  onSaveProfile={saveProfile}
  onChooseBackupFolder={chooseBackupFolder}
  onShowWebdav={() => { backupDialogOpen = false; view = 'settings'; }}
  onCloseWithPendingBackup={closeWithPendingBackup}
/>
