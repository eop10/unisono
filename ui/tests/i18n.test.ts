import { describe, expect, it } from 'vitest';
import { t } from '../src/lib/i18n';
import { noteLetter, noteOctave } from '../src/lib/notes';

describe('i18n', () => {
  it('switches RU/EN', () => {
    expect(t('ru', 'in_tune')).toBe('В строе');
    expect(t('en', 'in_tune')).toBe('In tune');
    expect(t('ru', 'listening')).toBe('Слушаю…');
    expect(t('en', 'listening')).toBe('Listening…');
  });
  it('status never repeats the cents number', () => {
    for (const key of ['below', 'above', 'in_tune', 'listening'] as const) {
      expect(t('ru', key)).not.toMatch(/\d/);
      expect(t('en', key)).not.toMatch(/\d/);
    }
  });
});

describe('note names', () => {
  it('latin and solfeggio, EN solfeggio latinized', () => {
    expect(noteLetter(64, 'latin', 'ru')).toBe('E');
    expect(noteLetter(64, 'solfeggio', 'ru')).toBe('Ми');
    expect(noteLetter(64, 'solfeggio', 'en')).toBe('Mi');
    expect(noteLetter(69, 'solfeggio', 'en')).toBe('La');
    expect(noteOctave(40)).toBe(2);
    expect(noteOctave(23)).toBe(0);
  });
});