//! Имена нот: латиница и сольфеджио; в EN сольфеджио латиницей (6.9).

import type { Language, Notation } from './types';

const LATIN = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const SOLF_RU = ['До', 'До#', 'Ре', 'Ре#', 'Ми', 'Фа', 'Фа#', 'Соль', 'Соль#', 'Ля', 'Ля#', 'Си'];
const SOLF_EN = ['Do', 'Do#', 'Re', 'Re#', 'Mi', 'Fa', 'Fa#', 'Sol', 'Sol#', 'La', 'La#', 'Si'];

export function noteLetter(midi: number, notation: Notation, lang: Language): string {
  const idx = ((midi % 12) + 12) % 12;
  if (notation === 'latin') return LATIN[idx];
  return lang === 'ru' ? SOLF_RU[idx] : SOLF_EN[idx];
}

export function noteOctave(midi: number): number {
  return Math.floor(midi / 12) - 1;
}

/** Буквы струн без октав (6.3.6). */
export function stringLetters(strings: number[], notation: Notation, lang: Language): string[] {
  return strings.map((m) => noteLetter(m, notation, lang));
}