//! Генератор опорного тона (камертон) через Web Audio API.
//! Плавно нарастает и затухает без щелчков, автоматически ставит микрофон на паузу.

import { get, writable } from 'svelte/store';
import type { Temperament } from './types';
import { listening, startListening, stopListening } from '../stores/tuner';

export const playingTone = writable<number | null>(null);

let activeCtx: AudioContext | null = null;
let activeOsc: OscillatorNode | null = null;
let activeGain: GainNode | null = null;
let stopTimer: ReturnType<typeof setTimeout> | null = null;
let resumeMicOnStop = false;

export function temperamentOffsetCents(
  temperament: Temperament,
  len: number,
  index: number,
): number {
  if (temperament === 'sweetened') {
    if (len === 6) return [-2.0, -1.0, 0.0, -2.5, -1.0, 0.0][index] ?? 0;
    if (index + 3 === len) return -2.0;
    return 0;
  }
  if (temperament === 'just_fifths' && len === 4) {
    return [-3.9, -2.0, 0.0, 2.0][index] ?? 0;
  }
  return 0;
}

export function targetHzForNote(
  midi: number,
  referenceHz: number,
  capo = 0,
  offsetCents = 0,
): number {
  return referenceHz * Math.pow(2, (midi + capo - 69 + offsetCents / 100) / 12);
}

function teardownAudio() {
  if (stopTimer !== null) {
    clearTimeout(stopTimer);
    stopTimer = null;
  }
  const osc = activeOsc;
  const gain = activeGain;
  const ctx = activeCtx;
  activeOsc = null;
  activeGain = null;
  activeCtx = null;

  if (ctx && gain && osc) {
    try {
      const now = ctx.currentTime;
      gain.gain.cancelScheduledValues(now);
      gain.gain.setValueAtTime(Math.max(gain.gain.value, 0.0001), now);
      gain.gain.linearRampToValueAtTime(0, now + 0.08);
      osc.stop(now + 0.09);
      setTimeout(() => {
        void ctx.close().catch(() => {});
      }, 120);
    } catch {
      // Игнорируем в окружениях без полноценного Web Audio.
    }
  }
  playingTone.set(null);
}

export async function stopTone(): Promise<void> {
  teardownAudio();
  if (resumeMicOnStop) {
    resumeMicOnStop = false;
    await startListening();
  }
}

export async function playTone(hz: number, durationMs = 3000): Promise<void> {
  const current = get(playingTone);
  if (current !== null && Math.abs(current - hz) < 0.05) {
    await stopTone();
    return;
  }

  teardownAudio();

  if (get(listening)) {
    resumeMicOnStop = true;
    await stopListening();
  }

  const Ctx =
    typeof window !== 'undefined'
      ? window.AudioContext ||
        (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
      : undefined;
  if (!Ctx) return;

  const ctx = new Ctx();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  const now = ctx.currentTime;

  osc.type = 'sine';
  osc.frequency.setValueAtTime(hz, now);

  gain.gain.setValueAtTime(0.0001, now);
  gain.gain.linearRampToValueAtTime(0.18, now + 0.025);

  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.start(now);

  activeCtx = ctx;
  activeOsc = osc;
  activeGain = gain;
  playingTone.set(hz);

  stopTimer = setTimeout(() => {
    void stopTone();
  }, durationMs);
}
