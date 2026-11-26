//! IPC-обёртки Tauri: каталог и аудио-команды (3.6). Типы — зеркало
//! serde-вывода ядра; аудио-логики в JS нет.

import { Channel, invoke } from '@tauri-apps/api/core';
import type { Catalog, Sensitivity, Settings, TunerState, Tuning } from '../lib/types';

/** Пользовательский строй для IPC (§5.6): имя — плоская строка. */
export interface PresetDto {
  id: string;
  instrumentId: string;
  name: string;
  strings: number[];
}

export async function getCatalog(): Promise<Catalog> {
  return await invoke<Catalog>('get_catalog');
}

export async function startListening(channel: Channel<TunerState>): Promise<void> {
  await invoke('start_listening', { channel });
}

export async function stopListening(): Promise<void> {
  await invoke('stop_listening');
}

export async function setReference(hz: number): Promise<void> {
  await invoke('set_reference', { hz });
}

export async function selectInstrument(id: string): Promise<void> {
  await invoke('select_instrument', { id });
}

/** Строй целиком: и встроенный, и пользовательский (движку нужны струны). */
export async function selectTuning(tuning: Tuning): Promise<void> {
  await invoke('select_tuning', { tuning });
}

/** Ручная струна; null — вернуться к автоопределению (ФТ-15). */
export async function selectString(index: number | null): Promise<void> {
  await invoke('select_string', { index });
}

export async function setTolerance(cents: number): Promise<void> {
  await invoke('set_tolerance', { cents });
}

export async function setSensitivity(sensitivity: Sensitivity): Promise<void> {
  await invoke('set_sensitivity', { sensitivity });
}

/** «Низкий диапазон» (5.3): окно 16384 в хроматике. */
export async function setLowRange(lowRange: boolean): Promise<void> {
  await invoke('set_low_range', { lowRange });
}

/** Настройки, переживающие перезапуск (5.2–5.3). */
export async function getSettings(): Promise<Settings> {
  return await invoke<Settings>('get_settings');
}

export async function saveSettings(settings: Settings): Promise<void> {
  await invoke('save_settings', { settings });
}

/** Пользовательские строи из файла (5.6). */
export async function getPresets(): Promise<PresetDto[]> {
  return await invoke<PresetDto[]>('get_presets');
}

export async function savePreset(preset: PresetDto): Promise<void> {
  await invoke('save_preset', { preset });
}

export async function deletePreset(id: string): Promise<void> {
  await invoke('delete_preset', { id });
}

/** Список доступных аудиовходов (desktop-only; на Android — пустой список). */
export async function getInputDevices(): Promise<string[]> {
  return await invoke<string[]>('get_input_devices');
}

/** Выбор аудиовхода (null — системный по умолчанию). */
export async function setInputDevice(name: string | null): Promise<void> {
  await invoke('set_input_device', { name });
}

/** ⌘, из нативного меню (6.7): открыть лист настроек. */
export async function onMenuSettings(cb: () => void): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen('menu-settings', () => cb());
  return unlisten;
}

export type Platform = 'android' | 'desktop';

/** Платформа: 'android' | 'desktop' (этап 5). */
export async function getPlatform(): Promise<Platform> {
  return await invoke<Platform>('platform');
}

/** Проверка фактического статуса разрешения микрофона. */
export async function hasMicPermission(): Promise<boolean> {
  return await invoke<boolean>('has_mic_permission');
}

/** Запрос разрешения микрофона (на Android — через Kotlin плагин). */
export async function requestMic(): Promise<void> {
  await invoke('request_mic');
}

/** Открыть системные настройки приложения (на Android — экран разрешений). */
export async function openAppSettings(): Promise<void> {
  await invoke('open_app_settings');
}

/** Управление флагом FLAG_KEEP_SCREEN_ON. */
export async function keepScreenOn(enabled: boolean): Promise<void> {
  await invoke('keep_screen_on', { enabled });
}

/** Вибрация 20 мс при попадании в строй. */
export async function vibrate(): Promise<void> {
  await invoke('vibrate');
}
