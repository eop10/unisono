// @vitest-environment jsdom
//! Компонентные тесты: Uncertain (6.3.2), редактор (5.7), хоткеи 6.7.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { get } from 'svelte/store';
import { mockCatalog } from './mockCatalog';

vi.mock('../src/ipc/commands', () => ({
  getCatalog: vi.fn(async () => mockCatalog),
  startListening: vi.fn(async () => {}),
  stopListening: vi.fn(async () => {}),
  setReference: vi.fn(async () => {}),
  selectInstrument: vi.fn(async () => {}),
  selectTuning: vi.fn(async () => {}),
  selectString: vi.fn(async () => {}),
  setTolerance: vi.fn(async () => {}),
  setSensitivity: vi.fn(async () => {}),
  setLowRange: vi.fn(async () => {}),
  getSettings: vi.fn(async () => null),
  saveSettings: vi.fn(async () => {}),
  getPresets: vi.fn(async () => []),
  savePreset: vi.fn(async () => {}),
  deletePreset: vi.fn(async () => {}),
  getInputDevices: vi.fn(async () => []),
  setInputDevice: vi.fn(async () => {}),
  getPlatform: vi.fn(async () => 'desktop' as const),
  hasMicPermission: vi.fn(async () => true),
  requestMic: vi.fn(async () => {}),
  keepScreenOn: vi.fn(async () => {}),
  vibrate: vi.fn(async () => {}),
}));

import * as ipc from '../src/ipc/commands';
import SettingsSheet from '../src/lib/SettingsSheet.svelte';
import TunerScreen from '../src/lib/TunerScreen.svelte';
import TuningEditor from '../src/lib/TuningEditor.svelte';
import type { TunerState } from '../src/lib/types';
import { loadCatalog } from '../src/stores/catalog';
import { customs } from '../src/stores/customs';
import { editorContext, overlay } from '../src/stores/overlay';
import { DEFAULT_SETTINGS, settings } from '../src/stores/settings';
import { listening, resetPlatformCache, tuner } from '../src/stores/tuner';

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

function setState(partial: Partial<TunerState>): void {
  tuner.set({
    kind: 'tracking',
    f0: 82.0,
    cents: -7,
    clarity: 0.93,
    db: -24,
    noteMidi: 40,
    targetHz: 82.41,
    stringIndices: [0],
    ...partial,
  });
}

/** Клавиша «в окно» — как будто фокус нигде в поле ввода. */
function press(code: string): void {
  window.dispatchEvent(new KeyboardEvent('keydown', { code, bubbles: true, cancelable: true }));
}

beforeEach(async () => {
  vi.clearAllMocks();
  resetPlatformCache();
  listening.set(false);
  settings.set({ ...DEFAULT_SETTINGS });
  customs.set([]);
  overlay.set(null);
  editorContext.set(null);
  tuner.set({ ...SILENT });
  await loadCatalog();
});

afterEach(() => {
  cleanup();
  overlay.set(null);
  editorContext.set(null);
  customs.set([]);
  settings.set({ ...DEFAULT_SETTINGS });
});

describe('tuner screen: cents in Uncertain (6.3.2)', () => {
  it('tracking shows cents and the label', () => {
    setState({ kind: 'tracking', cents: -7 });
    const { container } = render(TunerScreen);
    expect(container.querySelector('.cents')?.textContent?.trim()).toBe('−7');
    expect(container.querySelector('.centslbl')?.classList.contains('hidden')).toBe(false);
  });

  it('uncertain hides digits and the cents label, gauge stays dimmed with needle', () => {
    setState({ kind: 'uncertain', cents: -7 });
    const { container } = render(TunerScreen);
    const cents = container.querySelector('.cents') as HTMLElement;
    expect(cents.textContent?.trim()).toBe('—');
    expect(cents.classList.contains('dim')).toBe(false);
    expect(container.querySelector('.centslbl')?.classList.contains('hidden')).toBe(true);
    // Шкала осталась приглушённой, скринридер не объявляет центы.
    const gauge = container.querySelector('.gauge') as SVGElement;
    expect(gauge.getAttribute('aria-valuetext')).toBe('—');
    // Игла на месте (последняя позиция), 1.6 — толщина иглы в CentsGauge.
    expect(gauge.querySelector('line[stroke-width="1.6"]')).not.toBeNull();
  });

  it('nopermission shows retry and settings buttons, retry calls startListening', async () => {
    setState({ kind: 'nopermission' });
    const { container } = render(TunerScreen);
    await tick();

    const buttons = [...container.querySelectorAll('.state-screen button')] as HTMLButtonElement[];
    expect(buttons.length).toBe(2);
    expect(buttons[0].classList.contains('primary')).toBe(true);
    expect(buttons[0].textContent?.trim()).toBe('Повторить');
    expect(buttons[1].textContent?.trim()).toBe('Открыть настройки');

    await fireEvent.click(buttons[0]);
    expect(ipc.startListening).toHaveBeenCalled();
  });
});

