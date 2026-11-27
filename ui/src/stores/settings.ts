//! Настройки и выбор инструмент → строй → струны (D-5, D-7, 6.3.5).
//! Переживают перезапуск (5.2–5.3): `loadSettings` наполняет стор из файла,
//! каждое изменение уходит в движок сразу, а в файл — debounce 300 мс.
//! UI-настройки (тема, язык) тоже сохраняются: файл — источник правды.

import { derived, get, writable } from 'svelte/store';
import { clampReference } from '../lib/format';
import type { Instrument, Sensitivity, Settings, Tuning } from '../lib/types';
import {
  selectInstrument as ipcSelectInstrument,
  selectTuning as ipcSelectTuning,
  selectString as ipcSelectString,
  setInputDevice as ipcSetInputDevice,
  setLowRange as ipcSetLowRange,
  setReference as ipcSetReference,
  setSensitivity as ipcSetSensitivity,
  setTolerance as ipcSetTolerance,
  getSettings as ipcGetSettings,
  saveSettings as ipcSaveSettings,
} from '../ipc/commands';
import { catalog, tuningsByInstrument, tuningsFor } from './catalog';
import { customs } from './customs';

import { manualStrings } from './tuner';
export { getPlatform } from './tuner';

export const DEFAULT_SETTINGS: Settings = {
  referenceHz: 440,
  instrumentId: 'guitar',
  tuningId: 'standard_e',
  notation: 'latin',
  theme: 'system',
  language: 'ru',
  bigType: false,
  strobe: false,
  toleranceCents: 3,
  sensitivity: 'normal',
  lowRange: false,
  haptics: true,
  sound: false,
  inputDevice: null,
  onboardingDone: false,
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

// ---- сохранение (5.2): debounce 300 мс, кроме первого заполнения из файла

let hydrated = false;
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleSave(): void {
  if (!hydrated) return; // первичное наполнение из файла назад не пишем
  if (saveTimer !== null) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void ipcSaveSettings(get(settings)).catch(() => {
      // Хранение — не критичный путь: ошибка только в консоль.
    });
  }, 300);
}

/** Загрузка сохранённых настроек (старт приложения, после loadCatalog). */
export async function loadSettings(): Promise<void> {
  try {
    const stored = await ipcGetSettings();
    settings.set({
      ...DEFAULT_SETTINGS,
      ...stored,
      inputDevice: stored.inputDevice ?? null,
    });
    lastTuning.set(stored.instrumentId, stored.tuningId);
  } catch {
    // Нет IPC или хранилища — остаёмся на дефолтах.
  } finally {
    hydrated = true;
  }
}

/** Отправить настройки в движок (после loadPresets, чтобы работал
 *  пользовательский строй): строй/инструмент, A4, допуск, чувствительность,
 *  «Низкий диапазон», сброс ручной струны. */
export async function applySettingsToEngine(): Promise<void> {
  const s = get(settings);
  if (s.instrumentId === 'chromatic') {
    await ipcSelectInstrument('chromatic');
  } else {
    const list = tuningsFor(s.instrumentId);
    const tuning = list.find((t) => t.id === s.tuningId) ?? list.find((t) => t.built_in);
    if (tuning) {
      // Строй из файла не нашёлся (пресет удалён) — берём первый встроенный
      // и чиним стор, чтобы UI показывал то же, что движок.
      if (tuning.id !== s.tuningId) {
        settings.update((x) => ({ ...x, tuningId: tuning.id }));
        lastTuning.set(s.instrumentId, tuning.id);
      }
      await ipcSelectTuning(tuning);
    }
  }
  manualStrings.set([]);
  await Promise.all([
    ipcSetReference(s.referenceHz),
    ipcSetTolerance(s.toleranceCents),
    ipcSetSensitivity(s.sensitivity),
    ipcSetLowRange(s.lowRange),
    ipcSetInputDevice(s.inputDevice),
    ipcSelectString(null),
  ]);
}

export function selectInstrument(id: string): void {
  const list = tuningsFor(id);
  const remembered = lastTuning.get(id);
  const tuningId =
    remembered !== undefined && list.some((t) => t.id === remembered)
      ? remembered
      : (list[0]?.id ?? null);
  lastTuning.set(id, tuningId);
  settings.update((s) => ({ ...s, instrumentId: id, tuningId }));
  manualStrings.set([]);
  scheduleSave();
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
  manualStrings.set([]);
  scheduleSave();
  const tuning = tuningsFor(instrumentId).find((t) => t.id === tuningId);
  if (tuning) void ipcSelectTuning(tuning);
}

export function setReference(hz: number): void {
  const clamped = clampReference(hz);
  settings.update((s) => ({ ...s, referenceHz: clamped }));
  scheduleSave();
  void ipcSetReference(clamped);
}

/** Универсальный сеттер для SettingsSheet: синхронизирует движок там,
 * где настройка влияет на DSP; всё сохраняется в файл. */
export function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]): void {
  settings.update((s) => ({ ...s, [key]: value }));
  scheduleSave();
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
    case 'lowRange':
      void ipcSetLowRange(value as boolean);
      break;
    case 'inputDevice':
      void ipcSetInputDevice(value as string | null);
      break;
  }
}

export function resetSettings(): void {
  lastTuning.clear();
  settings.set({ ...DEFAULT_SETTINGS });
  scheduleSave();
  // Движок — к дефолту: Standard E, A4 440, допуск 3, автоопределение струны.
  const std = tuningsFor(DEFAULT_SETTINGS.instrumentId).find(
    (t) => t.id === DEFAULT_SETTINGS.tuningId,
  );
  if (std) void ipcSelectTuning(std);
  void ipcSetReference(DEFAULT_SETTINGS.referenceHz);
  void ipcSetTolerance(DEFAULT_SETTINGS.toleranceCents);
  void ipcSetSensitivity(DEFAULT_SETTINGS.sensitivity);
  void ipcSetLowRange(DEFAULT_SETTINGS.lowRange);
  void ipcSetInputDevice(null);
  manualStrings.set([]);
  void ipcSelectString(null);
}

export function customPresetsFor(instrumentId: string): Tuning[] {
  return tuningsFor(instrumentId).filter((t) => !t.built_in);
}
