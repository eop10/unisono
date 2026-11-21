(sec-data)=

# 5. Данные и хранение

## 5.1. Что нужно хранить

| Данные | Объём | Частота изменения | Требование |
| --- | --- | --- | --- |
| Инструмент (последний выбранный) | ID | редко | переживает перезапуск |
| Строй (последний выбранный) | ID | редко | то же |
| Опорная частота A4 | 8 байт | редко | то же |
| Настройки UI (нотация, тема, язык, вибрация, допуск, диапазон) | < 250 байт | редко | то же |
| Пользовательские строи | до 20 на инструмент × ~300 байт | редко | JSON, версия схемы, привязка к инструменту |
| Аудио | — | — | **не хранится** |
| Телеметрия | — | — | **не собирается** |

База данных не нужна: это десятки килобайт плоских данных. SQLite добавил бы размер и миграции
без выгоды (**ADR-003**).

## 5.2. Слои хранения

```text
tuner-storage (crate)
├── SettingsStore     settings.json  — инструмент, строй, A4, UI
├── PresetStore       presets.json   — пользовательские строи
└── AtomicJson<T>     временный файл + std::fs::rename
```

Пути (через `tauri::path()` в приложении, библиотека принимает `&Path`):

| Платформа | Каталог | Файлы |
| --- | --- | --- |
| macOS | `app_config_dir()` → `~/Library/Application Support/app.unisono.tuner/` | `settings.json`, `presets.json` |
| Android | `app_config_dir()` → `/data/data/app.unisono.tuner/files/` | `settings.json`, `presets.json` |
| Windows | `app_config_dir()` → `%APPDATA%\app.unisono.tuner\` | `settings.json`, `presets.json` |
| Linux | `app_config_dir()` → `~/.config/app.unisono.tuner/` | `settings.json`, `presets.json` |

Запись атомарная: `settings.json.tmp` → `sync_all()` → `rename`. Иначе выключение питания
в момент сохранения оставит битый JSON.

## 5.3. Настройки

`schema_version` пишется в оба файла.

| Поле | Тип | По умолчанию | Допустимый диапазон |
| --- | --- | --- | --- |
| `reference_hz` | `f64` | 440,0 | 400,0–480,0, шаг 0,1 |
| `instrument_id` | `String` | `guitar` | ID из 5.4 или `chromatic` |
| `tuning_id` | `Option<String>` | `standard_e` | строй выбранного инструмента; `null` для хроматики |
| `notation` | enum | `Latin` | `Latin`, `Solfeggio` |
| `theme` | enum | `System` | `System`, `Light`, `Dark` |
| `language` | enum | `Ru` | `Ru`, `En` |
| `haptics` | `bool` | `true` | — |
| `sound` | `bool` | `false` | — |
| `tolerance_cents` | `u8` | 3 | 2, 3, 5 |
| `sensitivity` | enum | `Normal` | `Low`, `Normal`, `High` (порог clarity 0,75 / 0,60 / 0,45) |
| `low_range` | `bool` | `false` | разрешает окно 16384 в хроматическом режиме |
| `big_type` | `bool` | `false` | — |
| `onboarding_done` | `bool` | `false` | — |
| `schema_version` | `u32` | 2 | 2 |

Правило чтения: значение вне диапазона **заменяется значением по умолчанию** без падения
и без диалога. Реализация — `Settings::sanitize()` после десериализации.

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub reference_hz: f64,
    pub instrument_id: String,
    pub tuning_id: Option<String>,
    pub notation: Notation,
    pub theme: Theme,
    pub language: Language,
    pub haptics: bool,
    pub sound: bool,
    pub tolerance_cents: u8,
    pub sensitivity: Sensitivity,
    pub low_range: bool,
    pub big_type: bool,
    pub onboarding_done: bool,
}
```

`#[serde(default)]` обязателен: при добавлении поля старый файл читается без ошибки.

## 5.4. Модель: инструмент → строй → струны

