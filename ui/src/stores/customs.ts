//! Пользовательские строи (5.6): источник — tuner-storage через IPC,
//! стор — единственное представление для каталога UI (name → `Custom`).

import { get, writable } from 'svelte/store';
import type { Tuning } from '../lib/types';
import { NAME_MAX, validateTuning } from '../lib/validate';
import {
  deletePreset as ipcDelete,
  getPresets as ipcGetPresets,
  savePreset as ipcSave,
  type PresetDto,
} from '../ipc/commands';

export const customs = writable<Tuning[]>([]);

function dtoToTuning(p: PresetDto): Tuning {
  return {
    id: p.id,
    instrument_id: p.instrumentId,
    name: { Custom: p.name },
    strings: p.strings,
    built_in: false,
  };
}

function tuningToDto(t: Tuning): PresetDto {
  return {
    id: t.id,
    instrumentId: t.instrument_id,
    name: 'Custom' in t.name ? t.name.Custom : '',
    strings: t.strings,
  };
}

/** Старт: пресеты из файла (после loadCatalog, до applySettingsToEngine). */
export async function loadPresets(): Promise<void> {
  try {
    const presets = await ipcGetPresets();
    customs.set(presets.map(dtoToTuning));
  } catch {
    // Нет IPC или хранилища — пусто; приложение продолжает работать.
  }
}

let seq = 0;

export function newPresetId(): string {
  seq += 1;
  return `custom.${Date.now().toString(36)}-${seq}`;
}

export function savePreset(tuning: Tuning): void {
  customs.update((list) => {
    const i = list.findIndex((x) => x.id === tuning.id);
    if (i >= 0) {
      const next = [...list];
      next[i] = tuning;
      return next;
    }
    return [...list, tuning];
  });
  void ipcSave(tuningToDto(tuning)).catch(() => {});
}

/** Удалить пользовательский строй; встроенные сюда не попадают (5.7). */
export function deletePreset(id: string): void {
  customs.update((list) => list.filter((t) => t.id !== id));
  void ipcDelete(id).catch(() => {});
}

export interface ExportedPreset {
  instrumentId: string;
  name: string;
  strings: number[];
}

export interface ExportedPresetsBundle {
  format: 'unisono-presets';
  version: 1;
  presets: ExportedPreset[];
}

/** Сериализация пользовательских строёв в переносимый JSON (1.1). */
export function exportPresetsJson(): { json: string; count: number } {
  const list = get(customs);
  const bundle: ExportedPresetsBundle = {
    format: 'unisono-presets',
    version: 1,
    presets: list.map((t) => ({
      instrumentId: t.instrument_id,
      name: 'Custom' in t.name ? t.name.Custom : '',
      strings: [...t.strings],
    })),
  };
  return { json: JSON.stringify(bundle, null, 2), count: list.length };
}

/** Импорт пользовательских строёв из JSON: валидирует, пропускает дубли и лимит. */
export function importPresetsJson(raw: string, knownInstrumentIds: Set<string>): number | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  const items = Array.isArray(parsed)
    ? parsed
    : typeof parsed === 'object' &&
        parsed !== null &&
        Array.isArray((parsed as { presets?: unknown }).presets)
      ? (parsed as { presets: unknown[] }).presets
      : null;
  if (!items) return null;

  let added = 0;
  for (const item of items) {
    if (typeof item !== 'object' || item === null) continue;
    const obj = item as { instrumentId?: unknown; name?: unknown; strings?: unknown };
    if (typeof obj.instrumentId !== 'string' || !knownInstrumentIds.has(obj.instrumentId)) continue;
    if (typeof obj.name !== 'string' || !Array.isArray(obj.strings)) continue;
    const strings = obj.strings.filter(
      (n): n is number => typeof n === 'number' && Number.isInteger(n),
    );
    if (strings.length !== obj.strings.length) continue;

    const trimmedName = obj.name.trim().slice(0, NAME_MAX);
    const existingForInst = get(customs)
      .filter((t) => t.instrument_id === obj.instrumentId)
      .map((t) => ({ id: t.id, name: 'Custom' in t.name ? t.name.Custom : '' }));
    if (validateTuning(trimmedName, strings, existingForInst, null) !== null) continue;

    savePreset({
      id: newPresetId(),
      instrument_id: obj.instrumentId,
      name: { Custom: trimmedName },
      strings,
      built_in: false,
    });
    added += 1;
  }
  return added;
}
