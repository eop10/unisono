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
  startListening: vi.fn(async (_channel: unknown) => {}),
  stopListening: vi.fn(async () => {}),
  setReference: vi.fn(async (_hz: number) => {}),
  selectInstrument: vi.fn(async (_id: string) => {}),
  selectTuning: vi.fn(async (_tuning: unknown) => {}),
  selectString: vi.fn(async (_index: number | null) => {}),
  setTolerance: vi.fn(async (_cents: number) => {}),
  setSensitivity: vi.fn(async (_s: string) => {}),
  setLowRange: vi.fn(async (_low: boolean) => {}),
  getSettings: vi.fn(async () => null),
  saveSettings: vi.fn(async (_s: unknown) => {}),
  getPresets: vi.fn(async () => [] as { id: string; instrumentId: string; name: string; strings: number[] }[]),
  savePreset: vi.fn(async (_p: unknown) => {}),
  deletePreset: vi.fn(async (_id: string) => {}),
  getInputDevices: vi.fn(async () => []),
  setInputDevice: vi.fn(async (_name: unknown) => {}),
  getPlatform: vi.fn(async () => 'desktop' as const),
  hasMicPermission: vi.fn(async () => false),
  requestMic: vi.fn(async () => {}),
  keepScreenOn: vi.fn(async (_enabled: boolean) => {}),
  vibrate: vi.fn(async () => {}),
}));

import * as ipc from '../src/ipc/commands';
import { loadCatalog, tuningsFor } from '../src/stores/catalog';
import {
  applySettingsToEngine,
  currentTuning,
  loadSettings,
  selectInstrument,
  selectTuning,
  setReference,
  settings,
  showStrings,
  DEFAULT_SETTINGS,
} from '../src/stores/settings';
import { loadPresets, deletePreset as removePreset } from '../src/stores/customs';
import {
  listening,
  resetPlatformCache,
  resetVibrateThrottle,
  selectString,
  startListening,
  stopListening,
  tuner,
} from '../src/stores/tuner';
import { customs } from '../src/stores/customs';

