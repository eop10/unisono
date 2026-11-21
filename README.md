<p align="center">
  <img src="brand/readme-header.png" alt="Unisono" width="560">
</p>

<h1 align="center">Unisono</h1>

<p align="center">
  Мультиинструментальный тюнер с настраиваемой опорной частотой A4.<br>
  macOS · Windows · Linux · Android
</p>

> **Статус:** 0.1.0 в разработке. Спецификация, интерфейс и айдентика готовы;
> код начинается с этапа 0 по [`HANDOFF.md`](HANDOFF.md). Готовых сборок пока нет.

## Что это

Тюнер для струнных инструментов: гитара (6/7), бас (4/5), укулеле, скрипка, альт,
виолончель, контрабас, балалайка, домра (3/4), мандолина, банджо (5-струнное и тенор)
и хроматический режим без струн.

- **Опорная частота** 400–480 Гц с шагом 0,1: слайдер, список значений с пояснениями
  и ввод с клавиатуры.
- **Инструмент → строй → струны** — основная навигация; хроматический режим стоит
  в том же списке.
- **Приборный интерфейс**: нониус ±50/±5 центов, латунный ноль, крупная буква ноты.
- Полностью офлайн: без аккаунтов, сети и телеметрии.

## Платформы

| Платформа | Формат | Примечание |
| --- | --- | --- |
| macOS 12+ | Universal DMG | без нотаризации: первый запуск через «Всё равно открыть» |
| Windows 10/11 x64 | NSIS `.exe` | SmartScreen: «Подробнее» → «Выполнить всё равно» |
| Linux x86_64 | AppImage, `.deb` | Ubuntu 24.04+, Fedora 40+ |
| Android 8.0+ | APK | подпись собственным ключом, установка sideload |

## Структура

```text
unisono/
├── crates/            tuner-core, tuner-audio, tuner-storage
├── src-tauri/         оболочка Tauri 2
├── ui/                Svelte + TypeScript
├── gen/android/       Gradle-проект и Kotlin-шим (создаётся `tauri android init`)
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

**Linux.** `chmod +x Unisono-*.AppImage && ./Unisono-*.AppImage` или
`sudo apt install ./Unisono_*.deb`.

**Android.** Разрешить установку из неизвестных источников и открыть APK.
Play Protect может предупредить — это ожидаемо для sideload.

## Лицензия

Код — [Apache License 2.0](LICENSE). Название **Unisono** и логотип (`brand/`) —
собственность автора и не покрываются лицензией Apache 2.0.

## Приватность и контакты

Приложение не собирает данные и не использует сеть: см. [PRIVACY.md](PRIVACY.md).
Вопросы и безопасность: **egorprisiazhniy@gmail.com**.
