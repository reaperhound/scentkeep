import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

const taskLabels: Record<string, string> = {
  list_fragrances: 'Loading fragrances…',
  get_fragrance_accords: 'Calculating main accords…',
  list_note_images: 'Loading perfume notes…',
  save_fragrance: 'Saving fragrance…',
  update_fragrance: 'Saving changes…',
  delete_fragrance: 'Deleting fragrance…',
  create_note: 'Adding note…',
  search_parfumo_notes: 'Searching Parfumo…',
  save_parfumo_note: 'Saving Parfumo note…',
  rename_note: 'Renaming note…',
  delete_note: 'Deleting note…',
  set_note_image: 'Saving note image…',
  remove_note_image: 'Removing note image…',
  set_note_categories: 'Saving note categories…',
  refresh_parfumo_note_metadata: 'Filling note metadata from Parfumo…',
  get_backup_folder: 'Loading backup settings…',
  set_backup_folder: 'Setting up local backups…',
  get_webdav_backup: 'Loading WebDAV settings…',
  set_webdav_backup: 'Connecting WebDAV backup…',
  disconnect_webdav_backup: 'Disconnecting WebDAV backup…',
  sync_from_webdav: 'Syncing from WebDAV…',
  backup_status: 'Checking backup status…',
  retry_backup: 'Retrying backup…',
  export_fragrances_csv: 'Exporting fragrances…',
  import_fragrances_csv: 'Importing fragrances…',
  export_notes_csv: 'Exporting notes…',
  import_notes_csv: 'Importing notes…',
  parfumo_status: 'Checking Parfumo connection…',
  open_parfumo_login: 'Opening Parfumo…',
  save_parfumo_session: 'Saving Parfumo session…',
  disconnect_parfumo: 'Disconnecting Parfumo…',
  reset_parfumo_browser: 'Clearing Parfumo browser data…'
};

const pendingTasks = new Map<number, string>();
let nextTaskId = 0;
export const activeBackendTasks = writable<string[]>([]);

async function invoke<T>(command: string, args?: Parameters<typeof tauriInvoke>[1]): Promise<T> {
  const taskId = ++nextTaskId;
  pendingTasks.set(taskId, taskLabels[command] ?? 'Working…');
  activeBackendTasks.set([...pendingTasks.values()]);
  try {
    return await tauriInvoke<T>(command, args);
  } finally {
    pendingTasks.delete(taskId);
    activeBackendTasks.set([...pendingTasks.values()]);
  }
}

export type Fragrance = {
  id?: number; collection: string; name: string; brand: string; year: number;
  audience: string; concentration: string; description: string; topNotes: string[]; heartNotes: string[];
  baseNotes: string[]; perfumer: string; bottleRating: number | null;
  ratings: { scent: number | null; longevity: number | null; sillage: number | null; bottle: number | null; valueForMoney: number | null };
  ratingCount: number | null; imageUrl: string;
  shelf: 'Collection' | 'Wishlist' | 'Empties'; bottlesOwned: number; bottlesFinished: number; favourite: boolean; priceBought: number | null; currentPrice: number | null;
};

export type NoteImage = { name: string; imageUrl: string; parfumoUrl: string; categories: string[]; usageCount: number };
export type AccordBar = { key: string; label: string; score: number; relativeStrength: number; contributingNotes: string[] };
export type ParfumoNote = { name: string; imageUrl: string; parfumoUrl: string; categories?: string[] };
export type CsvImportReport = { imported: number; duplicates: string[] };
export type ParfumoStatus = { connected: boolean; cookieCount: number };
export type WebdavBackup = { url: string; username: string; needsSync: boolean };
export type WebdavSyncReport = { fragrancesAdded: number; fragrancesUpdated: number; notesAdded: number; notesUpdated: number };
export type BackupStatus = { state: 'idle' | 'queued' | 'running' | 'error'; error: string | null };

export const backend = {
  listFragrances: () => invoke<Fragrance[]>('list_fragrances'),
  getFragranceAccords: (fragranceId: number) => invoke<AccordBar[]>('get_fragrance_accords', { fragranceId }),
  saveFragrance: (fragrance: Fragrance) => invoke<number>('save_fragrance', { fragrance }),
  updateFragrance: (fragrance: Fragrance) => invoke<void>('update_fragrance', { fragrance }),
  deleteFragrance: (id: number) => invoke<void>('delete_fragrance', { id }),
  listNoteImages: () => invoke<NoteImage[]>('list_note_images'),
  createNote: (name: string) => invoke<void>('create_note', { name }),
  searchParfumoNotes: (query: string) => invoke<ParfumoNote[]>('search_parfumo_notes', { query }),
  saveParfumoNote: (note: ParfumoNote) => invoke<void>('save_parfumo_note', { note }),
  refreshParfumoNoteMetadata: () => invoke<number>('refresh_parfumo_note_metadata'),
  renameNote: (oldName: string, newName: string) => invoke<void>('rename_note', { oldName, newName }),
  deleteNote: (name: string) => invoke<void>('delete_note', { name }),
  setNoteImage: (name: string, imageUrl: string) => invoke<void>('set_note_image', { name, imageUrl }),
  removeNoteImage: (name: string) => invoke<void>('remove_note_image', { name }),
  setNoteCategories: (name: string, categories: string[]) => invoke<void>('set_note_categories', { name, categories }),
  getBackupFolder: () => invoke<string | null>('get_backup_folder'),
  setBackupFolder: (folder: string) => invoke<void>('set_backup_folder', { folder }),
  getWebdavBackup: () => invoke<WebdavBackup | null>('get_webdav_backup'),
  setWebdavBackup: (url: string, username: string, password: string) => invoke<WebdavBackup>('set_webdav_backup', { url, username, password }),
  disconnectWebdavBackup: () => invoke<void>('disconnect_webdav_backup'),
  syncFromWebdav: () => invoke<WebdavSyncReport>('sync_from_webdav'),
  getBackupStatus: () => invoke<BackupStatus>('backup_status'),
  retryBackup: () => invoke<void>('retry_backup'),
  exportFragrancesCsv: (path: string) => invoke<void>('export_fragrances_csv', { path }),
  importFragrancesCsv: (content: string) => invoke<CsvImportReport>('import_fragrances_csv', { content }),
  exportNotesCsv: (path: string) => invoke<void>('export_notes_csv', { path }),
  importNotesCsv: (content: string) => invoke<CsvImportReport>('import_notes_csv', { content }),
  parfumoStatus: () => invoke<ParfumoStatus>('parfumo_status'),
  openParfumoLogin: () => invoke<void>('open_parfumo_login'),
  saveParfumoSession: () => invoke<ParfumoStatus>('save_parfumo_session'),
  disconnectParfumo: () => invoke<void>('disconnect_parfumo'),
  resetParfumoBrowser: () => invoke<void>('reset_parfumo_browser')
};