```rust
pub struct Instrument {
    pub id: &'static str,          // "guitar", "violin", "chromatic"
    pub name: Localized,           // { ru: "Гитара", en: "Guitar" }
    pub kind: InstrumentKind,      // Stringed | Chromatic
}

pub struct Tuning {
    pub id: String,                // "standard_e" | "custom.<uuid>"
    pub instrument_id: String,
    pub name: LocalizedName,       // либо { ru, en }, либо пользовательская строка
    pub strings: Vec<u8>,          // MIDI, от низкой к высокой; пусто для хроматики
    pub built_in: bool,
}
```

- Встроенные инструменты и строи — в коде (`music/instruments.rs`, `music/tunings.rs`).
- Пользовательский строй всегда привязан к инструменту: в списке другого инструмента его нет.
- Хроматический режим — инструмент `chromatic` с пустым списком струн и без строёв.
- Один инструмент может иметь несколько строёв (домра 3/4, банджо 5/тенор, бас 4/5, укулеле High/Low-G).

## 5.5. Встроенные инструменты и строи

MIDI-ноты от низкой струны к высокой; частоты указаны для A4 = 440 Гц и пересчитываются
при смене опорной частоты.

### Гитара (`guitar`)

| ID | Строй | Струны (MIDI) | Ноты |
| --- | --- | --- | --- |
| `standard_e` | Standard E | 40, 45, 50, 55, 59, 64 | E2 A2 D3 G3 B3 E4 |
| `drop_d` | Drop D | 38, 45, 50, 55, 59, 64 | D2 A2 D3 G3 B3 E4 |
| `eb` | Eb standard | 39, 44, 49, 54, 58, 63 | D#2 G#2 C#3 F#3 A#3 D#4 |
| `d_standard` | D standard | 38, 43, 48, 53, 57, 62 | D2 G2 C3 F3 A3 D4 |
| `drop_c` | Drop C | 36, 43, 48, 53, 57, 62 | C2 G2 C3 F3 A3 D4 |
| `dadgad` | DADGAD | 38, 45, 50, 55, 57, 62 | D2 A2 D3 G3 A3 D4 |
| `open_g` | Open G | 38, 43, 50, 55, 59, 62 | D2 G2 D3 G3 B3 D4 |
| `open_d` | Open D | 38, 45, 50, 54, 57, 62 | D2 A2 D3 F#3 A3 D4 |
| `guitar_7` | 7-струнная | 35, 40, 45, 50, 55, 59, 64 | B1 E2 A2 D3 G3 B3 E4 |

### Бас-гитара (`bass`)

| ID | Строй | Струны (MIDI) | Ноты |
| --- | --- | --- | --- |
| `bass4` | Бас 4 | 28, 33, 38, 43 | E1 A1 D2 G2 |
| `bass5` | Бас 5 | 23, 28, 33, 38, 43 | B0 E1 A1 D2 G2 |

### Укулеле (`ukulele`)

| ID | Строй | Струны (MIDI) | Ноты |
| --- | --- | --- | --- |
| `uke_high_g` | High-G | 67, 60, 64, 69 | G4 C4 E4 A4 |
| `uke_low_g` | Low-G | 55, 60, 64, 69 | G3 C4 E4 A4 |

### Скрипка (`violin`), альт (`viola`), виолончель (`cello`), контрабас (`contrabass`)

| Инструмент | ID | Строй | Струны (MIDI) | Ноты |
| --- | --- | --- | --- | --- |
| Скрипка | `violin_std` | Стандарт | 55, 62, 69, 76 | G3 D4 A4 E5 |
| Альт | `viola_std` | Стандарт | 48, 55, 62, 69 | C3 G3 D4 A4 |
| Виолончель | `cello_std` | Стандарт | 36, 43, 50, 57 | C2 G2 D3 A3 |
| Контрабас | `contrabass_std` | Стандарт | 28, 33, 38, 43 | E1 A1 D2 G2 |

ID всех встроенных строёв **глобально уникальны**: даже при одинаковом названии «Стандарт»
у скрипки и виолончели разные ID. Поиск всё равно выполняется парой `(instrument_id, tuning_id)`,
но уникальность упрощает хранение, deep links и логи.

