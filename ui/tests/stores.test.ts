import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { mockCatalog } from './mockCatalog';

vi.mock('../src/ipc/commands', () => ({
  getCatalog: async () => mockCatalog,
}));

import { loadCatalog, tuningsFor } from '../src/stores/catalog';
import {
  currentTuning,
  selectInstrument,
  selectTuning,
  settings,
  showStrings,
  DEFAULT_SETTINGS,
} from '../src/stores/settings';
import { selectString, tuner } from '../src/stores/tuner';
import { customs } from '../src/stores/customs';

beforeEach(async () => {
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

  it('chromatic hides strings and has no tuning', () => {
    selectInstrument('chromatic');
    expect(get(settings).tuningId).toBeNull();
    expect(get(showStrings)).toBe(false);
    expect(tuningsFor('chromatic')).toEqual([]);
    selectInstrument('guitar');
    expect(get(showStrings)).toBe(true);
  });

  it('restores last tuning when returning to instrument', () => {
    selectTuning('guitar', 'drop_d');
    selectInstrument('violin');
    selectInstrument('guitar');
    expect(get(settings).tuningId).toBe('drop_d');
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

  it('unison tap selects the whole pair', () => {
    selectInstrument('balalaika');
    selectString([0, 1], 64);
    expect(get(tuner).stringIndices).toEqual([0, 1]);
    expect(get(tuner).noteMidi).toBe(64);
  });
});