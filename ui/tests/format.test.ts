import { describe, expect, it } from 'vitest';
import { clampReference, fmtCents, fmtHz, fmtNumber, parseReference } from '../src/lib/format';

describe('localized numbers', () => {
  it('uses comma in RU and dot in EN', () => {
    expect(fmtNumber(82.41, 1, 'ru')).toBe('82,4');
    expect(fmtNumber(82.41, 1, 'en')).toBe('82.4');
    expect(fmtHz(442, 'ru')).toBe('442,0');
    expect(fmtHz(442, 'en')).toBe('442.0');
  });
});

describe('cents formatting', () => {
  it('no leading zero, typographic minus, explicit plus', () => {
    expect(fmtCents(-7)).toBe('−7');
    expect(fmtCents(12)).toBe('+12');
    expect(fmtCents(0)).toBe('0');
    expect(fmtCents(-0.4)).toBe('0');
    expect(fmtCents(-7.6)).toBe('−8');
  });
});

describe('reference input', () => {
  it('accepts comma and dot, rounds to 0.1, clamps 400-480', () => {
    expect(parseReference('442')).toBe(442);
    expect(parseReference('442,0')).toBe(442);
    expect(parseReference('442.05')).toBe(442.1);
    expect(parseReference(' 441,55 ')).toBe(441.6);
    expect(parseReference('500')).toBe(480);
    expect(parseReference('300')).toBe(400);
    expect(parseReference('abc')).toBeNull();
    expect(parseReference('')).toBeNull();
    expect(parseReference('-440')).toBeNull();
    expect(clampReference(440.04)).toBe(440);
  });
});