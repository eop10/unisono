(sec-build)=

# 8. Сборка, тестирование и выпуск

## 8.1. Cargo workspace

Один репозиторий, четыре crate'а и Tauri-приложение. Версии зависимостей — точные, `Cargo.lock`
коммитится; обновление — отдельной задачей.

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = ["crates/tuner-core", "crates/tuner-audio", "crates/tuner-storage", "src-tauri"]

[workspace.package]
edition = "2021"
rust-version = "1.80"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tracing = "0.1"
rtrb = "0.3"
```

```toml
# crates/tuner-core/Cargo.toml
[package]
name = "tuner-core"
edition.workspace = true

[dependencies]
serde = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
hound = "3"
criterion = "0.5"
proptest = "1"

[[bench]]
name = "detect"
harness = false
```

Release-профиль (важен для DSP и размера):

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
opt-level = 3
```

`tuner-core` не зависит от `tauri`, `cpal` и `oboe` — поэтому тесты DSP гоняются на любой машине,
включая CI без Android SDK.

## 8.2. Локальные команды

| Задача | Команда |
| --- | --- |
| Desktop dev (Tauri + Vite) | `npm run tauri dev` |
| Android dev на устройстве | `npm run tauri android dev` |
| Тесты ядра | `cargo test -p tuner-core` |
| Все тесты | `cargo test --workspace` |
| Линт | `cargo clippy --workspace --all-targets -- -D warnings` |
| Форматирование | `cargo fmt --all` |
| Бенчмарки DSP | `cargo bench -p tuner-core` |
| DMG (aarch64) | `npm run tauri build -- --target aarch64-apple-darwin` |
| DMG (x86_64) | `npm run tauri build -- --target x86_64-apple-darwin` |
| Windows-установщик | `npm run tauri build` на Windows (NSIS) |
| Linux AppImage / deb | `npm run tauri build` на Linux |
| Ad-hoc подпись | `codesign --force --deep --sign - Unisono.app` |
| Свой Android-ключ | `keytool -genkeypair -v -keystore unisono-release.jks -alias unisono ...` |
| Проверка APK | `apksigner verify --print-certs app-release.apk` |
| APK / AAB | `npm run tauri android build -- --apk` / `--aab` |
| UI отдельно | `npm --prefix ui run dev` |
| Диаграммы | `./tools/render-diagrams.sh` |
| Документация | `make -C docs-guitar-tuner html` |

## 8.3. Автотесты

### 8.3.1. Генерация эталонов

`tools/gen_fixtures.py` (Python 3 + `numpy`) создаёт WAV 48 кГц/16 бит моно и манифест.
Seed фиксирован: тесты должны быть воспроизводимы.

Манифест `fixtures/manifest.csv`:

```text
file,f0_hz,midi,type,snr_db,notes
sine_082_41.wav,82.41,40,sine,,чистая синусоида E2
ks_082_41_a.wav,82.41,40,karplus,,щипок, реализация A
harm_440_00.wav,440.00,69,harmonic,,6 гармоник, спад 1/n
drive_082_41.wav,82.41,40,overdrive,,tanh(3x) от гармонического ряда
noise_440_10db.wav,440.00,69,harmonic,10,шум 10 дБ
speech_01.wav,,,,,негативный тест, 10 с
silence.wav,,,,,тишина
```

Требования к генератору: калибровка по RMS (−20 dBFS), 100 мс тишины в начале (проверка
`Silent → Tracking`), устойчивая часть ≥ 1,5 с, три реализации Karplus–Strong, ресемплинг
в 44,1 кГц выполняется в тесте, а не в файлах.

Чтение WAV — `hound`:

```rust
struct Fixture {
    samples: Vec<f32>,
    sample_rate: u32,
    f0_hz: f64,
}

impl Fixture {
    fn load(name: &str) -> Self {
        let path = Path::new("tests/fixtures").join(name);
        let mut reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        let samples = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect::<Vec<_>>();
        let meta = manifest::lookup(name); // f0 из manifest.csv
        Self { samples, sample_rate: spec.sample_rate, f0_hz: meta.f0_hz }
    }
}
```

### 8.3.2. Тест точности

