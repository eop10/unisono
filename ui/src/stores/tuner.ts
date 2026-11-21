//! Мок состояния тюнера (этап 3: аудио нет). Демо-панель переключает состояния.
//! Целевые частоты считает мок-бэкенд от опорной A4 — в UI нет DSP/аудио-логики.

import { get, writable } from 'svelte/store';
import type { TunerKind, TunerState } from '../lib/types';
import { settings } from './settings';

function hz(midi: number, a4: number): number {
  return a4 * 2 ** ((midi - 69) / 12);
}

const initial: TunerState = {
  kind: 'tracking',
  cents: -7,
  clarity: 0.97,
  db: -24,
  noteMidi: 40,
  targetHz: hz(40, 440),
  f0: hz(40, 440) * 2 ** (-7 / 1200),
  stringIndices: [0],
};

export const tuner = writable<TunerState>({ ...initial });

function retarget(midi: number | null): void {
  const a4 = get(settings).referenceHz;
  tuner.update((t) => {
    const targetHz = midi === null ? null : hz(midi, a4);
    const f0 = targetHz === null || t.cents === null ? null : targetHz * 2 ** (t.cents / 1200);
    return { ...t, noteMidi: midi, targetHz, f0 };
  });
}

// Смена опорной частоты пересчитывает цели без скачка индикатора (4.3.5).
settings.subscribe((s) => {
  const t = get(tuner);
  if (t.noteMidi !== null) {
    const targetHz = hz(t.noteMidi, s.referenceHz);
    tuner.update((x) => ({
      ...x,
      targetHz,
      f0: x.cents === null ? null : targetHz * 2 ** (x.cents / 1200),
    }));
  }
});

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
    return {
      ...t,
      cents,
      kind,
      f0: t.targetHz === null ? null : t.targetHz * 2 ** (cents / 1200),
    };
  });
}

/** Тап по чипу струны: унисон-пара выбирается целиком (ФТ-15). */
export function selectString(indices: number[], midi: number): void {
  tuner.update((t) => ({ ...t, stringIndices: indices }));
  retarget(midi);
}

export function demoReset(): void {
  tuner.set({ ...initial, targetHz: hz(40, get(settings).referenceHz) });
  retarget(40);
}