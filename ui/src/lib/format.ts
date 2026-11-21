//! Локализованные числа и центы (НФТ-12, ФТ-10): запятая в RU, точка в EN;
//! центы без ведущего нуля, минус — типографский U+2212.

import type { Language } from './types';

export function fmtNumber(value: number, digits: number, lang: Language): string {
  const s = value.toFixed(digits);
  return lang === 'ru' ? s.replace('.', ',') : s;
}

export function fmtHz(value: number, lang: Language): string {
  return fmtNumber(value, 1, lang);
}

export function fmtCents(cents: number): string {
  const c = Math.round(cents);
  if (c === 0) return '0';
  if (c > 0) return `+${c}`;
  return `−${Math.abs(c)}`;
}

/** Ввод опорной частоты: принимает запятую и точку, шаг 0,1, границы 400–480. */
export function parseReference(raw: string): number | null {
  const s = raw.trim().replace(',', '.');
  if (s === '' || !/^\d+(\.\d+)?$/.test(s)) return null;
  const v = Number(s);
  if (!Number.isFinite(v)) return null;
  return clampReference(v);
}

export function clampReference(v: number): number {
  const clamped = Math.min(480, Math.max(400, v));
  return Math.round(clamped * 10) / 10;
}