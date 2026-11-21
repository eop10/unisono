//! tuner-audio: захват микрофона и передача `f32`-кадров анализатору.
//!
//! Trait `AudioSource`; бэкенды `cpal` (macOS/Windows/Linux) и `oboe` (Android)
//! подключаются на этапе 4–5. Real-time колбэк без аллокаций и блокировок:
//! только `rtrb::Producer` (docs/source/03_architecture.md, 3.6).
