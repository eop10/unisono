# 1. Ядро музыки: инструменты, строи, ноты

## Цель
Реализовать `tuner-core::music`: ноты, MIDI, центы, встроенные инструменты и строи, выбор окна анализа.

## Задачи
- [ ] `NoteMath`: `hz(midi, a4)`, `cents(f, target)`, `nearest_midi(f, a4)`.
- [ ] `Instrument` и `Tuning` из `docs/source/05_data.md`; 11 инструментов, 23 строя, хроматика.
- [ ] `InstrumentRegistry`: `tunings_for(instrument)`, `lowest_note(tuning)`, `window_size(tuning)` (4096/8192/16384).
- [ ] Правило унисонов: дубли нот разрешены, `StringMatcher` умеет отдавать пару.
- [ ] Тесты: A2 при 442 = 110,50 Гц; скрипка G3/E5, виолончель C2, бас-5 B0, балалайка E4+E4.

## Готово, когда
Все строи из 5.5 покрыты тестами; целевые частоты совпадают с таблицей; `window_size` возвращает верное окно.

## Ссылки
`docs/source/05_data.md`, `docs/source/04_dsp.md` §4.2.3.
