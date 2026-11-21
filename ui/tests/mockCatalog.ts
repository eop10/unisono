//! Мок каталога для vitest (Prompt 1.1): подмножество, не копия 23 строёв.

import type { Catalog } from '../src/lib/types';

export const mockCatalog: Catalog = {
  instruments: [
    { id: 'guitar', name: { ru: 'Гитара', en: 'Guitar' }, kind: 'Stringed' },
    { id: 'violin', name: { ru: 'Скрипка', en: 'Violin' }, kind: 'Stringed' },
    { id: 'balalaika', name: { ru: 'Балалайка', en: 'Balalaika' }, kind: 'Stringed' },
    { id: 'chromatic', name: { ru: 'Хроматический', en: 'Chromatic' }, kind: 'Chromatic' },
  ],
  tunings: [
    {
      id: 'standard_e',
      instrument_id: 'guitar',
      name: { BuiltIn: { ru: 'Стандарт E', en: 'Standard E' } },
      strings: [40, 45, 50, 55, 59, 64],
      built_in: true,
    },
    {
      id: 'drop_d',
      instrument_id: 'guitar',
      name: { BuiltIn: { ru: 'Drop D', en: 'Drop D' } },
      strings: [38, 45, 50, 55, 59, 64],
      built_in: true,
    },
    {
      id: 'violin_std',
      instrument_id: 'violin',
      name: { BuiltIn: { ru: 'Стандарт', en: 'Standard' } },
      strings: [55, 62, 69, 76],
      built_in: true,
    },
    {
      id: 'balalaika_prima',
      instrument_id: 'balalaika',
      name: { BuiltIn: { ru: 'Прима', en: 'Prima' } },
      strings: [64, 64, 69],
      built_in: true,
    },
  ],
};