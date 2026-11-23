//! Диагностика TCC-статуса микрофона (7.2): печатает текущий статус
//! этого процесса **без запроса** (`mic_status` не поднимает диалог).
//! Запуск: `cargo run -p tuner-audio --example tcc_status`.
//!
//! Для приёмки (Prompt 4B, часть 3.4): сравните вывод до и после
//! переключения Unisono в «Системные настройки → Конфиденциальность →
//! Микрофон».

use tuner_audio::tcc::{mic_status, MicPermission};

fn main() {
    match mic_status() {
        MicPermission::Authorized => println!("микрофон: разрешено"),
        MicPermission::NotDetermined => println!("микрофон: ещё не спрошено"),
        MicPermission::Denied => println!("микрофон: отказано или ограничено"),
    }
}