beforeEach(async () => {
  vi.clearAllMocks();
  resetPlatformCache();
  resetVibrateThrottle();
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

  it('watchdog give-up state drops the listening flag', async () => {
    await startListening();
    expect(get(listening)).toBe(true);
    const channel = vi
      .mocked(ipc.startListening)
      .mock.calls.at(-1)?.[0] as Channel<TunerState>;
    // 7.2.1: наблюдатель исчерпал попытки и отправил ошибку в канал.
    channel.onmessage?.({
      kind: 'error',
      f0: null,
      cents: null,
      clarity: 0,
      db: -180,
      noteMidi: null,
      targetHz: null,
      stringIndices: [],
    });
    expect(get(listening)).toBe(false);
    expect(get(tuner).kind).toBe('error');
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

describe('settings and presets survive restart (5.2–5.6)', () => {
  it('loadSettings fills the store from the file', async () => {
    vi.mocked(ipc.getSettings).mockResolvedValueOnce({
      ...DEFAULT_SETTINGS,
      referenceHz: 442,
      instrumentId: 'bass',
      tuningId: 'bass5',
      theme: 'light',
      language: 'en',
      toleranceCents: 5,
      sensitivity: 'high',
      lowRange: true,
      onboardingDone: true,
    });
    await loadSettings();
    const s = get(settings);
    expect(s.referenceHz).toBe(442);
    expect(s.instrumentId).toBe('bass');
    expect(s.tuningId).toBe('bass5');
    expect(s.theme).toBe('light');
    expect(s.toleranceCents).toBe(5);
    expect(s.sensitivity).toBe('high');
    expect(s.lowRange).toBe(true);
    // Первичное заполнение из файла не пишет назад.
    expect(ipc.saveSettings).not.toHaveBeenCalled();
  });

  it('changing A4/tolerance/lowRange reaches the engine and schedules save', async () => {
    vi.useFakeTimers();
    setReference(442);
    expect(ipc.setReference).toHaveBeenLastCalledWith(442);
    // Debounce: сохранение не раньше 300 мс.
    expect(ipc.saveSettings).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(350);
    expect(ipc.saveSettings).toHaveBeenCalledOnce();
    expect(ipc.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({ referenceHz: 442 }),
    );
    vi.useRealTimers();
  });

  it('presets come from IPC into tuningsFor; delete removes them', async () => {
    vi.mocked(ipc.getPresets).mockResolvedValueOnce([
      {
        id: 'custom.x',
        instrumentId: 'violin',
        name: 'Мой',
        strings: [55, 62, 69],
      },
    ]);
    await loadPresets();
    expect(get(customs)).toHaveLength(1);
    const list = tuningsFor('violin');
    expect(list.map((t) => t.id)).toContain('custom.x');
    const custom = list.find((t) => t.id === 'custom.x')!;
    expect(custom.name).toEqual({ Custom: 'Мой' });
    expect(custom.built_in).toBe(false);

    removePreset('custom.x');
    expect(ipc.deletePreset).toHaveBeenCalledWith('custom.x');
    expect(tuningsFor('violin').map((t) => t.id)).not.toContain('custom.x');
  });

  it('applySettingsToEngine sends tuning, calibration and precision', async () => {
    settings.set({
      ...DEFAULT_SETTINGS,
      instrumentId: 'guitar',
      tuningId: 'standard_e',
      referenceHz: 442,
      toleranceCents: 2,
      sensitivity: 'low',
      lowRange: true,
    });
    await applySettingsToEngine();
    expect(ipc.selectTuning).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'standard_e', instrument_id: 'guitar' }),
    );
    expect(ipc.setReference).toHaveBeenLastCalledWith(442);
    expect(ipc.setTolerance).toHaveBeenLastCalledWith(2);
    expect(ipc.setSensitivity).toHaveBeenLastCalledWith('low');
    expect(ipc.setLowRange).toHaveBeenLastCalledWith(true);
    expect(ipc.setInputDevice).toHaveBeenLastCalledWith(null);
    expect(ipc.selectString).toHaveBeenLastCalledWith(null);
  });

  it('applySettingsToEngine sends non-null inputDevice', async () => {
    settings.set({
      ...DEFAULT_SETTINGS,
      inputDevice: 'Studio Mic',
    });
    await applySettingsToEngine();
    expect(ipc.setInputDevice).toHaveBeenLastCalledWith('Studio Mic');
  });

  it('chromatic settings switch the engine without a tuning', async () => {
    settings.set({ ...DEFAULT_SETTINGS, instrumentId: 'chromatic', tuningId: null });
    await applySettingsToEngine();
    expect(ipc.selectInstrument).toHaveBeenCalledWith('chromatic');
    expect(ipc.selectTuning).not.toHaveBeenCalled();
  });

  it('missing custom tuning falls back to the first builtin', async () => {
    settings.set({
      ...DEFAULT_SETTINGS,
      instrumentId: 'violin',
      tuningId: 'custom.gone', // пресет не загружен
    });
    await applySettingsToEngine();
    expect(ipc.selectTuning).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'violin_std' }),
    );
    expect(get(settings).tuningId).toBe('violin_std');
  });

  describe('android platform, permissions, keepScreenOn & haptics', () => {
    it('requests mic on android before start_listening', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
      await startListening();
      expect(ipc.requestMic).toHaveBeenCalled();
      expect(ipc.startListening).toHaveBeenCalled();
      expect(ipc.keepScreenOn).toHaveBeenCalledWith(true);
      expect(get(listening)).toBe(true);
    });

    it('shows nopermission if mic request is denied on android', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
      vi.mocked(ipc.requestMic).mockRejectedValueOnce('permission denied');
      await startListening();
      expect(ipc.requestMic).toHaveBeenCalled();
      expect(ipc.startListening).not.toHaveBeenCalled();
      expect(get(listening)).toBe(false);
      expect(get(tuner).kind).toBe('nopermission');
      expect(ipc.keepScreenOn).toHaveBeenCalledWith(false);
    });

    it('refines nopermission to error if hasMicPermission resolves to true', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
      vi.mocked(ipc.requestMic).mockRejectedValueOnce('permission denied');
      vi.mocked(ipc.hasMicPermission).mockResolvedValueOnce(true);
      await startListening();
      await Promise.resolve();
      await Promise.resolve();
      expect(get(tuner).kind).toBe('error');
    });

    it('keeps nopermission if hasMicPermission resolves to false', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
      vi.mocked(ipc.requestMic).mockRejectedValueOnce('permission denied');
      vi.mocked(ipc.hasMicPermission).mockResolvedValueOnce(false);
      await startListening();
      await Promise.resolve();
      await Promise.resolve();
      expect(get(tuner).kind).toBe('nopermission');
    });

    it('refines channel nopermission to error if hasMicPermission is true', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
      let messageHandler: ((s: TunerState) => void) | undefined;
      vi.mocked(ipc.startListening).mockImplementationOnce(async (channel: unknown) => {
        messageHandler = (channel as Channel<TunerState>).onmessage;
      });
      vi.mocked(ipc.hasMicPermission).mockResolvedValueOnce(true);
      await startListening();
      messageHandler!({
        kind: 'nopermission',
        cents: null,
        clarity: 0,
        db: -60,
        f0: null,
        noteMidi: null,
        stringIndices: [],
        targetHz: null,
      });
      expect(get(tuner).kind).toBe('nopermission');
      await Promise.resolve();
      await Promise.resolve();
      expect(get(tuner).kind).toBe('error');
    });

    it('keeps nopermission on desktop when ipcStart fails with permissionDenied and does not call hasMicPermission', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('desktop');
      vi.mocked(ipc.startListening).mockRejectedValueOnce({ kind: 'permissionDenied' });
      await startListening();
      expect(get(listening)).toBe(false);
      expect(get(tuner).kind).toBe('nopermission');
      await Promise.resolve();
      await Promise.resolve();
      expect(get(tuner).kind).toBe('nopermission');
      expect(ipc.hasMicPermission).not.toHaveBeenCalled();
    });

    it('keeps nopermission on desktop when channel emits nopermission and does not call hasMicPermission', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('desktop');
      let messageHandler: ((s: TunerState) => void) | undefined;
      vi.mocked(ipc.startListening).mockImplementationOnce(async (channel: unknown) => {
        messageHandler = (channel as Channel<TunerState>).onmessage;
      });
      await startListening();
      messageHandler!({
        kind: 'nopermission',
        cents: null,
        clarity: 0,
        db: -60,
        f0: null,
        noteMidi: null,
        stringIndices: [],
        targetHz: null,
      });
      expect(get(tuner).kind).toBe('nopermission');
      await Promise.resolve();
      await Promise.resolve();
      expect(get(tuner).kind).toBe('nopermission');
      expect(ipc.hasMicPermission).not.toHaveBeenCalled();
    });

    it('caches platform after first startListening call', async () => {
      vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
      await startListening();
      expect(ipc.getPlatform).toHaveBeenCalledTimes(1);

      // Subsequent startListening call reuses cached platform
      await startListening();
      expect(ipc.getPlatform).toHaveBeenCalledTimes(1);
    });

    it('clears screen flag when stopped', async () => {
      await stopListening();
      expect(ipc.stopListening).toHaveBeenCalled();
      expect(ipc.keepScreenOn).toHaveBeenCalledWith(false);
      expect(get(listening)).toBe(false);
    });

    it('vibrates on transition into intune when haptics is enabled', async () => {
      let messageHandler: ((s: TunerState) => void) | undefined;
      vi.mocked(ipc.startListening).mockImplementationOnce(async (channel: unknown) => {
        messageHandler = (channel as Channel<TunerState>).onmessage;
      });
      settings.set({ ...DEFAULT_SETTINGS, haptics: true });
      await startListening();

      expect(messageHandler).toBeDefined();
      messageHandler!({
        kind: 'tracking',
        cents: -5,
        clarity: 0.9,
        db: -20,
        noteMidi: 40,
        targetHz: 440,
        f0: 435,
        stringIndices: [0],
      });
      expect(ipc.vibrate).not.toHaveBeenCalled();

      // Transition to intune
      messageHandler!({
        kind: 'intune',
        cents: 0,
        clarity: 0.98,
        db: -18,
        noteMidi: 40,
        targetHz: 440,
        f0: 440,
        stringIndices: [0],
      });
      expect(ipc.vibrate).toHaveBeenCalledTimes(1);

      // Subsequent intune state doesn't re-trigger vibrate
      messageHandler!({
        kind: 'intune',
        cents: 1,
        clarity: 0.98,
        db: -18,
        noteMidi: 40,
        targetHz: 440,
        f0: 440.2,
        stringIndices: [0],
      });
      expect(ipc.vibrate).toHaveBeenCalledTimes(1);
    });

    it('does not vibrate if haptics is disabled', async () => {
      let messageHandler: ((s: TunerState) => void) | undefined;
      vi.mocked(ipc.startListening).mockImplementationOnce(async (channel: unknown) => {
        messageHandler = (channel as Channel<TunerState>).onmessage;
      });
      settings.set({ ...DEFAULT_SETTINGS, haptics: false });
      await startListening();

      messageHandler!({
        kind: 'intune',
        cents: 0,
        clarity: 0.98,
        db: -18,
        noteMidi: 40,
        targetHz: 440,
        f0: 440,
        stringIndices: [0],
      });
      expect(ipc.vibrate).not.toHaveBeenCalled();
    });

    it('throttles vibration to at most once per 1000ms', async () => {
      vi.useFakeTimers();
      let messageHandler: ((s: TunerState) => void) | undefined;
      vi.mocked(ipc.startListening).mockImplementationOnce(async (channel: unknown) => {
        messageHandler = (channel as Channel<TunerState>).onmessage;
      });
      settings.set({ ...DEFAULT_SETTINGS, haptics: true });
      await startListening();

      const trackingState: TunerState = {
        kind: 'tracking',
        cents: -5,
        clarity: 0.9,
        db: -20,
        noteMidi: 40,
        targetHz: 440,
        f0: 435,
        stringIndices: [0],
      };
      const intuneState: TunerState = {
        kind: 'intune',
        cents: 0,
        clarity: 0.98,
        db: -18,
        noteMidi: 40,
        targetHz: 440,
        f0: 440,
        stringIndices: [0],
      };

      // First transition into intune
      messageHandler!(trackingState);
      messageHandler!(intuneState);
      expect(ipc.vibrate).toHaveBeenCalledTimes(1);

      // Fast toggle out and back in after 250ms (< 1000ms)
      vi.advanceTimersByTime(250);
      messageHandler!(trackingState);
      messageHandler!(intuneState);
      expect(ipc.vibrate).toHaveBeenCalledTimes(1);

      // After pause > 1000ms from the first vibration
      vi.advanceTimersByTime(800); // 250 + 800 = 1050ms > 1000ms
      messageHandler!(trackingState);
      messageHandler!(intuneState);
      expect(ipc.vibrate).toHaveBeenCalledTimes(2);

      vi.useRealTimers();
    });
  });

  describe('custom presets export/import (1.1)', () => {
    it('exports and imports valid presets, skips duplicates and invalid entries', async () => {
      const { customs, exportPresetsJson, importPresetsJson, savePreset } = await import(
        '../src/stores/customs'
      );
      customs.set([]);
      savePreset({
        id: 'custom.test1',
        instrument_id: 'guitar',
        name: { Custom: 'DADGAD' },
        strings: [38, 45, 50, 55, 57, 62],
        built_in: false,
      });

      const { json, count } = exportPresetsJson();
      expect(count).toBe(1);
      expect(json).toContain('DADGAD');

      // Re-importing the same JSON skips duplicate DADGAD, imports a new one, ignores unknown instrument
      const payload = JSON.stringify({
        format: 'unisono-presets',
        version: 1,
        presets: [
          { instrumentId: 'guitar', name: 'DADGAD', strings: [38, 45, 50, 55, 57, 62] },
          { instrumentId: 'guitar', name: 'Open G', strings: [38, 43, 50, 55, 59, 62] },
          { instrumentId: 'unknown_inst', name: 'Bad', strings: [40] },
        ],
      });
      const added = importPresetsJson(payload, new Set(['guitar']));
      expect(added).toBe(1);
      expect(get(customs).length).toBe(2);
      expect(importPresetsJson('not json', new Set(['guitar']))).toBeNull();
    });
  });
});
