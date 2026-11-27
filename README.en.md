<p align="center">
  <img src="brand/readme-header.png" alt="Unisono" width="560">
</p>

<h1 align="center">Unisono</h1>

<p align="center">
  Offline multi-instrument string &amp; chromatic tuner with custom tunings and adjustable A4.<br>
  macOS · Android · Windows · Linux
</p>

<p align="center">
  <a href="https://github.com/eop10/unisono/releases/tag/v0.2.0"><img src="https://img.shields.io/github/v/release/eop10/unisono?style=flat-square&color=C8A253" alt="Release"></a>
  <a href="https://github.com/eop10/unisono/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/eop10/unisono/ci.yml?branch=main&style=flat-square" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Android%20%7C%20Windows%20%7C%20Linux-lightgrey?style=flat-square" alt="Platforms">
</p>

<p align="center">
  <a href="README.md">Русский</a> · <strong>English</strong>
</p>

> **Status:** **`0.2.0`** is released for **macOS**, **Windows**, **Linux**, and **Android** (see [GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0)).

## Download v0.2.0

| Platform | Format | Notes |
| --- | --- | --- |
| macOS (Apple Silicon / Intel) | `.dmg` (`aarch64`, `x86_64`) | [Download from GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Windows 10/11 x64 | NSIS `.exe` | [Download from GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Linux x86_64 | `.deb`, `.rpm`, `.AppImage`, `.pkg.tar.zst` | [Download from GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Android 8.0+ | `.apk` (arm64-v8a / armeabi-v7a) | [Download from GitHub Releases](https://github.com/eop10/unisono/releases/tag/v0.2.0) |
| Checksums | `SHA256SUMS.txt` | Included in `v0.2.0` release assets |

## What it is

A precision tuner for stringed instruments: Guitar (6/7), Bass (4/5), Ukulele, Violin, Viola, Cello, Double Bass, Balalaika, Domra (3/4), Mandolin, Banjo (5-string and Tenor), plus a stringless Chromatic mode.

## Key Features

- **Custom Tunings Builder & Export/Import:** Create, edit, export, and import your own tunings in JSON (e.g., `DADGAD`, `Drop C`) for any instrument — stored locally across restarts.
- **Adjustable Reference Pitch A4 (400–480 Hz, 0.1 Hz step):** Curated preset list, smooth slider, direct keyboard input, capture A4 from microphone, and instant `↑`/`↓` nudging.
- **Reference Tone, Capo & Temperament:** Pure sine tone generator with automatic mic pause, capo transposition (`−6…+12` semitones), auto-advance to next string, and temperament modes (`Equal`, `Sweetened`, `Just Fifths`).
- **12th-Fret Intonation Check & Stability Sparkline:** Compare open string vs. 12th fret for saddle adjustment hints, strobe gauge mode, and a live cents stability sparkline.
- **Smart String & Unison Detection:** Hysteresis-backed auto-detection (including highlighting unison string pairs such as `E4+E4` on Balalaika) or manual string lock via tap / `1..8` keys.
- **Decay Hold:** After a pluck, the dimmed note, needle, and cents remain held (~2.5 s) so you can read the result comfortably, while a new attack locks in immediately.
- **Precision DSP & Instrument-Grade UI:** ±50/±5 cent vernier scale with brass zero mark, adaptive analysis window (4096 / 8192 / 16384 samples) for reliable tracking down to bass `B0`, configurable tolerance (`±2/3/5` cents), and sensitivity levels.
- **In-Tune Feedback:** Soft acoustic chime (macOS/Android) and haptic vibration (Android) when hitting the target pitch.
- **External Audio Interfaces & Shortcuts (macOS):** Select a specific microphone or USB audio interface (with automatic multichannel-to-mono downmix), global `Space` to start/stop, `1..8` for strings, and `↑`/`↓` for A4.
- **Accessibility & Privacy:** Light, Dark, and System themes, Large Type mode, screen reader support (VoiceOver / TalkBack), `prefers-reduced-motion`, English and Russian UI (`C-D-E` / `Do-Re-Mi`). **100% offline:** zero ads, accounts, network requests, or telemetry.

## Structure

```text
unisono/
├── crates/                    tuner-core, tuner-audio, tuner-storage
├── src-tauri/                 Tauri 2 shell
├── ui/                        Svelte 5 + TypeScript
├── src-tauri/gen/android/     Gradle project & Kotlin shim
├── brand/                     logo, icons (.icns, .ico, Android adaptive, social preview)
├── docs/                      Sphinx documentation (requirements, architecture, DSP, UI, plan)
└── tools/                     test WAV generator
```

## Build from Source

Requires Rust, Node.js 22+, JDK 17, and (for Android) Android SDK/NDK.

```bash
npm install
npm run tauri dev            # desktop
npm run tauri build          # current OS
npm run tauri android build  # Android APK
```

## First Launch (Unsigned / Sideload Builds)

**macOS.** Open the DMG, drag `Unisono.app` to Applications, then:
- macOS 14 and earlier: right-click `Unisono.app` → "Open" → "Open";
- macOS 15+: *System Settings → Privacy & Security → Open Anyway*.

**Windows.** Run the NSIS installer → "More info" → "Run anyway".

**Linux.**
- Ubuntu / Debian: `sudo apt install ./Unisono_*.deb`
- Fedora / openSUSE: `sudo dnf install ./Unisono-*.rpm`
- Arch / Manjaro: `sudo pacman -U ./unisono-bin-*.pkg.tar.zst` (or [`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD))
- Universal AppImage: `chmod +x Unisono-*.AppImage && ./Unisono-*.AppImage`
- If WebKitGTK shows a blank window on NVIDIA + Wayland:
  `WEBKIT_DISABLE_DMABUF_RENDERER=1 unisono`

**Android (sideload).**
- Requires Android 8.0+ (API 26+), `arm64-v8a` or `armeabi-v7a`.
- Allow installing unknown apps for your browser or file manager when prompted; tap "Install anyway" if Play Protect warns about an unknown publisher.

## License

Code is licensed under the [Apache License 2.0](LICENSE). The **Unisono** name and brand assets (`brand/`) are proprietary and not covered by Apache 2.0.

## Privacy & Contact

Unisono never records audio or sends data over the network: see [PRIVACY.md](PRIVACY.md).
Questions & security: **egorprisiazhniy@gmail.com**.
