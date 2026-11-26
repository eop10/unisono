//! Состояние тюнера: поток из Rust через Channel (этап 4A, 3.6).
//! Аудио-логики в UI нет: захват, DSP и выбор струны — в Rust. Стор хранит
//! последнее состояние и превращает аудио-ошибки в `error`/`nopermission`.

import { get, writable } from 'svelte/store';
import { Channel } from '@tauri-apps/api/core';
import type { TunerKind, TunerState } from '../lib/types';
import {
  getPlatform as ipcGetPlatform,
  hasMicPermission,
  keepScreenOn,
  requestMic,
  selectString as ipcSelectString,
  startListening as ipcStart,
  stopListening as ipcStop,
  vibrate,
  type Platform,
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
/** Ручной выбор струны (индексы чипов или пусто = авто); для мгновенной подсветки в тишине. */
export const manualStrings = writable<number[]>([]);

let prevKind: TunerKind = 'silent';
let lastVibrateAt = 0;
let platformCache: Platform | null = null;
let starting = false;

export async function getPlatform(): Promise<Platform> {
  if (platformCache === null) {
    platformCache = await ipcGetPlatform();
  }
  return platformCache;
}

export function resetPlatformCache(): void {
  platformCache = null;
}

export function resetVibrateThrottle(): void {
  lastVibrateAt = 0;
}

/** Аудио-ошибка → экран (4.4): отказ в доступе или сбой устройства.
 *  Ошибка приходит тегированным объектом `{"kind":"permissionDenied"}`
 *  (serde: tag+content); строку тоже принимаем — на случай старого формата. */
function isPermissionDenied(err: unknown): boolean {
  if (typeof err === 'string') {
    const s = err.toLowerCase();
    return s.includes('permission') || s.includes('denied');
  }
  const kind = (err as { kind?: unknown } | null | undefined)?.kind;
  if (typeof kind === 'string') {
    const s = kind.toLowerCase();
    return s.includes('permission') || s.includes('denied');
  }
  return false;
}

function audioErrorState(err: unknown): TunerState {
  const kind: TunerKind = isPermissionDenied(err) ? 'nopermission' : 'error';
  return { ...SILENT, kind };
}

/** Уточнение ошибки доступа: только Android (rev 5.2). Если фактически разрешение есть, значит сбой устройства. */
function refinePermissionError(): void {
  if (platformCache !== 'android') {
    return;
  }
  void hasMicPermission()
    .then((hasPerm) => {
      if (hasPerm && get(tuner).kind === 'nopermission') {
        tuner.set({ ...SILENT, kind: 'error' });
      }
    })
    .catch(() => {});
}

export async function startListening(): Promise<void> {
  if (starting) return;
  starting = true;
  try {
    try {
      if (platformCache === null) {
        platformCache = await getPlatform();
      }
      if (platformCache === 'android') {
        await requestMic();
      }
    } catch (e) {
      listening.set(false);
      const errState = audioErrorState(e);
      tuner.set(errState);
      if (errState.kind === 'nopermission') {
        refinePermissionError();
      }
      void keepScreenOn(false);
      return;
    }

    const channel = new Channel<TunerState>();
    prevKind = 'silent';
    channel.onmessage = (state) => {
      // Watchdog сдался (7.2.1): захват мёртв — кнопка возвращается в «Слушать».
      if (state.kind === 'error' || state.kind === 'nopermission') {
        listening.set(false);
        void keepScreenOn(false);
      } else if (state.kind === 'intune' && prevKind !== 'intune') {
        if (get(settings).haptics) {
          const now = Date.now();
          if (now - lastVibrateAt >= 1000) {
            lastVibrateAt = now;
            void vibrate();
          }
        }
      }
      prevKind = state.kind;
      tuner.set(state);
      if (state.kind === 'nopermission') {
        refinePermissionError();
      }
    };
    // Сразу в «Слушаю…»: после «Повторить» не мигают старые цифры (ревью 4A).
    tuner.set({ ...SILENT });
    try {
      await ipcStart(channel);
      listening.set(true);
      void keepScreenOn(true);
    } catch (e) {
      listening.set(false);
      const errState = audioErrorState(e);
      tuner.set(errState);
      if (errState.kind === 'nopermission') {
        refinePermissionError();
      }
      void keepScreenOn(false);
    }
  } finally {
    starting = false;
  }
}

export async function stopListening(): Promise<void> {
  prevKind = 'silent';
  lastVibrateAt = 0;
  try {
    await ipcStop();
  } catch {
    // Команда не прошла (IPC/бэкенд) — UI-флоу не роняем, состояние сбрасываем.
  } finally {
    listening.set(false);
    tuner.set({ ...SILENT });
    void keepScreenOn(false);
  }
}

export async function toggleListening(): Promise<void> {
  if (get(listening)) await stopListening();
  else await startListening();
}

/** Тап по чипу струны: унисон-пара выбирается целиком (ФТ-15), тап по
 * активной струне возвращает автоопределение. Пара живёт в движке. */
export function selectString(indices: number[], _midi: number): void {
  const manual = get(manualStrings);
  const current = get(tuner).stringIndices;
  const active =
    indices.length > 0 &&
    (indices.every((i) => manual.includes(i)) || indices.every((i) => current.includes(i)));
  manualStrings.set(active ? [] : indices);
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
