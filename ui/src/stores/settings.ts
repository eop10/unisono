//! Настройки и выбор инструмент → строй → струны (D-5, D-7, 6.3.5).
//! Изменения, влияющие на DSP (строй, A4, допуск, чувствительность, струна),
//! уходят в движок через IPC; UI-настройки (тема, язык) остаются локальными.

import { derived, writable } from 'svelte/store';
import { clampReference } from '../lib/format';
import type { Instrument, Sensitivity, Settings, Tuning } from '../lib/types';
import {
  selectInstrument as ipcSelectInstrument,
  selectTuning as ipcSelectTuning,
  selectString as ipcSelectString,
  setReference as ipcSetReference,
  setSensitivity as ipcSetSensitivity,
  setTolerance as ipcSetTolerance,
} from '../ipc/commands';
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
  // Движок: хроматика убирает строй, иначе — строй целиком (и встроенный,
  // и пользовательский: объект есть в каталоге UI, D-7).
  if (tuningId === null) {
    void ipcSelectInstrument(id);
  } else {
    const tuning = list.find((t) => t.id === tuningId);
    if (tuning) void ipcSelectTuning(tuning);
  }
}

export function selectTuning(instrumentId: string, tuningId: string): void {
  lastTuning.set(instrumentId, tuningId);
  settings.update((s) => ({ ...s, instrumentId, tuningId }));
  const tuning = tuningsFor(instrumentId).find((t) => t.id === tuningId);
  if (tuning) void ipcSelectTuning(tuning);
}

export function setReference(hz: number): void {
  const clamped = clampReference(hz);
  settings.update((s) => ({ ...s, referenceHz: clamped }));
  void ipcSetReference(clamped);
}

/** Универсальный сеттер для SettingsSheet: синхронизирует движок там,
 * где настройка влияет на DSP. */
export function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]): void {
  settings.update((s) => ({ ...s, [key]: value }));
  switch (key) {
    case 'referenceHz':
      void ipcSetReference(value as number);
      break;
    case 'toleranceCents':
      void ipcSetTolerance(value as number);
      break;
    case 'sensitivity':
      void ipcSetSensitivity(value as Sensitivity);
      break;
  }
}

export function resetSettings(): void {
  lastTuning.clear();
  settings.set({ ...DEFAULT_SETTINGS });
  // Движок — к дефолту: Standard E, A4 440, допуск 3, автоопределение струны.
  const std = tuningsFor(DEFAULT_SETTINGS.instrumentId).find(
    (t) => t.id === DEFAULT_SETTINGS.tuningId,
  );
  if (std) void ipcSelectTuning(std);
  void ipcSetReference(DEFAULT_SETTINGS.referenceHz);
  void ipcSetTolerance(DEFAULT_SETTINGS.toleranceCents);
  void ipcSetSensitivity(DEFAULT_SETTINGS.sensitivity);
  void ipcSelectString(null);
}

export function customPresetsFor(instrumentId: string): Tuning[] {
  return tuningsFor(instrumentId).filter((t) => !t.built_in);
}