```rust
#[test]
fn karplus_strong_e2_accuracy() {
    let fixture = Fixture::load("ks_082_41_a.wav");
    let mut det = PitchDetector::new(DetectorConfig::new(fixture.sample_rate, 4096));
    let mut errors = Vec::new();

    for (i, frame) in fixture.samples.chunks(1024).enumerate() {
        if i < 5 || frame.len() < 4096 {
            continue; // пропуск атаки и неполного кадра
        }
        let mut buf = frame[..4096].to_vec();
        if let Some(Estimate::Pitch { f0, .. }) = det.process_frame(&mut buf, fixture.sample_rate) {
            errors.push(cents(f0 as f64, fixture.f0_hz));
        }
    }

    assert!(!errors.is_empty());
    let max = errors.iter().fold(0.0_f64, |acc, c| acc.max(c.abs()));
    assert!(max < 1.0, "max error: {max}");
    let octaves = errors.iter().filter(|c| (c.abs() - 1200.0).abs() < 50.0).count();
    assert_eq!(octaves, 0, "octave errors: {octaves}");
}
```

Набор обязательных проверок:

| Тест | Что проверяет | Порог |
| --- | --- | --- |
| `accuracy_sine_*` | базовое разрешение | ≤ 0,5 цента |
| `accuracy_karplus_*` | реальный тембр | ≤ 1 цент, 0 октав |
| `accuracy_overdrive_*` | клиппинг | ≤ 2 цента, 0 октав |
| `accuracy_noise_*` | устойчивость | ≤ 2 цента при 10 дБ |
| `negative_speech_*` | ложные ноты | 0 кадров с `clarity ≥ 0,60` |
| `silence_gate` | гейт | состояние `Silent` |
| `note_mapping_*` | MIDI, октавы, нотация | точное совпадение |
| `reference_442` | пересчёт от A4 | A2 = 110,50 Гц ± 0,01 |
| `smoothing_step` | реакция на скачок | выход на 90 % за ≤ 5 кадров |
| `preset_validation` | границы и лимиты | по таблице 5.7 |
| `low_range_*` | B0/E1/C2, окно 16384 | ≤ 3 цента, 0 октав |
| `unison_*` | балалайка E4+E4 | пара подсвечена, нет дребезга |
| `chromatic_*` | A4 440/442/466 | ближайшая нота и центы |
| `rt_callback_no_alloc` | колбэк не аллоцирует | `dhat` или счётчик аллокаций в обёртке |
| `storage_v2_to_v3` | миграция схемы хранилища (#12) | чтение v2 → v3 с `input_device: None`, сохранение v3 |
| `input_device_selection` | выбор входа CPAL и строка UI (#12) | список устройств, тихий фолбэк на дефолт, скрытие на Android |
| `decay_hold_*` | удержание показания при затухании (#13) | `Uncertain` в течение `hold_ms` (1800 мс) → `Silent`, мгновенный `Tracking` |

Плюс `proptest` на `NoteMath`: для любых MIDI 21–96 и A4 400–480 частота положительна,
монотонна по MIDI и `cents(f, f_target) = 0` при точном совпадении.

### 8.3.3. Бенчмарки

- `criterion` (`cargo bench -p tuner-core`): `detect` на окнах 4096/8192, цель ≤ 3 мс/кадр
  на прогретом коде; сравнение с базовой линией в отчёте.
- На устройстве: `cargo run --release --bin bench_mic` (небольшой бинарник в `src-tauri`),
  замер p95 через `tracing`.
- Аллокации: `dhat` в тестовом профиле; после 1000 кадров прирост кучи не должен превышать 1 МБ.

### 8.3.4. Что не покрывается автотестами

Разрешения, реальный микрофон, WebView-рендеринг на конкретных устройствах, поведение при
сворачивании, haptics, нативный вид окна — только ручной чек-лист 8.4. Для UI используются
Svelte-тесты на логику стора (`vitest`), но не на отрисовку.

## 8.4. Ручной чек-лист приёмки

| № | Проверка | Ожидаемый результат |
| --- | --- | --- |
| 1 | Первый запуск Android, отказ в разрешении | объяснение, кнопка «Открыть настройки», без краша |
| 2 | Выдача разрешения после возврата из настроек | тюнер запускается автоматически |
| 3 | Акустическая гитара, 6 струн, 440 Гц | все струны настраиваются, «в строе» при ±3 центах |
| 3a | Переключение инструментов: скрипка, виолончель, бас-5, балалайка, домра, мандолина, банджо | список строёв и чипы струн меняются, целевые частоты верные |
| 3b | Хроматический режим | струны скрыты, ближайшая нота и центы корректны |
| 3c | Балалайка E4+E4 | подсвечивается пара, переключение на A4 без дребезга |
| 4 | Электрогитара с перегрузом | нет октавных ошибок на 6-й струне |
| 5 | Опорная 442 Гц | A2 показывает 110,5 Гц, индикатор 0 на настроенной струне |
| 6 | Пользовательский строй DADGAD | сохранение, перезапуск, корректные целевые частоты |
| 7 | Drop C | окно 8192, задержка ≤ 200 мс, точность ±2 цента |
| 8 | Тихая комната, разговор рядом | нет ложных нот, состояние «Слушаю…» |
| 9 | Сворачивание Android | микрофон освобождён, индикатор не работает |
| 10 | Смена устройства ввода на macOS | переоткрытие потока, восстановление ≤ 2 с |
| 11 | Тон 440 Гц | без щелчков, захват на паузе |
| 12 | Тёмная/светлая тема, крупный шрифт | layout не ломается, контраст сохранён |
| 13 | TalkBack/VoiceOver в WebView | элементы озвучены, порядок логичный |
| 14 | macOS 12 Intel и macOS 14 Apple Silicon | окно, меню, микрофон, DMG открывается после «Всё равно открыть» |
| 14a | Windows 10 и Windows 11 | установщик, SmartScreen → «Выполнить всё равно», микрофон, меню |
| 14b | Ubuntu 24.04 (AppImage и deb) | запуск, микрофон через PipeWire/ALSA, окно WebKitGTK |
| 15 | Android 8 и Android 14 | установка, разрешения, микрофон, вибрация |
| 16 | 10 минут непрерывной работы | расход батареи ≤ 3 %, нет нагрева выше 40 °C, нет щелчков |

## 8.5. CI

```yaml
name: ci
on:
  push: { branches: [main] }
  pull_request:

jobs:
  core:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "clippy, rustfmt" }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace

  ui:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: npm, cache-dependency-path: ui/package-lock.json }
      - run: npm ci --prefix ui
      - run: npm --prefix ui run check
      - run: npm --prefix ui run build

  desktop:
    runs-on: macos-15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - run: npm ci --prefix ui && npm ci
      - run: npm run tauri build
      - uses: actions/upload-artifact@v4
        with: { name: dmg, path: target/release/bundle/dmg/*.dmg }

  android:
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: "aarch64-linux-android,armv7-linux-androideabi" }
      - uses: actions/setup-java@v4
        with: { distribution: temurin, java-version: '17' }
      - uses: android-actions/setup-android@v3
      - run: sdkmanager "ndk;26.1.10909125"
      - run: npm ci --prefix ui && npm ci
      - run: npm run tauri android build -- --apk
        env:
          NDK_HOME: ${{ env.ANDROID_HOME }}/ndk/26.1.10909125
      - uses: actions/upload-artifact@v4
        with: { name: apk, path: src-tauri/gen/android/app/build/outputs/apk/**/*.apk }

  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - run: npm ci --prefix ui && npm ci
      - run: npm run tauri build
      - uses: actions/upload-artifact@v4
        with: { name: windows-installer, path: target/release/bundle/nsis/*.exe }

  linux:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libasound2-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - run: npm ci --prefix ui && npm ci
      - run: npm run tauri build
      - uses: actions/upload-artifact@v4
        with: { name: linux-packages, path: "target/release/bundle/{appimage,deb}/*" }

  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pip install -r docs-guitar-tuner/requirements.txt
      - run: make -C docs-guitar-tuner html
```

Правила:

- CI собирает **неподписанные** артефакты: per-arch DMG (aarch64, x86_64) без notarization и debug APK.
  Release-подпись Android (свой keystore) и ad-hoc подпись macOS выполняются локально
  на релизном шаге — секретов Apple/Google в CI нет.
- Публикация — GitHub Releases: DMG, APK, SHA-256, раздел «Первый запуск».
- `cargo clippy -D warnings` — обязательное условие мержа: в аудио-коде предупреждения
  про `unsafe` и лишние аллокации слишком дорого игнорировать.
- Кэш `Swatinem/rust-cache` обязателен: сборка Tauri с нуля — 8–12 минут.

## 8.5.1. Ассеты и иконки

- `brand/src/unisono-mark.svg` — мастер; `brand/icon.icns` и `brand/android/mipmap-*` —
  производные, пересобираются скриптом, а не руками.
- В репозитории лежат `brand/png/icon-1024…16.png`, `brand/icon.ico`, `brand/readme-header.png`, `brand/sizes.png`.
- CI проверяет, что `icon.icns` существует и что в `tauri.conf.json` указан `icons/icon.icns`.
- `src-tauri/gen/android` содержит `mipmap-anydpi-v26/ic_launcher.xml` с foreground/background/monochrome.

## 8.6. Релизный чек-лист

**Android (GitHub Releases, без Google Play)**

1. `versionCode`/`versionName` подняты в `src-tauri/gen/android/app/build.gradle.kts` и `tauri.conf.json`.
2. Release APK собран и подписан **своим** keystore; `apksigner verify` проходит.
3. Keystore и пароль сохранены в менеджере секретов и в бэкапе (потеря = нет обновлений).
4. Пройден чек-лист 8.4 на Android 8 и Android 14 (минимум два устройства), установка — sideload.
5. APK + SHA-256 загружены в GitHub Releases; в описании — «разрешите установку из неизвестных источников».
6. Нет разрешения `INTERNET`, нет телеметрии; политика приватности — в README.

**macOS (GitHub Releases, без Apple Developer Program)**

1. `version` = `X.Y.Z` в `tauri.conf.json`.
2. `plutil -p "…/Unisono.app/Contents/Info.plist"` содержит `NSMicrophoneUsageDescription`.
3. Собраны per-arch DMG (`aarch64` и `x86_64`), `.app` подписан ad-hoc.
4. Notarization **не выполняется**; `spctl` ожидаемо вернёт `rejected` — это нормально для unsigned.
5. В README есть блок «Первый запуск» со шагами для macOS 14 и 15+.
6. Установка с нуля на Apple Silicon (`Unisono_*_aarch64.dmg`) и Intel
   (`Unisono_*_x86_64.dmg`): DMG открывается, приложение запускается после
   «Всё равно открыть», микрофон работает.
7. DMG + SHA-256 в GitHub Releases; в «О программе» указано `unsigned build`.

**Windows (GitHub Releases, без подписи)**

1. `version` в `tauri.conf.json`; собран NSIS-установщик.
2. Установка на Windows 10 и 11: SmartScreen → «Подробнее» → «Выполнить всё равно».
3. Микрофон разрешён в параметрах приватности; настройка гитары и скрипки работает.
4. SmartScreen не блокирует навсегда (пользователь может запустить), в README есть шаги.
5. `.exe` + SHA-256 в GitHub Releases.

**Linux (GitHub Releases, без подписи)**

1. AppImage собран на Ubuntu 24.04, `.deb` — для Debian/Ubuntu.
2. Запуск на Ubuntu 24.04 и Fedora 40: WebKitGTK окно, ALSA/PipeWire микрофон.
3. AppImage исполняемый (`chmod +x`), `.deb` ставится через `apt install ./Unisono_*.deb`.
4. В README — про NVIDIA и `WEBKIT_DISABLE_DMABUF_RENDERER=1`.
5. AppImage/`.deb` + SHA-256 в GitHub Releases.

## 8.7. Документация

Проект документации — этот Sphinx-проект:

```bash
cd docs-guitar-tuner
make html     # build/html/index.html, сборка с -W
make pdf      # dist/documentation.pdf
make bundle   # zip со статикой для отправки
```

Диаграммы пересобираются `tools/render-diagrams.sh` (Mermaid CLI, Chrome в качестве браузера).
Исходники `.mmd` — источник истины, PNG коммитятся, чтобы сборка документации не требовала Node.js.
