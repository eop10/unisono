#!/usr/bin/env python3
"""Генератор WAV-фикстур для тестов точности DSP (docs/source/04_dsp.md, 4.6).

Только стандартная библиотека Python. Сигналы: sine, harmonic, Karplus-Strong,
tanh-overdrive, noise (SNR 20/10/6), speech-like (формантный шум с syllable-огибающей,
негативный тест), silence. Формат: WAV 48 кГц / 16 бит / моно, манифест manifest.csv.

Запуск: python3 tools/gen_fixtures.py
Фикстуры коммитятся вместе с манифестом (4.6); генерация детерминирована (seed 42).
"""

import array
import csv
import math
import os
import random
import wave

FS = 48000
OUT = os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..",
                 "crates", "tuner-core", "tests", "fixtures")
)
random.seed(42)

MANIFEST = []


def window_for(f0):
    """Окно по нижней ноте (4.2.3, ADR-009): >= E2 4096, >= B1 8192, иначе 16384."""
    if f0 >= 82.41:
        return 4096
    if f0 >= 61.5:
        return 8192
    return 16384


def write_wav(name, samples, kind, f0=0.0, snr="", duration=None):
    os.makedirs(OUT, exist_ok=True)
    data = array.array("h", (max(-32768, min(32767, int(round(x * 32767.0))))
                             for x in samples))
    with wave.open(os.path.join(OUT, name), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(FS)
        w.writeframes(data.tobytes())
    MANIFEST.append({
        "file": name,
        "type": kind,
        "f0_hz": f"{f0:.4f}" if f0 else "0",
        "snr_db": snr,
        "window": window_for(f0) if f0 else 4096,
        "duration_s": f"{(duration if duration else len(samples) / FS):.3f}",
    })


def normalize(samples, db=-20.0):
    peak = max(abs(x) for x in samples) or 1.0
    amp = 10 ** (db / 20.0)
    return [x * amp / peak for x in samples]


def rms(samples):
    return math.sqrt(sum(x * x for x in samples) / len(samples)) or 1.0


def sine(freq, dur, amp=0.1):
    n = int(FS * dur)
    return [amp * math.sin(2 * math.pi * freq * i / FS) for i in range(n)]


def harmonic(freq, dur, decay=1.0, nh=6):
    n = int(FS * dur)
    phases = [random.uniform(0, 2 * math.pi) for _ in range(nh)]
    out = []
    for i in range(n):
        t = i / FS
        out.append(sum(math.sin(2 * math.pi * freq * (k + 1) * t + phases[k])
                       / (k + 1) ** decay for k in range(nh)))
    return out


def karplus_strong(freq, dur, seed):
    """KS-петля с усреднением x[n]+x[n+1]: задержка петли P − 0.5 отсчёта,
    истинная частота синтеза = FS / (P − 0.5) (проверено замером периода)."""
    rng = random.Random(seed)
    n = int(FS * dur)
    period = max(2, round(FS / freq))
    buf = [rng.uniform(-1, 1) for _ in range(period)]
    out = []
    for i in range(n):
        idx = i % period
        nxt = (idx + 1) % period
        out.append(buf[idx])
        buf[idx] = 0.9999 * 0.5 * (buf[idx] + buf[nxt])
    return out, period


def speech_like(dur, formants, syllable_rate, seed):
    """Речеподобный негативный сигнал: формантный шум (без периодичности),
    syllable-огибающая со случайными паузами. Два «диктора» = наборы формант."""
    rng = random.Random(seed)
    n = int(FS * dur)
    noise = [rng.uniform(-1, 1) for _ in range(n)]
    out = [0.0] * n
    syllable_len = int(FS / syllable_rate)
    for start in range(0, n, syllable_len):
        end = min(n, start + syllable_len)
        active = rng.random() > 0.3
        fs_local = [f * rng.uniform(0.85, 1.15) for f in formants]
        for fi, f in enumerate(fs_local):
            theta = 2 * math.pi * f / FS
            r = 0.97 - 0.01 * fi
            a1, a2 = 2 * r * math.cos(theta), -r * r
            y1 = y2 = 0.0
            gain = 1.0 / (fi + 1)
            for i in range(start, end):
                env = 0.0
                if active:
                    ph = (i - start) / syllable_len
                    env = math.sin(math.pi * ph) ** 2
                x = noise[i] * env
                y = a1 * y1 + a2 * y2 + x
                y2, y1 = y1, y
                out[i] += gain * y
    return out


def main():
    # 1. Синусы 30-1400 Гц (лог-сетка, покрывает границы групп точности).
    sines = [30.87, 32.70, 41.20, 49.00, 55.00, 61.74, 65.41, 73.42, 77.78,
             82.41, 98.00, 110.00, 123.47, 146.83, 196.00, 246.94, 293.66,
             329.63, 392.00, 440.00, 523.25, 587.33, 659.26, 783.99, 880.00,
             1046.50, 1174.66, 1318.51, 1396.91]
    for k, f in enumerate(sines):
        dur = 0.8 if f < 65 else 0.5
        write_wav(f"sine_{k:02d}_{f:.0f}hz.wav", normalize(sine(f, dur)), "sine", f)

    # 2. Гармонический ряд: спад 1/n и 1/n^2, случайные фазы.
    for k, f in enumerate([30.87, 65.41, 82.41, 196.00, 329.63, 659.26]):
        for decay in (1.0, 2.0):
            dur = 0.8 if f < 65 else 0.5
            name = f"harm_{k}_{f:.0f}hz_d{int(decay)}.wav"
            write_wav(name, normalize(harmonic(f, dur, decay)), "harmonic", f)

    # 3. Karplus-Strong: 20 нот E2-E4, 3 реализации на ноту.
    # Истина в манифесте — фактическая частота синтеза FS/(P-0.5), а не FS/P.
    for i in range(20):
        midi = round(40 + i * (64 - 40) / 19)
        f = 440.0 * 2 ** ((midi - 69) / 12)
        for rep in (1, 2, 3):
            name = f"ks_{i:02d}_m{midi}_r{rep}.wav"
            samples, period = karplus_strong(f, 0.6, rep * 1000 + midi)
            write_wav(name, normalize(samples), "ks", FS / (period - 0.5))

    # 4. Перегруз: tanh(3x) после гармонического ряда.
    for k, f in enumerate([65.41, 82.41, 110.00, 196.00, 329.63, 440.00]):
        dur = 0.8 if f < 65 else 0.5
        x = harmonic(f, dur, 1.0)
        peak = max(abs(v) for v in x)
        over = [math.tanh(3.0 * v / peak) for v in x]
        write_wav(f"over_{k}_{f:.0f}hz.wav", normalize(over), "overdrive", f)

    # 5. Шум: SNR 20/10/6 дБ поверх гармонического ряда.
    # Окно 16384 и длительность 1 с: усреднение по большому числу периодов
    # подавляет ложные локальные максимумы NSDF при SNR <= 10.
    for k, f in enumerate([82.41, 196.00, 440.00]):
        sig = harmonic(f, 1.0, 1.0)
        s_rms = rms(sig)
        for snr in (20, 10, 6):
            # Детерминированная реализация шума на файл: при SNR 6 σ оценки
            # ≈ 2,6 цента, отдельные «неудачные» отрезки шума не репрезентативны.
            rng = random.Random(1000 + k * 100 + snr)
            n_rms = s_rms * 10 ** (-snr / 20.0)
            mixed = [v + rng.uniform(-1, 1) * n_rms * math.sqrt(3)
                     for v in sig]
            name = f"noise_{k}_{f:.0f}hz_snr{snr}.wav"
            write_wav(name, normalize(mixed), "noise", f, snr)
            MANIFEST[-1]["window"] = 16384

    # 6. Речь (негативный тест): 10 с, два «диктора».
    write_wav("speech_0.wav", normalize(speech_like(10.0, [500, 1500, 2500], 3.5, 7)),
              "speech", 0, "", 10.0)
    write_wav("speech_1.wav", normalize(speech_like(10.0, [350, 1100, 2200], 4.5, 8)),
              "speech", 0, "", 10.0)

    # 7. Тишина / шум комнаты: -60 dBFS.
    quiet = [random.uniform(-1, 1) * 1e-3 * math.sqrt(3) for _ in range(FS // 2)]
    write_wav("silence.wav", quiet, "silence")

    with open(os.path.join(OUT, "manifest.csv"), "w", newline="") as fh:
        writer = csv.DictWriter(
            fh, fieldnames=["file", "type", "f0_hz", "snr_db", "window", "duration_s"])
        writer.writeheader()
        writer.writerows(MANIFEST)
    print(f"{len(MANIFEST)} fixtures -> {OUT}")


if __name__ == "__main__":
    main()