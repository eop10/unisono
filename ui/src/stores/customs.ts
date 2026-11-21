//! Пользовательские строи этапа 3: только в памяти (storage — этап 4).

import { writable } from 'svelte/store';
import type { Tuning } from '../lib/types';

export const customs = writable<Tuning[]>([]);

let seq = 0;

export function newPresetId(): string {
  seq += 1;
  return `custom.${Date.now().toString(36)}-${seq}`;
}

export function savePreset(t: Tuning): void {
  customs.update((list) => {
    const i = list.findIndex((x) => x.id === t.id);
    if (i >= 0) {
      const next = [...list];
      next[i] = t;
      return next;
    }
    return [...list, t];
  });
}