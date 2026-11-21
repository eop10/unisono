# Передача в разработку

Документ для разработчика (Квен-4): что уже решено, в каком порядке делать и когда этап
считается готовым. Источник истины — `docs/source/`; при расхождении выигрывает документация,
этот файл — краткая навигация.

## 1. Контекст

Unisono — мультиинструментальный тюнер (Rust + Tauri 2 + Svelte) для macOS, Windows, Linux
и Android. Продукт офлайновый, без аккаунтов и телеметрии. Версия — 0.1.0,
package/bundle id — `app.unisono.tuner`.

Сейчас в репозитории нет кода: только спецификация, айдентика и прототип. Задача — пройти
этапы 0–10 из `docs/source/09_plan.md`, не нарушая решений из `10_risks.md` (D-1…D-16)
и ADR (`11_adr.md`).

## 2. Что читать и в каком порядке

1. `docs/source/02_requirements.md` — объём и критерии приёмки.
2. `docs/source/05_data.md` — инструменты, строи, формат настроек.
3. `docs/source/04_dsp.md` — NSDF/MPM, окна, точность.
4. `docs/source/03_architecture.md` — модули и потоки.
5. `docs/source/06_ui.md` — экраны, состояния, токены, айдентика.
6. `docs/source/07_platform.md` — Android/macOS/Windows/Linux.
7. `docs/source/08_build_test.md` — тесты, CI, чек-листы.
8. `docs/source/09_plan.md` — этапы и оценки.
9. `docs/source/10_risks.md` — решения и открытые вопросы.
10. `docs/source/11_adr.md`, `12_glossary.md` — справка.

Собранная версия: `make -C docs html` → `docs/build/html/index.html`.

## 3. Жёсткие правила

- `tuner-core` не знает про аудио, Tauri, файлы и платформы. Никакого `unsafe` в нём.
- Колбэк аудио: без аллокаций, `Mutex`, `println!` и блокировок. Только `rtrb::Producer`.
- Окно анализа выбирается по нижней ноте строя: 4096 / 8192 / 16384. Не фиксировать глобально.
- Модель данных: инструмент → строй → струны; пользовательский строй привязан к инструменту.
- Дубли нот разрешены (балалайка E4+E4, мандолина): автоопределение подсвечивает пару.
- UI: буква ноты без наклона; вес меняется только при «в строе» (560 → 720).
- Число центов не дублируется в строке статуса.
- Android-keystore и пароли не попадают в репозиторий.
- Сборки без подписи/нотаризации; инструкции первого запуска — часть README.

## 4. Структура, которую нужно создать

```text
Cargo.toml                 # workspace: crates/*, src-tauri
crates/
  tuner-core/              # dsp, music (instruments, tunings, note), engine
  tuner-audio/             # trait AudioSource + cpal/oboe
  tuner-storage/           # settings.json, presets.json, миграции
src-tauri/                 # Tauri 2, commands, channel
ui/                        # Svelte + TS + Vite
gen/android/               # создаётся `tauri android init`, коммитится
brand/                     # готовые иконки (не перерисовывать)
design/                    # прототип и макеты
docs/                      # спецификация
```

`brand/` и `design/` уже готовы. `crates/`, `src-tauri/`, `ui/`, `gen/android/` создаются
на этапе 0.

## 5. Этап 0: каркас (3 дня)

1. `cargo new --lib crates/tuner-core`, `crates/tuner-audio`, `crates/tuner-storage`;
   корневой `Cargo.toml` с `[workspace]`, `resolver = "2"`, edition 2021, release-профиль
   (`lto`, `codegen-units = 1`, `panic = "abort"`, `strip`).
2. `npm create vite@latest ui -- --template svelte-ts`; `npm install` в корне.
3. `npm run tauri init` → `src-tauri/`; `identifier = app.unisono.tuner`,
   `productName = Unisono`, окно 900×640.
4. `npm run tauri android init` → `gen/android/`; **закоммитить** сгенерированный проект.
5. CI: `.github/workflows/docs.yml` уже есть; добавить `ci.yml` с `cargo fmt --check`,
   `cargo clippy -D warnings`, `cargo test`, `npm --prefix ui run build`.
6. Проверка: пустое окно открывается на macOS, Android, Windows и Linux; `cargo test` зелёный.

## 6. Этапы 1–10: критерии готовности

| Этап | Готово, когда |
| --- | --- |
| 1. Ядро музыки | все строи из 5.5 в коде; A2 при 442 = 110,50 Гц; скрипка/виолончель/бас дают верные цели |
| 2. DSP | фикстуры 8.3 проходят для всех групп; `criterion` в бюджете; 0 октавных ошибок на низких нотах |
| 3. UI | тюнер, нониус, лист «Инструмент · Строй», хроматика, A4 с вводом, настройки, редактор, RU/EN |
| 4. macOS | `cpal`, DMG, ad-hoc подпись, первый запуск по README |
| 5. Android | `oboe`, Kotlin-шим, разрешения, sideload APK с собственным ключом |
| 6. Полировка | чек-лист 8.4 полностью, reduced-motion, доступность |
| 7. Выпуск | GitHub Releases: DMG, `.exe`, AppImage, `.deb`, APK, SHA-256 |
| 8. Айдентика | иконки из `brand/` в Dock, Explorer, Android launcher |
| 9. Windows | WASAPI, NSIS, SmartScreen-инструкция |
| 10. Linux | ALSA/PipeWire, AppImage/deb, проверка на Ubuntu 24.04 и Fedora 40 |

## 7. Ассеты

- Иконки: `brand/icon.icns` (macOS), `brand/icon.ico` (Windows), `brand/android/mipmap-*`
  (Android), `brand/png/icon-*` (общие). Мастер — `brand/src/unisono-mark.svg`.
- Шапка репозитория: `brand/readme-header.png`.
- Прототип интерфейса: `design/prototype.html` (открывать через локальный сервер,
  шрифты в `design/fonts/`).
- Не перерисовывать иконки вручную: только из мастер-SVG.

## 8. Версии и ключи

- Версия стартует с `0.1.0`; в Android — `versionCode 1`; каждый релиз +1.
- Android-keystore создан владельцем и в репозитории отсутствует. Сборка release APK —
  локально, через переменные окружения `UNISONO_KEYSTORE`, `UNISONO_STORE_PASSWORD`,
  `UNISONO_KEY_ALIAS`, `UNISONO_KEY_PASSWORD`. Не запрашивать ключ и пароли.
- Потеря ключа = невозможность обновить приложение поверх; ключ не пересоздавать.

## 9. Определение готовности 0.1.0

- все MUST из `02_requirements.md` реализованы;
- сборки запускаются на macOS 12+ (arm64/x64), Windows 10/11, Ubuntu 24.04, Android 8+;
- тюнер настраивает гитару, скрипку, виолончель, бас-5 и балалайку на реальных инструментах;
- хроматический режим не показывает струны;
- RU/EN переключается, числа локализованы;
- README и «Первый запуск» соответствуют фактическим сборкам;
- `make -C docs html` и `cargo test` проходят на чистом клоне.

## 10. Открытые вопросы (не блокируют код)

- **Q13** — юридическая проверка «Unisono» (классы 9/15/41) до публичного релиза.
- **Q15** — базовая версия Linux: Ubuntu 24.04+/Fedora 40+, без Snap/Flatpak.
- **Q6/Q8** — состав 1.1 и сроки iOS, после 0.1.0.

Вопросы по спецификации — в issue репозитория; решения фиксировать в `docs/source/11_adr.md`.