describe('tuning editor defaults (5.7)', () => {
  it('new tuning takes strings of the current tuning, not [64,69,74]', async () => {
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const { container } = render(TuningEditor);
    await tick();
    const rows = [...container.querySelectorAll('.string-edit-row select')] as HTMLSelectElement[];
    expect(rows.map((s) => s.value)).toEqual(['40', '45', '50', '55', '59', '64']);
  });

  it('default follows the currently selected tuning of the instrument', async () => {
    settings.update((s) => ({ ...s, instrumentId: 'guitar', tuningId: 'drop_d' }));
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const { container } = render(TuningEditor);
    await tick();
    const rows = [...container.querySelectorAll('.string-edit-row select')] as HTMLSelectElement[];
    expect(rows.map((s) => s.value)).toEqual(['38', '45', '50', '55', '59', '64']);
  });

  it('editing a preset loads its strings and name', async () => {
    customs.set([
      {
        id: 'custom.1',
        instrument_id: 'guitar',
        name: { Custom: 'Мой' },
        strings: [40, 40, 45],
        built_in: false,
      },
    ]);
    editorContext.set({ instrumentId: 'guitar', tuningId: 'custom.1' });
    const { container } = render(TuningEditor);
    await tick();
    const rows = [...container.querySelectorAll('.string-edit-row select')] as HTMLSelectElement[];
    expect(rows.map((s) => s.value)).toEqual(['40', '40', '45']);
    expect((container.querySelector('#te-name') as HTMLInputElement).value).toBe('Мой');
  });

  it('name input is capped at 24 characters', async () => {
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const { container } = render(TuningEditor);
    await tick();
    expect((container.querySelector('#te-name') as HTMLInputElement).getAttribute('maxlength')).toBe('24');
  });

  it('option labels follow settings.notation', async () => {
    settings.update((s) => ({ ...s, notation: 'solfeggio', language: 'ru' }));
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const { container } = render(TuningEditor);
    await tick();
    const sel = container.querySelector('.string-edit-row select') as HTMLSelectElement;
    expect(sel.selectedOptions[0]?.textContent).toContain('Ми2 · 40');

    cleanup();
    settings.update((s) => ({ ...s, notation: 'latin' }));
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const again = render(TuningEditor);
    await tick();
    const selLatin = again.container.querySelector('.string-edit-row select') as HTMLSelectElement;
    expect(selLatin.selectedOptions[0]?.textContent).toContain('E2 · 40');  });

  it('stale error is cleared by editing name or strings', async () => {
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const { container } = render(TuningEditor);
    await tick();
    // Пустое имя → ошибка.
    await fireEvent.click(container.querySelector('.actions .btn.primary') as HTMLElement);
    expect(container.querySelector('.err')).not.toBeNull();
    // Ввод струны сбрасывает ошибку.
    const sel = container.querySelector('.string-edit-row select') as HTMLSelectElement;
    await fireEvent.change(sel, { target: { value: '41' } });
    expect(container.querySelector('.err')).toBeNull();
    // Снова ошибка, ввод имени сбрасывает.
    await fireEvent.click(container.querySelector('.actions .btn.primary') as HTMLElement);
    expect(container.querySelector('.err')).not.toBeNull();
    await fireEvent.input(container.querySelector('#te-name') as HTMLInputElement, {
      target: { value: 'Мой строй' },
    });
    expect(container.querySelector('.err')).toBeNull();
  });
});

