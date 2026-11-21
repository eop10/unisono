import { describe, expect, it } from 'vitest';
import { validateTuning } from '../src/lib/validate';

const existing = [
  { id: 'custom.1', name: 'Мой квартовый' },
  { id: 'custom.2', name: 'Другой' },
];

describe('tuning editor validation (5.7)', () => {
  it('name required and trimmed', () => {
    expect(validateTuning('   ', [64], [], null)).toBe('name_required');
    expect(validateTuning(' _ok ', [64], [], null)).toBeNull();
  });
  it('name at most 24 chars', () => {
    expect(validateTuning('x'.repeat(25), [64], [], null)).toBe('name_long');
    expect(validateTuning('x'.repeat(24), [64], [], null)).toBeNull();
  });
  it('name unique per instrument, case-insensitive, editing excluded', () => {
    expect(validateTuning('мой квартовый', [64], existing, null)).toBe('name_dup');
    expect(validateTuning('Мой квартовый', [64], existing, 'custom.1')).toBeNull();
    expect(validateTuning('Новый', [64], existing, null)).toBeNull();
  });
  it('1-8 strings', () => {
    expect(validateTuning('a', [], [], null)).toBe('strings_count');
    expect(validateTuning('a', [64, 64, 64, 64, 64, 64, 64, 64, 64], [], null)).toBe('strings_count');
    expect(validateTuning('a', [64, 64], [], null)).toBeNull(); // унисон разрешён
  });
  it('MIDI 21-96', () => {
    expect(validateTuning('a', [20], [], null)).toBe('midi_range');
    expect(validateTuning('a', [97], [], null)).toBe('midi_range');
    expect(validateTuning('a', [21, 96], [], null)).toBeNull();
  });
  it('limit 20 presets per instrument, editing does not count', () => {
    const twenty = Array.from({ length: 20 }, (_, i) => ({ id: `c${i}`, name: `n${i}` }));
    expect(validateTuning('new', [64], twenty, null)).toBe('preset_limit');
    expect(validateTuning('n0', [64], twenty, 'c0')).toBeNull();
  });
});