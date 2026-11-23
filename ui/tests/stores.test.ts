// @vitest-environment jsdom
//! Сторы с реальным IPC-моком: выбор инструмента/строя шлётся в движок,
//! поток состояний приходит через Channel, ошибки ауда — в error/nopermission.

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { Channel } from '@tauri-apps/api/core';
import { mockCatalog } from './mockCatalog';
import type { TunerState } from '../src/lib/types';

vi.mock('../src/ipc/commands', () => ({
  getCatalog: vi.fn(async () => mockCatalog),
  startListening: vi.fn(async (_channel: Channel<TunerState>) => {}),
  stopListening: vi.fn(async () => {}),
  setReference: vi.fn(async (_hz: number) => {}),
  selectInstrument: vi.fn(async (_id: string) => {}),
  selectTuning: vi.fn(async (_tuning: unknown) => {}),
  selectString: vi.fn(async (_index: number | null) => {}),
  setTolerance: vi.fn(async (_cents: number) => {}),
  setSensitivity: vi.fn(async (_s: string) => {}),
}));

import * as ipc from '../src/ipc/commands';
import { loadCatalog, tuningsFor } from '../src/stores/catalog';
import {
  currentTuning,
  selectInstrument,
  selectTuning,
  setReference,
  settings,
  showStrings,
  DEFAULT_SETTINGS,
} from '../src/stores/settings';
import {
  listening,
  selectString,
  startListening,
  stopListening,
  tuner,
} from '../src/stores/tuner';
import { customs } from '../src/stores/customs';

beforeEach(async () => {
  vi.clearAllMocks();
  settings.set({ ...DEFAULT_SETTINGS });
  customs.set([]);
  await loadCatalog();
});

describe('instrument -> tuning -> strings', () => {
  it('selects first tuning of instrument', () => {
    selectInstrument('violin');
    expect(get(settings).tuningId).toBe('violin_std');
    expect(get(currentTuning)?.strings).toEqual([55, 62, 69, 76]);
  });

  it('chromatic hides strings and switches the engine without a tuning', () => {
    selectInstrument('chromatic');
    expect(get(settings).tuningId).toBeNull();
    expect(get(showStrings)).toBe(false);
    expect(tuningsFor('chromatic')).toEqual([]);
    expect(ipc.selectInstrument).toHaveBeenCalledWith('chromatic');
    expect(ipc.selectTuning).not.toHaveBeenCalled();
    selectInstrument('guitar');
    expect(get(showStrings)).toBe(true);
  });

  it('instrument selection sends the tuning object to the engine', () => {
    selectInstrument('violin');
    expect(ipc.selectTuning).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'violin_std', instrument_id: 'violin' }),
    );
    // Повторный выбор того же инструмента шлёт выбранный строй, не дефолт.
    selectTuning('violin', 'violin_std');
    expect(ipc.selectTuning).toHaveBeenCalledTimes(2);
  });

  it('restores last tuning when returning to instrument', () => {
    selectTuning('guitar', 'drop_d');
    selectInstrument('violin');
    selectInstrument('guitar');
    expect(get(settings).tuningId).toBe('drop_d');
    expect(ipc.selectTuning).toHaveBeenLastCalledWith(
      expect.objectContaining({ id: 'drop_d' }),
    );
  });

  it('reference is clamped and sent to the engine', () => {
    setReference(999);
    expect(get(settings).referenceHz).toBe(480);
    expect(ipc.setReference).toHaveBeenCalledWith(480);
    setReference(415);
    expect(ipc.setReference).toHaveBeenLastCalledWith(415);
  });

  it('custom presets are scoped to their instrument', () => {
    customs.set([
      {
        id: 'custom.x',
        instrument_id: 'violin',
        name: { Custom: 'Мой' },
        strings: [55, 62, 69],
        built_in: false,
      },
    ]);
    expect(tuningsFor('violin').map((t) => t.id)).toContain('custom.x');
    expect(tuningsFor('guitar').map((t) => t.id)).not.toContain('custom.x');
  });
});