describe('hotkeys (6.7)', () => {
  it('digit keys select a string; repeat press returns to auto', async () => {
    render(TunerScreen);
    await tick();
    press('Digit1');
    expect(ipc.selectString).toHaveBeenLastCalledWith(0);
    press('Digit3');
    expect(ipc.selectString).toHaveBeenLastCalledWith(2);
    // Струна уже активна — то же нажатие возвращает автоопределение.
    tuner.set({ ...get(tuner), stringIndices: [2] });
    press('Digit3');
    expect(ipc.selectString).toHaveBeenLastCalledWith(null);
    // За пределами строя (у гитары 6 струн) — игнор.
    press('Digit8');
    expect(ipc.selectString).toHaveBeenCalledTimes(3);
  });

  it('digit keys are ignored in chromatic mode', async () => {
    settings.update((s) => ({ ...s, instrumentId: 'chromatic', tuningId: null }));
    render(TunerScreen);
    await tick();
    press('Digit1');
    expect(ipc.selectString).not.toHaveBeenCalled();
  });

  it('arrow keys adjust A4 by 0.1 and reach the engine', async () => {
    render(TunerScreen);
    await tick();
    press('ArrowUp');
    expect(get(settings).referenceHz).toBeCloseTo(440.1, 10);
    expect(ipc.setReference).toHaveBeenLastCalledWith(440.1);
    // Повтор (удержание клавиши) разрешён.
    press('ArrowDown');
    press('ArrowDown');
    expect(get(settings).referenceHz).toBeCloseTo(439.9, 10);
    expect(ipc.setReference).toHaveBeenLastCalledWith(439.9);
  });

  it('hotkeys do not intercept typing in inputs', async () => {
    render(TunerScreen);
    await tick();
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const { container } = render(TuningEditor);
    await tick();
    const before = get(settings).referenceHz;
    const input = container.querySelector('#te-name') as HTMLInputElement;
    input.dispatchEvent(
      new KeyboardEvent('keydown', { code: 'ArrowUp', bubbles: true, cancelable: true }),
    );
    expect(get(settings).referenceHz).toBe(before);
    expect(ipc.setReference).not.toHaveBeenCalled();
  });

  it('space toggles listening even when focus is on a button (rev 6.3)', async () => {
    const { container } = render(TunerScreen);
    await tick();
    const btn = container.querySelector('.chip') as HTMLButtonElement;
    btn.focus();
    expect(document.activeElement).toBe(btn);

    const btnEvent = new KeyboardEvent('keydown', { code: 'Space', bubbles: true, cancelable: true });
    btn.dispatchEvent(btnEvent);
    expect(btnEvent.defaultPrevented).toBe(true);
    await tick();
    await tick();
    expect(ipc.startListening).toHaveBeenCalled();

    // В поле ввода Space не перехватывается
    vi.clearAllMocks();
    editorContext.set({ instrumentId: 'guitar', tuningId: null });
    const { container: editorContainer } = render(TuningEditor);
    await tick();
    const input = editorContainer.querySelector('#te-name') as HTMLInputElement;
    input.focus();
    const inputEvent = new KeyboardEvent('keydown', { code: 'Space', bubbles: true, cancelable: true });
    input.dispatchEvent(inputEvent);
    expect(inputEvent.defaultPrevented).toBe(false);
    expect(ipc.startListening).not.toHaveBeenCalled();

    // При открытом листе Space не перехватывается
    vi.clearAllMocks();
    overlay.set('settings');
    await tick();
    const sheetEvent = new KeyboardEvent('keydown', { code: 'Space', bubbles: true, cancelable: true });
    window.dispatchEvent(sheetEvent);
    expect(sheetEvent.defaultPrevented).toBe(false);
    expect(ipc.startListening).not.toHaveBeenCalled();
  });
});

