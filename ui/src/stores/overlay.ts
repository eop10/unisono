//! Оверлеи: листы и редактор (6.2).

import { writable } from 'svelte/store';
import type { EditorContext, Overlay } from '../lib/types';

export const overlay = writable<Overlay>(null);
export const editorContext = writable<EditorContext | null>(null);

export function openOverlay(o: Exclude<Overlay, null>): void {
  overlay.set(o);
}

export function closeOverlay(): void {
  overlay.set(null);
  editorContext.set(null);
}

export function openEditor(instrumentId: string, tuningId: string | null): void {
  editorContext.set({ instrumentId, tuningId });
  overlay.set('editor');
}