(sec-platform)=

# 7. Платформенная интеграция

Общий код заканчивается на trait `AudioSource`. Всё, что ниже, зависит от ОС: бэкенд аудио,
разрешения, жизненный цикл, упаковка.

```rust
// crates/tuner-audio/src/lib.rs
pub trait AudioSource: Send {
    /// Фактическая частота дискретизации, установленная устройством.
    fn sample_rate(&self) -> u32;
    /// Запускает поток; кадры f32 уходят в producer без аллокаций.
    fn start(&mut self, sink: rtrb::Producer<f32>) -> Result<(), AudioError>;
    fn stop(&mut self);
}

#[cfg(target_os = "macos")]
pub use cpal_source::CpalSource as PlatformSource;

#[cfg(target_os = "android")]
pub use oboe_source::OboeSource as PlatformSource;
```

Ключевое требование к обеим реализациям: **колбэк не аллоцирует и не блокирует** (3.6).
`Vec`, `Mutex`, `println!`, `String` внутри колбэка — запрещены.

## 7.1. Android

### 7.1.1. Проект и манифест

Android-часть создаётся командой `tauri android init`: появляется `src-tauri/gen/android` (Gradle-проект,
`MainActivity`, манифест). Дальше правки идут в нём и коммитятся вместе с репозиторием.

```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.VIBRATE" />
<uses-feature android:name="android.hardware.microphone" android:required="true" />
```

- `minSdk 26`, `targetSdk 35`: Tauri 2 требует минимум 24, но `UNPROCESSED`/AAudio-поведение
  стабильно с 26.
- `android:allowBackup="false"`: настройки не критичны, бэкап микрофонного приложения не нужен.
- Разрешение `INTERNET` не добавляется: это техническая гарантия ФТ-12/НФТ-13.
  Важно: без него не работает dev-сервер Tauri — проверять офлайн-режим нужно на release-сборке,
  где `devUrl` отсутствует.

### 7.1.2. Захват через oboe

```rust
use oboe::{AudioInputStream, AudioStreamBuilder, DataCallbackResult, InputPreset, AudioFormat};

struct Capture {
    producer: rtrb::Producer<f32>,
    overruns: std::sync::atomic::AtomicU32,
}

impl oboe::AudioInputCallback for Capture {
    type FrameType = f32;

    fn on_audio_ready(
        &mut self,
        _stream: &mut AudioInputStream<f32>,
        data: &[f32],
    ) -> DataCallbackResult {
        for &sample in data {
            if self.producer.push(sample).is_err() {
                self.overruns.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        DataCallbackResult::Continue
    }
}

pub fn open(sink: rtrb::Producer<f32>) -> Result<AudioStream, oboe::Error> {
    let mut builder = AudioStreamBuilder::default();
    builder
        .set_format(AudioFormat::Float)
        .set_channel_count(1)
        .set_sample_rate(48_000)
        .set_input_preset(InputPreset::VoiceRecognition) // без AGC/NS
        .set_callback(Capture { producer: sink, overruns: AtomicU32::new(0) });
    builder.build_input_stream()
}
```

Практические детали:

| Деталь | Почему |
| --- | --- |
| `InputPreset::VoiceRecognition` | отключает AGC/шумоподавление; `Unprocessed` доступен не на всех прошивках |
| `AudioFormat::Float` | без потери точности, сразу в `f32` для DSP |
| `set_sample_rate(48_000)` | для тюнера достаточно; фактическая частота читается из потока |
| `rtrb::Producer::push` с игнорированием ошибки | переполнение буфера не должно ронять аудио-поток, только счётчик |
| `request_start()` вне UI-потока | открытие потока может занять десятки мс |

### 7.1.3. Разрешения: без Kotlin не обойтись

Это единственное место, где Rust не заменяет платформенный код: `RECORD_AUDIO` — опасное
разрешение, его запрашивает Activity. В `src-tauri/gen/android` добавляется маленький плагин Tauri:

```kotlin
@TauriPlugin
class MicPlugin(private val activity: Activity) : Plugin(activity) {
    private var pending: Invoke? = null

    @Command
    fun requestMic(invoke: Invoke) {
        if (ContextCompat.checkSelfPermission(activity, Manifest.permission.RECORD_AUDIO)
                == PackageManager.PERMISSION_GRANTED) {
            invoke.resolve(); return
        }
        pending = invoke
        activity.requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), REQ_MIC)
    }

    @Command
    fun keepScreenOn(invoke: Invoke) {
        activity.runOnUiThread {
            activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
        invoke.resolve()
    }

    @Command
    fun vibrate(invoke: Invoke) {
        val v = activity.getSystemService(Vibrator::class.java)
        v.vibrate(VibrationEffect.createOneShot(20, VibrationEffect.DEFAULT_AMPLITUDE))
        invoke.resolve()
    }
}
```

