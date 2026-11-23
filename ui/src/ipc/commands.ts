//! IPC-обёртки Tauri: каталог и аудио-команды (3.6). Типы — зеркало
//! serde-вывода ядра; аудио-логики в JS нет.

import { Channel, invoke } from '@tauri-apps/api/core';
import type { Catalog, Sensitivity, TunerState, Tuning } from '../lib/types';

export async function getCatalog(): Promise<Catalog> {
  return await invoke<Catalog>('get_catalog');
}

export async function startListening(channel: Channel<TunerState>): Promise<void> {
  await invoke('start_listening', { channel });
}

export async function stopListening(): Promise<void> {
  await invoke('stop_listening');
}

export async function setReference(hz: number): Promise<void> {
  await invoke('set_reference', { hz });
}

export async function selectInstrument(id: string): Promise<void> {
  await invoke('select_instrument', { id });
}

/** Строй целиком: и встроенный, и пользовательский (движку нужны струны). */
export async function selectTuning(tuning: Tuning): Promise<void> {
  await invoke('select_tuning', { tuning });
}

/** Ручная струна; null — вернуться к автоопределению (ФТ-15). */
export async function selectString(index: number | null): Promise<void> {
  await invoke('select_string', { index });
}

export async function setTolerance(cents: number): Promise<void> {
  await invoke('set_tolerance', { cents });
}

export async function setSensitivity(sensitivity: Sensitivity): Promise<void> {
  await invoke('set_sensitivity', { sensitivity });
}
