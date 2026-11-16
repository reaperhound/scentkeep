import { invoke } from '@tauri-apps/api/core';

export type Fragrance = {
  id?: number; collection: string; name: string; brand: string; year: number;
  audience: string; concentration: string; description: string; topNotes: string[]; heartNotes: string[];
  baseNotes: string[]; perfumer: string; bottleRating: number | null;
  ratingCount: number | null; imageUrl: string;
  shelf: 'Collection' | 'Wishlist' | 'Empties'; bottlesOwned: number; bottlesFinished: number; favourite: boolean; priceBought: number | null; currentPrice: number | null;
};

export type NoteImage = { name: string; imageUrl: string; usageCount: number };
export type CsvImportReport = { imported: number; duplicates: string[] };
export type ParfumoStatus = { connected: boolean; cookieCount: number };

export const backend = {
  listFragrances: () => invoke<Fragrance[]>('list_fragrances'),
  saveFragrance: (fragrance: Fragrance) => invoke<number>('save_fragrance', { fragrance }),
  updateFragrance: (fragrance: Fragrance) => invoke<void>('update_fragrance', { fragrance }),
  deleteFragrance: (id: number) => invoke<void>('delete_fragrance', { id }),
  listNoteImages: () => invoke<NoteImage[]>('list_note_images'),
  createNote: (name: string) => invoke<void>('create_note', { name }),
  renameNote: (oldName: string, newName: string) => invoke<void>('rename_note', { oldName, newName }),
  deleteNote: (name: string) => invoke<void>('delete_note', { name }),
  setNoteImage: (name: string, imageUrl: string) => invoke<void>('set_note_image', { name, imageUrl }),
  removeNoteImage: (name: string) => invoke<void>('remove_note_image', { name }),
  getBackupFolder: () => invoke<string | null>('get_backup_folder'),
  setBackupFolder: (folder: string) => invoke<void>('set_backup_folder', { folder }),
  exportFragrancesCsv: (path: string) => invoke<void>('export_fragrances_csv', { path }),
  importFragrancesCsv: (content: string) => invoke<CsvImportReport>('import_fragrances_csv', { content }),
  exportNotesCsv: (path: string) => invoke<void>('export_notes_csv', { path }),
  importNotesCsv: (content: string) => invoke<CsvImportReport>('import_notes_csv', { content }),
  parfumoStatus: () => invoke<ParfumoStatus>('parfumo_status'),
  openParfumoLogin: () => invoke<void>('open_parfumo_login'),
  saveParfumoSession: () => invoke<ParfumoStatus>('save_parfumo_session'),
  disconnectParfumo: () => invoke<void>('disconnect_parfumo')
};
