//! Пользовательские строи (5.6): источник — tuner-storage через IPC,
//! стор — единственное представление для каталога UI (name → `Custom`).

import { writable } from 'svelte/store';
import type { Tuning } from '../lib/types';
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
