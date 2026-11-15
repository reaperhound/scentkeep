import { invoke } from '@tauri-apps/api/core';

export type Fragrance = {
  id?: number; collection: string; name: string; brand: string; year: number;
  audience: string; description: string; topNotes: string[]; heartNotes: string[];
  baseNotes: string[]; perfumer: string; bottleRating: number | null;
  ratingCount: number | null; imageUrl: string;
};

export type NoteImage = { name: string; imageUrl: string; usageCount: number };

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
  removeNoteImage: (name: string) => invoke<void>('remove_note_image', { name })
};
