<p align="center">
  <img src="brand/readme-header.png" alt="Unisono" width="560">
</p>

<h1 align="center">Unisono</h1>

<p align="center">
  Мультиинструментальный тюнер с настраиваемой опорной частотой A4.<br>
  macOS · Android · Windows · Linux
</p>

<p align="center">
  <a href="https://github.com/eop10/unisono/releases/tag/v0.2.0"><img src="https://img.shields.io/github/v/release/eop10/unisono?style=flat-square&color=C8A253" alt="Release"></a>
  <a href="https://github.com/eop10/unisono/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/eop10/unisono/ci.yml?branch=main&style=flat-square" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Android%20%7C%20Windows%20%7C%20Linux-lightgrey?style=flat-square" alt="Platforms">
</p>

<p align="center">
  <strong>Русский</strong> · <a href="README.en.md">English</a>
</p>

> **Статус:** выпущена версия **`0.2.0`** для **macOS**, **Windows**, **Linux** и **Android** (см. [GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0)).

## Скачать v0.2.0

| Платформа | Формат | Примечание |
| --- | --- | --- |
| macOS (Apple Silicon / Intel) | `.dmg` (`aarch64`, `x86_64`) | [Скачать с GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Windows 10/11 x64 | NSIS `.exe` | [Скачать с GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Linux x86_64 | `.deb`, `.rpm`, `.AppImage`, `.pkg.tar.zst` | [Скачать с GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Android 8.0+ | `.apk` (arm64-v8a / armeabi-v7a) | [Скачать с GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Контрольные суммы | `SHA256SUMS.txt` | В разделе Assets релиза `v0.2.0` |

## Что это

Тюнер для струнных инструментов: гитара (6/7), бас (4/5), укулеле, скрипка, альт,
виолончель, контрабас, балалайка, домра (3/4), мандолина, банджо (5-струнное и тенор)
и хроматический режим без струн.

## Ключевые преимущества

- **Свои строи и экспорт/импорт:** создавайте, редактируйте, экспортируйте и импортируйте
  собственные строи в JSON (например, `DADGAD` или `Drop C`) для любого инструмента — всё хранится
  локально и переживает перезапуск.
- **Точная опорная частота A4 (400–480 Гц, шаг 0,1 Гц):** быстрый выбор из пресетов, слайдер,
  прямой ввод, калибровка с микрофона и подстройка клавишами `↑`/`↓`.
- **Камертон, каподастр и темперация:** встроенный генератор чистого опорного тона с автопаузой
  микрофона, сдвиг каподастра (`−6…+12` полутонов), автопереход к следующей струне и режимы
  темперации (`Равномерная`, `Сладкий строй`, `Чистые квинты`).
- **Проверка мензуры и график стабильности:** замер открытой струны и 12-го лада с подсказкой
  регулировки седла, стробоскопическая шкала и скользящий мини-график отклонения.
- **Умное определение струн и унисонов:** автоопределение с гистерезисом (включая подсветку
  обеих струн унисон-пары `E4+E4` на балалайке) или ручная фиксация цели тапом / клавишами `1..8`.
- **Фиксация показаний при затухании:** после удара по струне результат мягко удерживается на
  экране (~2,5 с), чтобы не приходилось ловить ускользающую стрелку, а новая атака подхватывается
  мгновенно.
- **Точный DSP и приборный интерфейс:** шкала-нониус ±50/±5 центов с латунным нулём, адаптивное
  окно анализа (4096 / 8192 / 16384 сэмплов) для уверенного захвата даже низких нот баса `B0`,
  настраиваемый допуск «в строе» (`±2/3/5` центов) и чувствительность.
- **Обратная связь «в строе»:** мягкий звуковой сигнал (desktop/Android) и тактильный отклик
  вибрацией (Android), чтобы настраиваться даже не глядя на экран.
- **Внешние аудиокарты и управление с клавиатуры (macOS):** выбор конкретного микрофона или
  аудиоинтерфейса в настройках (с автоматическим сведением многоканальных входов в моно),
  глобальный `Space` для старт/стопа, `1..8` для струн, `↑`/`↓` для A4.
- **Доступность и приватность:** светлая, тёмная и системная темы, режим крупного шрифта,
  поддержка скринридеров (VoiceOver / TalkBack) и `prefers-reduced-motion`, русский и английский
  языки (`C-D-E` / `До-Ре-Ми`). **Полностью офлайн:** без рекламы, подписок, сети и телеметрии.

## Платформы

| Платформа | Формат | Примечание |
| --- | --- | --- |
| macOS 12+ | DMG per-arch (aarch64 / x86_64) | без нотаризации: первый запуск через «Всё равно открыть» |
| Windows 10/11 x64 | NSIS `.exe` | SmartScreen: «Подробнее» → «Выполнить всё равно» |
| Linux x86_64 | AppImage, `.deb` | Ubuntu 24.04+, Fedora 40+ |
| Android 8.0+ | APK | подпись собственным ключом, установка sideload |

## Структура

```text
unisono/
├── crates/            tuner-core, tuner-audio, tuner-storage
├── src-tauri/         оболочка Tauri 2
├── ui/                Svelte + TypeScript
├── src-tauri/gen/android/       Gradle-проект и Kotlin-шим (создаётся `tauri android init`)
├── brand/             логотип, иконки (.icns, .ico, Android adaptive)
├── design/            интерактивный прототип и макеты
├── docs/              Sphinx-документация: требования, архитектура, DSP, UI, план
├── HANDOFF.md         порядок работ и критерии готовности для разработки
└── tools/             генератор тестовых WAV
```

## Документация

```bash
cd docs
python3 -m venv .venv
./.venv/bin/pip install -r requirements.txt
make html          # -> docs/build/html/index.html
make pdf           # -> docs/dist/documentation.pdf
```

Ключевые разделы: `docs/source/02_requirements.md`, `03_architecture.md`, `04_dsp.md`,
`05_data.md`, `06_ui.md`, `09_plan.md`.

## Сборка

Понадобятся Rust, Node.js 22+, JDK 17 и (для Android) Android SDK/NDK.

```bash
npm install
npm run tauri dev            # desktop
npm run tauri build          # текущая ОС
npm run tauri android dev    # Android
```

Точные команды и зависимости — в `docs/source/07_platform.md` и `08_build_test.md`.

## Первый запуск (сборки без подписи)

**macOS.** Открыть DMG, перетащить в Applications, затем:
- macOS 14 и старше: правый клик по `Unisono.app` → «Открыть» → «Открыть»;
- macOS 15+: «Системные настройки → Конфиденциальность и безопасность → Всё равно открыть».

**Windows.** Запустить установщик → «Подробнее» → «Выполнить всё равно».

**Linux.**
- Ubuntu / Debian: `sudo apt install ./Unisono_*.deb`
- Fedora / openSUSE: `sudo dnf install ./Unisono-*.rpm`
- Arch / Manjaro: `sudo pacman -U ./unisono-bin-*.pkg.tar.zst` (или [`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD))
- Универсальный AppImage: `chmod +x Unisono-*.AppImage && ./Unisono-*.AppImage`
- Если на GPU NVIDIA + Wayland окно WebKitGTK открывается белым или не рисуется:
  `WEBKIT_DISABLE_DMABUF_RENDERER=1 unisono`

**Android (sideload).**
- Требования: Android 8.0+ (API 26+), архитектуры `arm64-v8a` или `armeabi-v7a`.
- Разрешите установку из неизвестных источников для браузера или проводника в системных настройках безопасности.
- Откройте APK. Play Protect может выдать предупреждение о неизвестном издателе — это штатно для sideload, выберите «Всё равно установить».
- Обновление поверх: последующие сборки устанавливаются поверх существующего приложения без потери пользовательских строев и настроек при условии совпадения ключа подписи (`UNISONO_KEYSTORE`).

## Лицензия

Код — [Apache License 2.0](LICENSE). Название **Unisono** и логотип (`brand/`) —
собственность автора и не покрываются лицензией Apache 2.0.

## Приватность и контакты

Приложение не собирает данные и не использует сеть: см. [PRIVACY.md](PRIVACY.md).
Вопросы и безопасность: **egorprisiazhniy@gmail.com**.
