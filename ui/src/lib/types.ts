//! Типы каталога и состояния: зеркало serde-вывода `get_catalog` (tuner-core).

export interface Localized {
  ru: string;
  en: string;
}

export type InstrumentKind = 'Stringed' | 'Chromatic';

export interface Instrument {
  id: string;
  name: Localized;
  kind: InstrumentKind;
}

export type LocalizedName = { BuiltIn: Localized } | { Custom: string };

export interface Tuning {
  id: string;
  instrument_id: string;
  name: LocalizedName;
  strings: number[];
  built_in: boolean;
}

export interface Catalog {
  instruments: Instrument[];
  tunings: Tuning[];
}

export type TunerKind =
  | 'silent'
  | 'uncertain'
  | 'tracking'
  | 'intune'
  | 'error'
  | 'nopermission';

/** Мок состояния тюнера: на этапе 3 аудио нет, поток приходит из демо-панели. */
export interface TunerState {
  kind: TunerKind;
  f0: number | null;
  cents: number | null;
  clarity: number;
  db: number;
  noteMidi: number | null;
  targetHz: number | null;
  stringIndices: number[];
}

export type Notation = 'latin' | 'solfeggio';
export type Theme = 'system' | 'light' | 'dark';
export type Language = 'ru' | 'en';
export type Sensitivity = 'low' | 'normal' | 'high';
export type Tolerance = 2 | 3 | 5;

export interface Settings {
  referenceHz: number;
  instrumentId: string;
  tuningId: string | null;
  notation: Notation;
  theme: Theme;
  language: Language;
  bigType: boolean;
  toleranceCents: Tolerance;
  sensitivity: Sensitivity;
  lowRange: boolean;
  haptics: boolean;
  sound: boolean;
}

export type Overlay = null | 'instrument' | 'reference' | 'settings' | 'editor';

export interface EditorContext {
  instrumentId: string;
  tuningId: string | null;
}

export function tuningName(t: Tuning, lang: Language): string {
  if ('BuiltIn' in t.name) return t.name.BuiltIn[lang];
  return t.name.Custom;
}

export function instrumentName(i: Instrument, lang: Language): string {
  return i.name[lang];
}