describe('settings sheet: input device row (stage 6)', () => {
  it('input device row is visible on desktop and lists devices with default option', async () => {
    vi.mocked(ipc.getPlatform).mockResolvedValueOnce('desktop');
    vi.mocked(ipc.getInputDevices).mockResolvedValueOnce(['Built-in Mic', 'AirPods']);
    const { container } = render(SettingsSheet);
    await tick();
    await tick();

    const select = container.querySelector('.device-select') as HTMLSelectElement;
    expect(select).not.toBeNull();
    const options = [...select.options];
    expect(options.length).toBe(3);
    expect(options[0].value).toBe('');
    expect(options[0].textContent).toBe('По умолчанию');
    expect(options[1].value).toBe('Built-in Mic');
    expect(options[1].textContent).toBe('Built-in Mic');
    expect(options[2].value).toBe('AirPods');
    expect(options[2].textContent).toBe('AirPods');
    expect(select.value).toBe('');
  });

  it('input device row is hidden on android', async () => {
    vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
    const { container } = render(SettingsSheet);
    await tick();
    await tick();

    expect(container.querySelector('.device-select')).toBeNull();
  });

  it('shows missing saved device with not found label', async () => {
    settings.update((s) => ({ ...s, inputDevice: 'Old USB Mic' }));
    vi.mocked(ipc.getPlatform).mockResolvedValueOnce('desktop');
    vi.mocked(ipc.getInputDevices).mockResolvedValueOnce(['Built-in Mic']);
    const { container } = render(SettingsSheet);
    await tick();
    await tick();

    const select = container.querySelector('.device-select') as HTMLSelectElement;
    expect(select).not.toBeNull();
    const options = [...select.options];
    expect(options.length).toBe(3);
    expect(options[2].value).toBe('Old USB Mic');
    expect(options[2].textContent).toContain('Old USB Mic (не найдено)');
    expect(select.value).toBe('Old USB Mic');
  });

  it('changing device writes settings and restarts listening if active', async () => {
    vi.mocked(ipc.getPlatform).mockResolvedValueOnce('desktop');
    vi.mocked(ipc.getInputDevices).mockResolvedValueOnce(['Built-in Mic', 'AirPods']);
    listening.set(true);
    const { container } = render(SettingsSheet);
    await tick();
    await tick();

    const select = container.querySelector('.device-select') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'AirPods' } });
    await tick();

    expect(get(settings).inputDevice).toBe('AirPods');
    expect(ipc.setInputDevice).toHaveBeenCalledWith('AirPods');
    expect(ipc.saveSettings).toHaveBeenCalled();
    expect(ipc.stopListening).toHaveBeenCalled();
    expect(ipc.startListening).toHaveBeenCalled();
  });

  it('selecting empty value sets inputDevice to null and does not restart if stopped', async () => {
    settings.update((s) => ({ ...s, inputDevice: 'AirPods' }));
    vi.mocked(ipc.getPlatform).mockResolvedValueOnce('desktop');
    vi.mocked(ipc.getInputDevices).mockResolvedValueOnce(['AirPods']);
    listening.set(false);
    const { container } = render(SettingsSheet);
    await tick();
    await tick();

    const select = container.querySelector('.device-select') as HTMLSelectElement;
    expect(select.value).toBe('AirPods');

    await fireEvent.change(select, { target: { value: '' } });
    await tick();

    expect(get(settings).inputDevice).toBeNull();
    expect(ipc.setInputDevice).toHaveBeenCalledWith(null);
    expect(ipc.saveSettings).toHaveBeenCalled();
    expect(ipc.stopListening).not.toHaveBeenCalled();
    expect(ipc.startListening).not.toHaveBeenCalled();
  });

  it('haptics row is hidden on desktop', async () => {
    vi.mocked(ipc.getPlatform).mockResolvedValueOnce('desktop');
    const { queryByRole } = render(SettingsSheet);
    await tick();
    await tick();
    expect(queryByRole('switch', { name: /вибрация/i })).toBeNull();
  });

  it('haptics row is visible on android', async () => {
    vi.mocked(ipc.getPlatform).mockResolvedValueOnce('android');
    const { queryByRole } = render(SettingsSheet);
    await tick();
    await tick();
    expect(queryByRole('switch', { name: /вибрация/i })).not.toBeNull();
  });
});

describe('manual string highlight in silence', () => {
  it('highlights manually selected string chip immediately in silence and toggles off on repeat', async () => {
    const { container } = render(TunerScreen);
    await tick();
    const chips = [...container.querySelectorAll<HTMLButtonElement>('.strings .str')];
    expect(chips.length).toBe(6);
    expect(chips[0].classList.contains('on')).toBe(false);

    press('Digit1');
    await tick();
    expect(chips[0].classList.contains('on')).toBe(true);
    expect(ipc.selectString).toHaveBeenLastCalledWith(0);

    press('Digit1');
    await tick();
    expect(chips[0].classList.contains('on')).toBe(false);
    expect(ipc.selectString).toHaveBeenLastCalledWith(null);
  });
});

describe('strobe mode (1.1)', () => {
  it('renders StrobeGauge when settings.strobe is true and toggles via SettingsSheet', async () => {
    settings.update((s) => ({ ...s, strobe: true }));
    const screen = render(TunerScreen);
    await tick();
    expect(screen.container.querySelector('svg.gauge.strobe')).not.toBeNull();
    screen.unmount();

    const sheet = render(SettingsSheet);
    await tick();
    await tick();
    const sw = sheet.getByRole('switch', { name: /стробоскоп/i });
    expect(sw.getAttribute('aria-checked')).toBe('true');
    await fireEvent.click(sw);
    expect(get(settings).strobe).toBe(false);
  });
});
