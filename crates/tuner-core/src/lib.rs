//! tuner-core: DSP (NSDF/MPM) и теория музыки (ноты, инструменты, строи, движок).
//!
//! Границы (docs/source/03_architecture.md): крейт не знает про аудио, Tauri,
//! файлы и платформы. `unsafe` запрещён.

#![forbid(unsafe_code)]

pub mod dsp;
pub mod engine;
pub mod music;
