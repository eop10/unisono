//! Разрешение на микрофон (7.2.1): cpal/CoreAudio при отказе TCC отдаёт
//! **тишину** вместо ошибки — UI показывал бы «Слушаю…» вместо экрана
//! `NoPermission`. Поэтому статус спрашиваем явно через AVFoundation,
//! до открытия устройства, и всегда в потоке-владельце (не в UI).
//!
//! Отказ распознаётся только по статусу TCC, не по тексту ошибки cpal.

use crate::AudioError;

#[cfg(target_os = "macos")]
mod imp {
    use super::AudioError;
    use objc2::runtime::Bool;
    use objc2_av_foundation::{
        AVAuthorizationStatus, AVCaptureDevice, AVMediaType, AVMediaTypeAudio,
    };
    use std::sync::mpsc;

    /// Статус → ошибка: отказ и родительский контроль понятны сразу;
    /// `NotDetermined` обрабатываем запросом, `Authorized` пропускаем.
    pub fn status_to_error(status: AVAuthorizationStatus) -> Option<AudioError> {
        if status == AVAuthorizationStatus::Denied || status == AVAuthorizationStatus::Restricted {
            Some(AudioError::PermissionDenied)
        } else {
            None
        }
    }

    /// Гарантирует разрешение на микрофон. `NotDetermined` поднимает
    /// системный запрос и ждёт ответа (поток-владелец может блокироваться,
    /// это не UI и не аудио-колбэк).
    pub fn ensure_mic_permission() -> Result<(), AudioError> {
        let media_type: &AVMediaType =
            unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio существует");
        let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
        if let Some(err) = status_to_error(status) {
            return Err(err);
        }
        if status == AVAuthorizationStatus::NotDetermined {
            return request_access(media_type);
        }
        Ok(())
    }

    /// `+[AVCaptureDevice requestAccessForMediaType:completionHandler:]`:
    /// системный запрос, ждём ответа пользователя по каналу.
    fn request_access(media_type: &AVMediaType) -> Result<(), AudioError> {
        let (tx, rx) = mpsc::channel::<bool>();
        // Block должен быть Send (требование сгенерированного биндинга):
        // отправка в канал — Send.
        let handler = block2::RcBlock::new(move |granted: Bool| {
            let _ = tx.send(granted.as_bool());
        });
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
    use super::AudioError;

    /// Windows/Linux: статус через WinUI/portal — этапы 9–10 (7.2.6–7.2.7).
    pub fn ensure_mic_permission() -> Result<(), AudioError> {
        Ok(())
    }
}

pub use imp::ensure_mic_permission;

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::imp::status_to_error;
    use crate::AudioError;
    use objc2_av_foundation::AVAuthorizationStatus;

    /// Маппинг статусов TCC (7.2.1): отказ и restricted → PermissionDenied,
    /// авторизован и «ещё не спрошено» → ошибки нет.
    #[test]
    fn tcc_status_mapping() {
        assert!(status_to_error(AVAuthorizationStatus::Authorized).is_none());
        assert!(status_to_error(AVAuthorizationStatus::NotDetermined).is_none());
        assert!(matches!(
            status_to_error(AVAuthorizationStatus::Denied),
            Some(AudioError::PermissionDenied)
        ));
        assert!(matches!(
            status_to_error(AVAuthorizationStatus::Restricted),
            Some(AudioError::PermissionDenied)
        ));
    }
}