describe('audio stream store (4A)', () => {
  it('start listening opens a channel and raises the flag', async () => {
    await startListening();
    expect(ipc.startListening).toHaveBeenCalledOnce();
    expect(get(listening)).toBe(true);
  });

  it('channel messages flow into the store', async () => {
    await startListening();
    const channel = vi
      .mocked(ipc.startListening)
      .mock.calls.at(-1)?.[0] as Channel<TunerState>;
    channel.onmessage?.({
      kind: 'intune',
      f0: 82.4,
      cents: 0,
      clarity: 0.95,
      db: -20,
      noteMidi: 40,
      targetHz: 82.4,
      stringIndices: [0],
    });
    expect(get(tuner).kind).toBe('intune');
    expect(get(tuner).noteMidi).toBe(40);
  });

  it('tagged permission error maps to nopermission, other audio errors to error', async () => {
    // Формат serde: {"kind":"permissionDenied"} (tag+content, camelCase).
    vi.mocked(ipc.startListening).mockRejectedValueOnce({ kind: 'permissionDenied' });
    await startListening();
    expect(get(tuner).kind).toBe('nopermission');
    expect(get(listening)).toBe(false);

    vi.mocked(ipc.startListening).mockRejectedValueOnce({ kind: 'device', message: 'busy' });
    await startListening();
    expect(get(tuner).kind).toBe('error');
    expect(get(listening)).toBe(false);

    // Старый строковый формат тоже принимается.
    vi.mocked(ipc.startListening).mockRejectedValueOnce('permissionDenied');
    await startListening();
    expect(get(tuner).kind).toBe('nopermission');
  });

  it('start resets the display to silent so retry does not flash old digits', async () => {
    tuner.set({
      kind: 'tracking',
      f0: 82.4,
      cents: -7,
      clarity: 0.9,
      db: -20,
      noteMidi: 40,
      targetHz: 82.4,
      stringIndices: [0],
    });
    vi.mocked(ipc.startListening).mockRejectedValueOnce({ kind: 'device', message: 'busy' });
    await startListening();
    expect(get(tuner).kind).toBe('error');
    // До invoke стор уже был в silent: старые центы не мигают после «Повторить».
    tuner.set({
      kind: 'tracking',
      f0: 82.4,
      cents: -7,
      clarity: 0.9,
      db: -20,
      noteMidi: 40,
      targetHz: 82.4,
      stringIndices: [0],
    });
    await startListening();
    expect(ipc.startListening).toHaveBeenCalledTimes(2);
  });

  it('stop listening resets the state even if the command fails', async () => {
    await startListening();
    vi.mocked(ipc.stopListening).mockRejectedValueOnce(new Error('ipc down'));
    await stopListening();
    expect(ipc.stopListening).toHaveBeenCalledOnce();
    expect(get(listening)).toBe(false);
    expect(get(tuner).kind).toBe('silent');
    expect(get(tuner).f0).toBeNull();
  });

  it('string tap sends the manual index; tapping the active pair returns to auto', async () => {
    await startListening();
    const channel = vi
      .mocked(ipc.startListening)
      .mock.calls.at(-1)?.[0] as Channel<TunerState>;
    channel.onmessage?.({
      kind: 'tracking',
      f0: 330,
      cents: 0,
      clarity: 0.9,
      db: -20,
      noteMidi: 64,
      targetHz: 329.6,
      stringIndices: [0, 1],
    });
    // Пара подсвечена движком; тап по ней возвращает автоопределение.
    selectString([0, 1], 64);
    expect(ipc.selectString).toHaveBeenLastCalledWith(null);
    // Чужая струна — ручной выбор её индекса.
    selectString([2], 69);
    expect(ipc.selectString).toHaveBeenLastCalledWith(2);
  });
});