Плагин регистрируется в `MainActivity` (`registerPlugin(MicPlugin::class.java)`), а из Rust
вызывается через `PluginHandle::run_mobile_plugin("requestMic", ())`. Точные имена методов
сверяются с версией Tauri при обновлении — это одна из причин, почему версия фиксируется.

Порядок запуска: `requestMic` → пользователь выдаёт разрешение → `start_listening` →
`oboe` открывает поток. Если разрешение отозвано, `build_input_stream` вернёт ошибку,
и UI получит состояние `NoPermission`, а не пустой экран.

### 7.1.4. Жизненный цикл

```text
onStart   -> requestMic (если нужно) -> start_listening -> FLAG_KEEP_SCREEN_ON
onPause   -> stop_listening -> clear FLAG_KEEP_SCREEN_ON
onDestroy -> drop AudioStream
```

- Фоновое прослушивание в MVP не поддерживается: нет foreground service, нет уведомления.
  Тюнер нужен на экране, а микрофон в фоне — лишний расход и вопросы приватности.
- Кнопка «назад» на Android: из редактора строя возвращает к списку, с главного экрана —
  сворачивает приложение (обрабатывается в Svelte-роутере и `tauri-plugin-*`/Kotlin).
- Смена ориентации переживается без перезапуска аудио: активность не пересоздаётся
  (`configChanges` в манифесте).

### 7.1.5. Сборка

| Задача | Команда |
| --- | --- |
| Инициализация | `npm run tauri android init` |
| Dev на устройстве | `npm run tauri android dev` |
| Release APK (свой ключ) | `npm run tauri android build -- --apk` |
| AAB для Play (позже) | `npm run tauri android build -- --aab` |

**Подпись.** Android не устанавливает APK без подписи — это ограничение платформы, а не магазина.
Мы подписываем **собственным ключом** (self-signed), а не ключом Google Play:

```bash
keytool -genkeypair -v -keystore unisono-release.jks -alias unisono \
  -keyalg RSA -keysize 2048 -validity 10000
```

Ключ и пароль хранятся в менеджере секретов и в бэкапе: потеря ключа означает, что обновление
с тем же `applicationId` установить поверх нельзя. Настройка — в `src-tauri/gen/android/app/build.gradle.kts`
(`signingConfigs.release`); версия — `versionCode`/`versionName`.

Требуется Android SDK, NDK (`ANDROID_NDK_HOME`), Rust-таргеты `aarch64-linux-android`,
`armv7-linux-androideabi` (и `x86_64-linux-android` для эмулятора). Release-сборка:
`strip`, `lto = true`, `codegen-units = 1`, `panic = "abort"` — бинарник ~5–7 МБ.

## 7.2. macOS

### 7.2.1. Захват через cpal

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn open(sink: rtrb::Producer<f32>) -> Result<(cpal::Stream, u32), AudioError> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or(AudioError::NoInputDevice)?;

    let supported = device.default_input_config()?;   // обычно 44,1 или 48 кГц
    let sample_rate = supported.sample_rate().0;
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Fixed(512),
    };

    let mut producer = sink;
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _| {
            for &s in data {
                if producer.push(s).is_err() { /* переполнение: счётчик, не паника */ }
            }
        },
        |err| tracing::error!(?err, "audio stream error"),
        None,
    )?;
    stream.play()?;
    Ok((stream, sample_rate))
}
```

Особенности:

- Частота берётся из конфигурации устройства (44,1 или 48 кГц), а не из константы: DSP
  инициализируется по `sample_rate` (см. 4.3).
- Первый `stream.play()` вызывает системный запрос доступа к микрофону (TCC). При запуске
  из `tauri dev` запрос атрибутируется терминалу/IDE — проверять выдачу разрешения нужно
  на собранном `.app`.
- Устройство по умолчанию меняется (наушники, док-станция): при ошибке потока или отсутствии
  данных дольше 1 с переоткрываем поток, состояние сбрасывается в `Silent`.
- `BufferSize::Fixed(512)` — запрос, а не гарантия: фактический размер узнаётся из колбэка.
  Для тюнера это не критично: анализ идёт окнами по 4096.

### 7.2.2. Info.plist, entitlements, конфигурация бандла

`src-tauri/tauri.conf.json`:

```json
{
  "productName": "Unisono",
  "identifier": "app.unisono.tuner",
  "build": {
    "frontendDist": "../ui/dist"
  },
  "bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns"],
    "macOS": {
      "minimumSystemVersion": "12.0",
      "entitlements": "entitlements.plist",
      "signingIdentity": "Developer ID Application: Tuner Dev (TEAMID)"
    }
  },
  "app": {
    "windows": [
      { "title": "Unisono", "width": 900, "height": 640, "minWidth": 720, "minHeight": 520 }
    ]
  }
}
```

| Что | Значение |
| --- | --- |
| `NSMicrophoneUsageDescription` | «Тюнер слушает струну, чтобы показать ноту. Звук не записывается и не покидает устройство.» |
| `src-tauri/Info.plist` | Tauri мержит его в итоговый `Contents/Info.plist`; ключ обязателен, иначе macOS убьёт приложение при первом обращении к микрофону |
| `entitlements.plist` | `com.apple.security.device.audio-input` — только если включён App Sandbox |
| Hardened Runtime | обязателен для notarization; для микрофона исключений не требует |
| App Sandbox | для notarization **не обязателен**; включать только под Mac App Store (**⚠️ Q4**) |

Если установленная версия Tauri не подхватывает `src-tauri/Info.plist`, ключ добавляется
в сгенерированный бандл до подписи — это детерминированный обходной путь:

```bash
/usr/libexec/PlistBuddy -c "Add :NSMicrophoneUsageDescription string 'Тюнер слушает струну. Звук не записывается.'" \
  "target/release/bundle/macos/Unisono.app/Contents/Info.plist"
