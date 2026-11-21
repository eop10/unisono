//! Теория музыки: ноты, инструменты, строи, реестр (docs/source/05_data.md).
//!
//! Не знает про аудио, Tauri, файлы и платформы; `unsafe` запрещён на уровне крейта.

pub mod instruments;
pub mod note;
pub mod registry;
pub mod tunings;

pub use instruments::{instrument, Instrument, InstrumentKind, Localized, INSTRUMENTS};
pub use note::{cents, hz, latin_name, nearest_midi, solfeggio_name, A4_MIDI};
pub use registry::InstrumentRegistry;
pub use tunings::{LocalizedName, Tuning};