Контрабас звучит на октаву ниже записи, но тюнер измеряет **фактическую** частоту,
поэтому в модели хранятся реальные MIDI-ноты.

### Балалайка (`balalaika`)

| ID | Строй | Струны (MIDI) | Ноты | Особенность |
| --- | --- | --- | --- | --- |
| `balalaika_prima` | Прима | 64, 64, 69 | E4 E4 A4 | две струны-унисона |

### Домра (`domra`)

| ID | Строй | Струны (MIDI) | Ноты |
| --- | --- | --- | --- |
| `domra_3` | 3-струнная (квартовый) | 64, 69, 74 | E4 A4 D5 |
| `domra_4` | 4-струнная (квинтовый) | 55, 62, 69, 76 | G3 D4 A4 E5 |

### Мандолина (`mandolin`)

| ID | Строй | Струны (MIDI) | Ноты | Особенность |
| --- | --- | --- | --- | --- |
| `mandolin_std` | Стандарт | 55, 62, 69, 76 | G3 D4 A4 E5 | 4 пары струн |

Мандолина имеет 8 струн, но 4 курса; тюнер показывает 4 чипа, пара настраивается как одна струна.

### Банджо (`banjo`)

| ID | Строй | Струны (MIDI) | Ноты |
| --- | --- | --- | --- |
| `banjo_5` | 5-струнное, Open G | 50, 55, 59, 62, 67 | D3 G3 B3 D4 G4 |
| `banjo_tenor` | Тенор C-G-D-A | 48, 55, 62, 69 | C3 G3 D4 A4 |

### Хроматический (`chromatic`)

Строёв нет, струн нет. Диапазон 30–1400 Гц, показывается ближайшая нота равномерной темперации.

## 5.6. Пользовательские строи

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TuningPreset {
    pub id: String,              // "custom.<uuid>"
    pub instrument_id: String,
    pub name: String,            // 1..24 символа, пользовательский текст
    pub strings: Vec<u8>,        // MIDI 21..96, 1..8 элементов
    #[serde(default)]
    pub built_in: bool,
}
```

Пример `presets.json`:

```json
{
  "schema_version": 2,
  "presets": [
    {
      "id": "custom.3f1c9a2e-7b44-4a1f-9d0e-6b2f5c8d1a77",
      "instrument_id": "domra",
      "name": "Мой квартовый",
      "strings": [64, 69, 74],
      "built_in": false
    }
  ]
}
```

Встроенные строи в файл не пишутся — они в коде, иначе обновление приложения не доедет до пользователя.

## 5.7. Валидация

| Правило | Сообщение | Блокирует |
| --- | --- | --- |
| Имя не пустое после trim | «Введите название» | да |
| Длина имени ≤ 24 | «Не больше 24 символов» | да |
| Имя уникально в пределах инструмента (без регистра) | «Такой строй уже есть» | да |
| Струн 1–8 | «От 1 до 8 струн» | да |
| MIDI в диапазоне 21–96 | «Нота вне поддерживаемого диапазона» | да |
| Дубли нот в строе | «Две одинаковые струны» | нет (унисон — норма) |
| Лимит 20 пользовательских строёв на инструмент | «Удалите один из сохранённых строёв» | да |

Дубли нот разрешены осознанно: балалайка-прима и мандолина имеют унисонные струны.
Автоопределение струны обязано подсвечивать пару, а не выбирать одну случайно (ФТ-15).

Встроенные строи нельзя удалить или переименовать; можно «клонировать в свой» — так пользователь
меняет Drop D, не теряя оригинал.

## 5.8. Миграции

1. Версия совпадает — читаем как есть.
2. Версия меньше текущей — цепочка миграций; для схемы 1→2 у старых настроек добавляется
   `instrument_id = "guitar"`, `tuning_id = "standard_e"`, `language = "ru"`, `schema_version = 2`.
3. Версия больше текущей — читаем известные поля, файл не перезаписываем, показываем уведомление.
4. JSON невалиден — переименовываем в `presets.corrupt-<timestamp>.json`, стартуем со встроенными,
   причину пишем в `tracing`.