```

### 7.2.3. Распространение без Apple Developer Program

Основной путь для 1.0: **Universal DMG без нотаризации**, раздача через GitHub Releases.
Apple Developer Program нет, поэтому нет Developer ID и notarization; это осознанное решение (D-14).

Сборка и ad-hoc подпись:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run tauri build -- --target universal-apple-darwin
codesign --force --deep --sign - "target/universal-apple-darwin/release/bundle/macos/Unisono.app"
```

`codesign -s -` — ad-hoc подпись: она не убирает предупреждение Gatekeeper, но убирает
«приложение повреждено» и проблемы с переносом. DMG после этого можно собрать
`hdiutil create` или взять из `bundle/dmg/`, если Tauri его уже создал.

Что видит пользователь и что пишем в README:

| macOS | Действие |
| --- | --- |
| 14 и старше | правый клик по `Unisono.app` → «Открыть» → «Открыть» |
| 15 Sequoia и новее | «Системные настройки → Конфиденциальность и безопасность → Всё равно открыть» |
| Продвинутый вариант | `xattr -d com.apple.quarantine /Applications/Unisono.app` |

Ограничения этого пути:

- Gatekeeper предупреждает при первом запуске; часть пользователей не установит;
- корпоративные Mac с MDM могут блокировать запуск полностью;
- нет Mac App Store и автоматической проверки Apple;
- SHA-256 рядом с DMG обязателен;
- в «О программе» пишем «unsigned build», чтобы не создавать ложного впечатления.

Если позже появится Apple Developer Program, к той же сборке добавляются Developer ID,
hardened runtime и notarization — bundle id и DMG не меняются. Порядок:

```bash
codesign --force --deep --options runtime --timestamp \
  --sign "Developer ID Application: ..." --entitlements desktopApp/entitlements.plist Unisono.app
xcrun notarytool submit Unisono-1.0.0.dmg --keychain-profile unisono-notary --wait
xcrun stapler staple Unisono-1.0.0.dmg
spctl -a -vvv -t install Unisono.app
```

### 7.2.4. Окно и меню

- Окно из `tauri.conf.json` (900×640, минимум 720×520), `decorations: true`,
  системные «светофоры» macOS.
- Нативное меню — через API Tauri (`Menu`, `Submenu`, `PredefinedMenuItem`): «О программе»,
  «Настройки…» (⌘,), «Выход» (⌘Q). Меню создаётся в Rust при старте.
- Закрытие окна завершает приложение, но сначала останавливает захват и сохраняет настройки.
- `activationPolicy: "regular"` — приложение с иконкой в Dock и собственным меню.

### 7.2.5. Иконки приложения

Источник — `brand/` (см. 6.8.4). В проект кладутся готовые файлы:

| Файл | Куда |
| --- | --- |
| `brand/icon.icns` | `src-tauri/icons/icon.icns`, прописан в `bundle.icon` |
| `brand/png/icon-32.png`, `icon-128.png`, `icon-256.png` | `src-tauri/icons/` |
| `brand/android/mipmap-*` | `src-tauri/gen/android/app/src/main/res/` |
| `brand/android/ic_launcher.xml`, `colors.xml` | `res/mipmap-anydpi-v26/` и `res/values/` |

Правила: иконка — из мастер-SVG, без ручной перерисовки; foreground adaptive занимает 82 % слоя,
чтобы `U` не обрезался круглой маской; monochrome-слой — белый силуэт без латуни.

## 7.2.6. Windows

