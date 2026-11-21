//! Каталог из ядра: один источник правды, без дублирования строёв в TS.

import { derived, writable, get } from 'svelte/store';
import { getCatalog } from '../ipc/commands';
import type { Catalog, Tuning } from '../lib/types';
import { customs } from './customs';

export const catalog = writable<Catalog | null>(null);
export const catalogError = writable<boolean>(false);

export async function loadCatalog(): Promise<void> {
  try {
    catalog.set(await getCatalog());
    catalogError.set(false);
  } catch {
    catalogError.set(true);
  }
}

export const instruments = derived(catalog, (c) => c?.instruments ?? []);

/** Все строи инструмента: встроенные + пользовательские (привязка к инструменту). */
export const tuningsByInstrument = derived(
  [catalog, customs],
  ([c, cu]): Map<string, Tuning[]> => {
    const map = new Map<string, Tuning[]>();
    for (const t of [...(c?.tunings ?? []), ...cu]) {
      const list = map.get(t.instrument_id) ?? [];
      list.push(t);
      map.set(t.instrument_id, list);
    }
    return map;
  },
);

export function tuningsFor(instrumentId: string): Tuning[] {
  return get(tuningsByInstrument).get(instrumentId) ?? [];
}