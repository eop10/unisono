// @vitest-environment jsdom
//! Компонентные тесты (часть 0): скрытие центов в Uncertain (6.3.2),
//! дефолт нового строя, maxlength=24, нотация в <option> (5.7, 6.9).

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { get } from 'svelte/store';
import { mockCatalog } from './mockCatalog';

vi.mock('../src/ipc/commands', () => ({
  getCatalog: async () => mockCatalog,
  startListening: async () => {},
  stopListening: async () => {},
  setReference: async () => {},
  selectInstrument: async () => {},
  selectTuning: async () => {},
  selectString: async () => {},
  setTolerance: async () => {},
  setSensitivity: async () => {},
}));

import TunerScreen from '../src/lib/TunerScreen.svelte';
import TuningEditor from '../src/lib/TuningEditor.svelte';
import type { TunerState } from '../src/lib/types';
import { loadCatalog } from '../src/stores/catalog';
import { customs } from '../src/stores/customs';
import { editorContext, overlay } from '../src/stores/overlay';
import { DEFAULT_SETTINGS, settings } from '../src/stores/settings';
import { tuner } from '../src/stores/tuner';

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

beforeEach(async () => {
  settings.set({ ...DEFAULT_SETTINGS });
  customs.set([]);
  overlay.set(null);
  editorContext.set(null);
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
