//! Валидация пользовательского строя (docs/source/05_data.md, 5.7).
//! Дубли нот разрешены (унисон), лимит 20 на инструмент.

export const MAX_STRINGS = 8;
export const MIN_STRINGS = 1;
export const MIDI_MIN = 21;
export const MIDI_MAX = 96;
export const MAX_PRESETS_PER_INSTRUMENT = 20;
export const NAME_MAX = 24;

export type ValidationError =
  | 'name_required'
  | 'name_long'
  | 'name_dup'
  | 'strings_count'
  | 'midi_range'
  | 'preset_limit';

export interface ExistingPreset {
  id: string;
  name: string;
}

export function validateTuning(
  name: string,
  strings: number[],
  existing: ExistingPreset[],
  editingId: string | null,
): ValidationError | null {
  const trimmed = name.trim();
  if (trimmed.length === 0) return 'name_required';
  if (trimmed.length > NAME_MAX) return 'name_long';
  const dup = existing.some(
    (e) => e.id !== editingId && e.name.trim().toLowerCase() === trimmed.toLowerCase(),
  );
  if (dup) return 'name_dup';
  if (strings.length < MIN_STRINGS || strings.length > MAX_STRINGS) return 'strings_count';
  if (strings.some((m) => m < MIDI_MIN || m > MIDI_MAX)) return 'midi_range';
  if (editingId === null && existing.length >= MAX_PRESETS_PER_INSTRUMENT) return 'preset_limit';
  return null;
}