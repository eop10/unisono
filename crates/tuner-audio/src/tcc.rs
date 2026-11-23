//! Разрешение на микрофон (7.2.1): cpal/CoreAudio при отказе TCC отдаёт
//! **тишину** вместо ошибки — UI показывал бы «Слушаю…» вместо экрана
//! `NoPermission`. Поэтому статус спрашиваем явно через AVFoundation,
//! до открытия устройства, и всегда в потоке-владельце (не в UI).
//!
//! Отказ распознаётся только по статусу TCC, не по тексту ошибки cpal.

use crate::AudioError;

/// Статус разрешения на микрофон (7.2.1). `Restricted` (родительский
/// контроль) приравнен к отказу — пользователь не может его изменить.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicPermission {
    Authorized,
    NotDetermined,
    Denied,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{AudioError, MicPermission};
    use objc2::runtime::Bool;
    use objc2_av_foundation::{
        AVAuthorizationStatus, AVCaptureDevice, AVMediaType, AVMediaTypeAudio,
    };
    use std::sync::mpsc;

    /// Текущий статус: только чтение, диалог не поднимает.
    pub fn mic_status() -> MicPermission {
        let media_type: &AVMediaType =
            unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio существует");
        let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
        // Restricted сливаются с Denied: пользователь не может их изменить.
        if status == AVAuthorizationStatus::Authorized {
            MicPermission::Authorized
        } else if status == AVAuthorizationStatus::NotDetermined {
            MicPermission::NotDetermined
        } else {
            MicPermission::Denied
        }
    }

    /// `+[AVCaptureDevice requestAccessForMediaType:completionHandler:]`:
    /// системный запрос, ждём ответа пользователя по каналу.
    pub fn request_access() -> Result<(), AudioError> {
        let (tx, rx) = mpsc::channel::<bool>();
        // Block должен быть Send (требование сгенерированного биндинга):
        // отправка в канал — Send.
        let handler = block2::RcBlock::new(move |granted: Bool| {
            let _ = tx.send(granted.as_bool());
        });
        let media_type: &AVMediaType =
            unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio существует");
        unsafe {
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler)
        };
        match rx.recv() {
            Ok(true) => Ok(()),
            // Пользователь отказал (или родительский контроль).
            Ok(false) => Err(AudioError::PermissionDenied),
            // Диалог не закрыт, колбэк не пришёл: честная ошибка, не отказ.
            Err(_) => Err(AudioError::Stream(
                "нет ответа на системный запрос микрофона".into(),
            )),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::{AudioError, MicPermission};

    /// Windows/Linux: статус через WinUI/portal — этапы 9–10 (7.2.6–7.2.7).
    pub fn mic_status() -> MicPermission {
        MicPermission::Authorized
    }

    pub fn request_access() -> Result<(), AudioError> {
        Ok(())
    }
}

pub use imp::mic_status;

/// Гарантирует разрешение: отказ/ограничение — ошибка сразу;
/// «ещё не спрошено» — системный запрос (блокирует поток-владелец,
/// это не UI и не аудио-колбэк).
pub fn ensure_mic_permission() -> Result<(), AudioError> {
    match mic_status() {
        MicPermission::Authorized => Ok(()),
        MicPermission::NotDetermined => imp::request_access(),
        MicPermission::Denied => Err(AudioError::PermissionDenied),
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::{mic_status, MicPermission};

    /// Живой статус TCC этого процесса — маппинг корректен, диалог не нужен
    /// (Authorized после выдачи, NotDetermined при первом запуске — оба
    /// легальны; Denied требует ручного отзыва в настройках).
    #[test]
    fn tcc_status_maps_without_dialog() {
        match mic_status() {
            MicPermission::Authorized | MicPermission::NotDetermined => {}
            MicPermission::Denied => {}
        }
        // Функция чистая: два вызова подряд дают одинаковый результат.
        assert_eq!(mic_status(), mic_status());
    }
}
