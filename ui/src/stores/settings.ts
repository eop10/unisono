//! Настройки и выбор инструмент → строй → струны (D-5, D-7, 6.3.5).

import { derived, get, writable } from 'svelte/store';
import { clampReference } from '../lib/format';
import type { Instrument, Settings, Tuning } from '../lib/types';
import { catalog, tuningsByInstrument, tuningsFor } from './catalog';
import { customs } from './customs';

export const DEFAULT_SETTINGS: Settings = {
  referenceHz: 440,
  instrumentId: 'guitar',
  tuningId: 'standard_e',
  notation: 'latin',
  theme: 'system',
  language: 'ru',
  bigType: false,
  toleranceCents: 3,
  sensitivity: 'normal',
  lowRange: false,
  haptics: true,
  sound: false,
};

export const settings = writable<Settings>({ ...DEFAULT_SETTINGS });

/** Последний выбранный строй на инструмент: возврат восстанавливает его (6.3.5). */
const lastTuning = new Map<string, string | null>();

export const currentInstrument = derived(
  [settings, catalog],
  ([s, c]): Instrument | null => c?.instruments.find((i) => i.id === s.instrumentId) ?? null,
);

export const currentTuning = derived(
  [settings, tuningsByInstrument],
  ([s, map]): Tuning | null =>
    (map.get(s.instrumentId) ?? []).find((t) => t.id === s.tuningId) ?? null,
);

/** Хроматический режим: струны скрыты полностью (ФТ-06, D-6). */
export const showStrings = derived(currentInstrument, (i) => i?.kind === 'Stringed');

export function selectInstrument(id: string): void {
  const list = tuningsFor(id);
  const remembered = lastTuning.get(id);
  const tuningId =
    remembered !== undefined && list.some((t) => t.id === remembered)
      ? remembered
      : (list[0]?.id ?? null);
  lastTuning.set(id, tuningId);
  settings.update((s) => ({ ...s, instrumentId: id, tuningId }));
}

export function selectTuning(instrumentId: string, tuningId: string): void {
  lastTuning.set(instrumentId, tuningId);
  settings.update((s) => ({ ...s, instrumentId, tuningId }));
}

export function setReference(hz: number): void {
  settings.update((s) => ({ ...s, referenceHz: clampReference(hz) }));
}

export function resetSettings(): void {
  lastTuning.clear();
  settings.set({ ...DEFAULT_SETTINGS });
}

export function customPresetsFor(instrumentId: string): Tuning[] {
  return get(customs).filter((t) => t.instrument_id === instrumentId);
}