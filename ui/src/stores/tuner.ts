//! Состояние тюнера: поток из Rust через Channel (этап 4A, 3.6).
//! Аудио-логики в UI нет: захват, DSP и выбор струны — в Rust. Стор хранит
//! последнее состояние и превращает аудио-ошибки в `error`/`nopermission`.

import { get, writable } from 'svelte/store';
import { Channel } from '@tauri-apps/api/core';
import type { TunerKind, TunerState } from '../lib/types';
import {
  selectString as ipcSelectString,
  startListening as ipcStart,
  stopListening as ipcStop,
} from '../ipc/commands';
import { settings } from './settings';

const SILENT: TunerState = {
  kind: 'silent',
  f0: null,
  cents: null,
  clarity: 0,
  db: -60,
  noteMidi: null,
  targetHz: null,
  stringIndices: [],
};

export const tuner = writable<TunerState>({ ...SILENT });
export const listening = writable(false);

/** Аудио-ошибка → экран (4.4): отказ в доступе или сбой устройства. */
function audioErrorState(err: unknown): TunerState {
  const kind: TunerKind = err === 'PermissionDenied' ? 'nopermission' : 'error';
  return { ...SILENT, kind };
}

export async function startListening(): Promise<void> {
  const channel = new Channel<TunerState>();
  channel.onmessage = (state) => tuner.set(state);
  try {
    await ipcStart(channel);
    listening.set(true);
  } catch (e) {
    listening.set(false);
    tuner.set(audioErrorState(e));
  }
}

export async function stopListening(): Promise<void> {
  await ipcStop();
  listening.set(false);
  tuner.set({ ...SILENT });
}

export async function toggleListening(): Promise<void> {
  if (get(listening)) await stopListening();
  else await startListening();
}

/** Тап по чипу струны: унисон-пара выбирается целиком (ФТ-15), тап по
 * активной струне возвращает автоопределение. Пара живёт в движке. */
export function selectString(indices: number[], _midi: number): void {
  const current = get(tuner).stringIndices;
  const active = indices.length > 0 && indices.every((i) => current.includes(i));
  void ipcSelectString(active ? null : (indices[0] ?? null));
}

// ---- Демо-панель (DEV/?demo): локальные состояния поверх реального потока.
// Первое действие панели останавливает прослушивание (см. DemoPanel).

export function demoSetKind(kind: TunerKind): void {
  tuner.update((t) => ({
    ...t,
    kind,
    f0: kind === 'silent' || kind === 'error' || kind === 'nopermission' ? null : t.f0,
    cents: kind === 'silent' || kind === 'error' || kind === 'nopermission' ? null : t.cents,
  }));
}

export function demoSetCents(cents: number): void {
  tuner.update((t) => {
    const kind: TunerKind =
      t.kind === 'silent' || t.kind === 'error' || t.kind === 'nopermission'
        ? t.kind
        : Math.abs(cents) <= get(settings).toleranceCents
          ? 'intune'
          : 'tracking';
    const targetHz = t.targetHz ?? 440;
    return { ...t, cents, kind, f0: targetHz * 2 ** (cents / 1200), targetHz };
  });
}

export function demoReset(): void {
  tuner.set({
    kind: 'tracking',
    cents: -7,
    clarity: 0.97,
    db: -24,
    noteMidi: 40,
    targetHz: 440 * 2 ** ((40 - 69) / 12),
    f0: 440 * 2 ** ((40 - 69) / 12) * 2 ** (-7 / 1200),
    stringIndices: [0],
  });
}