### Аудио (WASAPI)

```rust
// тот же trait AudioSource, другой бэкенд cpal
let host = cpal::default_host();                  // WASAPI
let device = host.default_input_device()?;
let supported = device.default_input_config()?;   // 48 кГц, f32, моно/стерео
```

- WASAPI в **shared mode**: система может ресемплить поток, поэтому фактическая частота берётся
  из конфигурации устройства, а не из константы.
- Буфер 512 кадров, задержка 10–30 мс.
- Доступ к микрофону регулируется в «Параметры → Конфиденциальность → Микрофон»; при запрете
  `cpal` вернёт ошибку устройства — UI показывает `NoPermission`.
- Смена устройства по умолчанию (USB-гарнитура, док-станция) — та же логика перезапуска, что на macOS.

### Сборка и упаковка (Windows)

| Что | Значение |
| --- | --- |
| Тулчейн | Rust `x86_64-pc-windows-msvc`, Visual Studio Build Tools, WebView2 |
| WebView | WebView2 Runtime (есть в Windows 10/11; для офлайна — `webviewInstallMode`) |
| Установщик | NSIS `.exe` (основной), MSI — опционально |
| Иконка | `brand/icon.ico` (16–256) |
| Подпись | нет; SmartScreen покажет «Windows protected your PC» → «Подробнее» → «Выполнить всё равно» |
| Данные | `%APPDATA%\app.unisono.tuner\` |

Команда: `npm run tauri build` на Windows-раннере; артефакт — `bundle/nsis/Unisono_<version>_x64-setup.exe`.
Authenticode-сертификат можно добавить позже, код и bundle id не меняются.

## 7.2.7. Linux

### Зависимости (Linux)

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev
```

### Аудио (ALSA/PipeWire)

- `cpal` работает через **ALSA**; PipeWire и PulseAudio подхватываются через `pipewire-alsa`
  или `pulseaudio-alsa`.
- Устройство по умолчанию — `default`; при отсутствии явного device list используется системный.
- Частота и формат — из устройства (обычно 48 кГц, `f32` после конвертации).
- Задержка 10–40 мс зависит от периода ALSA; на слабых USB-микрофонах возможны пропуски —
  счётчик переполнений `rtrb` помогает диагностировать.
- Известная проблема WebKitGTK на NVIDIA: чёрный экран или артефакты. Обход —
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` в переменных окружения.

### Сборка и упаковка (Linux)

| Что | Значение |
| --- | --- |
| Базовая система | Ubuntu 24.04 (WebKitGTK 4.1), проверка на Fedora 40 |
| Форматы | AppImage (основной), `.deb` (Ubuntu/Debian) |
| Иконка | PNG 256/512 из `brand/png/` |
| Подпись | нет; AppImage запускается после `chmod +x` |
| Данные | `~/.config/app.unisono.tuner/` |

AppImage собирается на самой старой поддерживаемой базе, чтобы не требовать свежий glibc;
`.deb` — для удобной установки в Ubuntu/Debian. Snap/Flatpak в MVP не входят.

## 7.3. Сравнение способов захвата

| Способ | Плюсы | Минусы | Статус |
| --- | --- | --- | --- |
| `cpal` на macOS | один trait на все desktop-ОС, CoreAudio из коробки, f32 | задержка 10–40 мс, поведение при смене устройства нужно обрабатывать | **выбран** |
| `oboe` на Android | официальные биндинги AAudio, низкая задержка, `InputPreset` | ошибки привязываются к версии NDK/прошивки | **выбран** |
| `cpal` на Android | единый код с desktop | мобильный бэкенд менее предсказуем, чем прямой oboe | не используем |
| `AVAudioEngine` через `objc2` | максимум контроля на macOS | много unsafe/FFI, выигрыш для тюнера не нужен | резерв (R-2) |
| Web Audio в WebView | нет нативного кода | браузерная обработка и задержка, нет `f32`-потока | не подходит |

## 7.4. Последовательность запуска и цикла настройки

```{figure} diagrams/sequence.png
:alt: Последовательность: запуск, разрешение, поток кадров через ring buffer, вход в строй, уход в фон
:name: fig-seq
:width: 1000px

Рис. 5. Запуск и один цикл настройки. Исходник: `diagrams/src/sequence.mmd`.
```

Ключевые точки, которые легко сломать при реализации:

- `sample_rate` приходит из платформы **до** первого кадра; DSP инициализируется по нему,
  а не по константе.
- Пресет и A4 читаются один раз при старте; их изменение не перезапускает захват — движок
  пересчитывает целевые частоты.
- При уходе в фон захват закрывается, а не «замирает»: на Android открытый `AudioRecord`/AAudio
  в фоне быстро приводит к отзыву разрешения системой.
