<script lang="ts">
  //! Экран «Тюнер» (6.3): один экран — одно число; статус без дубля центов.
  import { onDestroy, onMount, untrack } from 'svelte';
  import { get } from 'svelte/store';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import CentsGauge from './CentsGauge.svelte';
  import StrobeGauge from './StrobeGauge.svelte';
  import StabilitySparkline from './StabilitySparkline.svelte';
  import LevelMeter from './LevelMeter.svelte';
  import StringChips from './StringChips.svelte';
  import { fmtCents, fmtHz } from '../lib/format';
  import { t } from '../lib/i18n';
  import { noteLetter, noteOctave } from '../lib/notes';
  import { playTone, playingTone, targetHzForNote } from '../lib/tone';
  import { instrumentName, tuningName } from '../lib/types';
  import { currentInstrument, currentTuning, setReference, settings, showStrings } from '../stores/settings';
  import { listening, manualStrings, selectString, startListening, toggleListening, tuner } from '../stores/tuner';
  import { getPlatform } from '../stores/settings';
  import { openAppSettings } from '../ipc/commands';
  import { overlay, openOverlay } from '../stores/overlay';

  const s = $derived($settings);
  const lang = $derived(s.language);
  const inst = $derived($currentInstrument);
  const tun = $derived($currentTuning);
  const st = $derived($tuner);
  const stringsVisible = $derived($showStrings && tun !== null);

  const baseChipLabel = $derived(
    inst === null
      ? ''
      : inst.kind === 'Chromatic' || tun === null
        ? instrumentName(inst, lang)
        : `${instrumentName(inst, lang)} · ${tuningName(tun, lang)}`,
  );
  const chipLabel = $derived(
    s.capo === 0
      ? baseChipLabel
      : `${baseChipLabel} · Capo ${s.capo > 0 ? `+${s.capo}` : s.capo}`,
  );

  let centsHistory = $state<number[]>([]);
  $effect(() => {
    if ((st.kind === 'tracking' || st.kind === 'intune') && st.cents !== null) {
      const cents = st.cents;
      const prev = untrack(() => centsHistory);
      const next = [...prev, cents];
      centsHistory = next.length > 40 ? next.slice(next.length - 40) : next;
    } else if (st.kind === 'silent') {
      if (untrack(() => centsHistory.length > 0)) {
        centsHistory = [];
      }
    }
  });

  const toneHz = $derived.by(() => {
    if ($manualStrings.length > 0 && tun !== null) {
      const midi = tun.strings[$manualStrings[$manualStrings.length - 1]];
      if (midi !== undefined) {
        return targetHzForNote(midi, s.referenceHz, s.capo);
      }
    }
    return st.targetHz ?? s.referenceHz;
  });
  const isTonePlaying = $derived($playingTone !== null);

  let intonationOpen = $state(false);
  let openMidi = $state<number | null>(null);
  let openCents = $state<number | null>(null);
  let fret12Cents = $state<number | null>(null);

  function resetIntonation() {
    openMidi = null;
    openCents = null;
    fret12Cents = null;
  }

  $effect(() => {
    if (!intonationOpen) return;
    if ((st.kind === 'tracking' || st.kind === 'intune') && st.cents !== null && st.noteMidi !== null) {
      if (openCents === null && st.kind === 'intune') {
        openCents = st.cents;
        openMidi = st.noteMidi;
      } else if (openCents !== null && openMidi !== null && st.noteMidi === openMidi + 12) {
        fret12Cents = st.cents;
      }
    }
  });

  const intonationHint = $derived.by(() => {
    if (openCents === null) return t(lang, 'intonation_open_hint');
    if (fret12Cents === null) return t(lang, 'intonation_12_hint');
    const delta = fret12Cents - openCents;
    if (Math.abs(delta) <= s.toleranceCents) return t(lang, 'intonation_ok');
    return delta > 0
      ? `${t(lang, 'intonation_saddle_back')} (${fmtCents(delta)})`
      : `${t(lang, 'intonation_saddle_fwd')} (${fmtCents(delta)})`;
  });

  const idle = $derived(st.kind === 'silent' || st.kind === 'error' || st.kind === 'nopermission');
  const inTune = $derived(st.kind === 'intune');
  const letter = $derived(
    idle || st.noteMidi === null ? '—' : noteLetter(st.noteMidi, s.notation, lang),
  );
  const octave = $derived(st.noteMidi === null ? null : noteOctave(st.noteMidi));

  const statusKey = $derived(
    st.kind === 'silent'
      ? 'listening'
      : st.kind === 'uncertain'
        ? 'play_one'
        : st.kind === 'intune'
          ? 'in_tune'
          : st.cents === null
            ? 'listening'
            : st.cents < 0
              ? 'below'
              : st.cents > 0
                ? 'above'
                : 'even',
  );

  /** 6.7: Space — глобальный старт/стоп (rev 6.3); `1..8` — ручная струна (повтор — авто);
   *  ↑/↓ — A4 ±0,1. В полях ввода и при открытом листе не перехватываем. */
  function onKeyDown(e: KeyboardEvent): void {
    const target = e.target as HTMLElement | null;
    const inField = target !== null && /^(INPUT|SELECT|TEXTAREA)$/.test(target.tagName);
    if (inField) return;
    const sheetOpen = get(overlay) !== null;

    if (e.code === 'Space' || e.key === ' ') {
      // Space — глобальный старт/стоп (6.7, rev 6.3).
      if (e.repeat || sheetOpen) return;
      e.preventDefault();
      (document.activeElement as HTMLElement | null)?.blur();
      void toggleListening();
      return;
    }
    if (sheetOpen) return;

    // e.code — физическая клавиша: работает и в русской раскладке.
    if (/^Digit[1-8]$/.test(e.code)) {
      if (e.repeat || !tun) return; // хроматика без струн — игнор
      const index = Number(e.code.slice(5)) - 1;
      if (index >= tun.strings.length) return;
      e.preventDefault();
      const midi = tun.strings[index];
      const indices = tun.strings.map((m, j) => (m === midi ? j : -1)).filter((j) => j >= 0);
      selectString(indices, midi);
      return;
    }

    if (e.code === 'ArrowUp' || e.code === 'ArrowDown') {
      e.preventDefault();
      const step = e.code === 'ArrowUp' ? 0.1 : -0.1;
      setReference(Math.round((s.referenceHz + step) * 10) / 10);
    }
  }

  onMount(() => window.addEventListener('keydown', onKeyDown));
  onDestroy(() => window.removeEventListener('keydown', onKeyDown));

  /** Настройки доступа к микрофону: macOS — Privacy_Microphone, Android — экран приложения (7.2). */
  function openMicSettings(): void {
    void getPlatform().then((p) => {
      if (p === 'android') {
        void openAppSettings();
      } else {
        void openUrl('x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone');
      }
    });
  }
</script>

<div class="tuner-col">
  <div class="topbar">
    <button class="chip" aria-label={t(lang, 'instrument_tuning')} onclick={() => openOverlay('instrument')}>
      <span class="k">{chipLabel}</span><span class="chev"></span>
    </button>
    <div class="top-right">
      <button class="chip a4" aria-label={t(lang, 'reference')} onclick={() => openOverlay('reference')}>
        <span class="k">A4</span><span class="v">{fmtHz(s.referenceHz, lang)}</span><span class="k">{t(lang, 'hz')}</span>
      </button>
      <button class="icon-btn" aria-label={t(lang, 'settings')} onclick={() => openOverlay('settings')}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 0 1 0-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z"/>
          <path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z"/>
        </svg>
      </button>
    </div>
  </div>

  {#if st.kind === 'error' || st.kind === 'nopermission'}
    <div class="state-screen">
      <h2>{st.kind === 'error' ? t(lang, 'error_title') : t(lang, 'perm_title')}</h2>
      <p>{st.kind === 'error' ? t(lang, 'error_text') : t(lang, 'perm_text')}</p>
      <button class="btn primary" onclick={() => void startListening()}>{t(lang, 'retry')}</button>
      {#if st.kind === 'nopermission'}
        <button class="btn" onclick={openMicSettings}>{t(lang, 'perm_settings')}</button>
      {/if}
    </div>
  {:else}
    <div class="center" class:in-tune-text={inTune}>
      <div class="notewrap">
        <span
          class="note"
          class:in-tune={inTune}
          class:dim={st.kind === 'uncertain'}
          class:empty={idle}
          aria-label={`${t(lang, 'aria_note')} ${letter}${octave === null ? '' : ` ${octave}`}`}
        >{letter}</span>
        {#if octave !== null && !idle}
          <span class="oct">{octave}</span>
        {/if}
      </div>
      <div class="freq">
        <span class="val">{st.f0 === null ? '—' : fmtHz(st.f0, lang)}</span>
        <span class="unit">{t(lang, 'hz')}</span>
        <span class="sep">/</span>
        <span class="target">{st.targetHz === null ? '—' : fmtHz(st.targetHz, lang)}</span>
      </div>
      {#if s.strobe}
        <StrobeGauge
          cents={st.cents}
          dim={st.kind === 'uncertain'}
          {inTune}
          showNeedle={st.kind !== 'silent'}
          {lang}
        />
      {:else}
        <CentsGauge
          cents={st.cents}
          dim={st.kind === 'uncertain'}
          {inTune}
          showNeedle={st.kind !== 'silent'}
          {lang}
        />
      {/if}
      <div class="centsrow">
        <!-- 6.3.2: в Uncertain цифры скрыты — «—», подпись «центов» не показывается. -->
        <span class="cents" class:empty={idle || st.kind === 'uncertain'}>
          {st.kind === 'uncertain' || st.cents === null ? '—' : fmtCents(st.cents)}
        </span>
        <span class="centslbl" class:hidden={st.kind === 'silent' || st.kind === 'uncertain'}>
          {t(lang, 'cents')}
        </span>
      </div>
      {#if centsHistory.length >= 2 && !idle}
        <StabilitySparkline history={centsHistory} toleranceCents={s.toleranceCents} />
      {/if}
    </div>

    {#if stringsVisible && tun}
      <StringChips
        tuning={tun}
        active={st.stringIndices.length > 0 ? st.stringIndices : $manualStrings}
        notation={s.notation}
        {lang}
        onSelect={selectString}
      />
    {/if}

    {#if intonationOpen}
      <div class="intonation-bar">
        <span class="intonation-hint">{intonationHint}</span>
        {#if openCents === null && (st.kind === 'tracking' || st.kind === 'intune') && st.cents !== null && st.noteMidi !== null}
          <button
            class="btn ghost"
            onclick={() => {
              openCents = st.cents;
              openMidi = st.noteMidi;
            }}
          >{t(lang, 'save')}</button>
        {:else if openCents !== null && fret12Cents === null && (st.kind === 'tracking' || st.kind === 'intune') && st.cents !== null}
          <button
            class="btn ghost"
            onclick={() => {
              fret12Cents = st.cents;
            }}
          >12</button>
        {/if}
        <button class="btn ghost" onclick={resetIntonation}>{t(lang, 'intonation_reset')}</button>
      </div>
    {/if}

    <div class="bottom">
      <button
        class="listen"
        class:on={$listening}
        aria-pressed={$listening}
        aria-label={t(lang, 'aria_listen')}
        onclick={() => void toggleListening()}
      >{$listening ? t(lang, 'stop') : t(lang, 'listen')}</button>
      <button
        class="btn ghost"
        class:on={isTonePlaying}
        aria-pressed={isTonePlaying}
        onclick={() => void playTone(toneHz)}
      >{isTonePlaying ? t(lang, 'tone_stop') : t(lang, 'tone_play')}</button>
      {#if stringsVisible}
        <button
          class="btn ghost"
          class:on={intonationOpen}
          aria-pressed={intonationOpen}
          onclick={() => {
            intonationOpen = !intonationOpen;
            if (!intonationOpen) resetIntonation();
          }}
        >{t(lang, 'intonation')}</button>
      {/if}
      <!-- Движок шлёт уровень даже в «Слушаю…» — индикатор живой (ФТ-16). -->
      <LevelMeter db={st.db} {lang} />
      <div class="status" class:idle={st.kind === 'silent' || st.kind === 'uncertain'} class:tune={inTune}>
        <i class="dot"></i><span>{t(lang, statusKey)}</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .intonation-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 10px;
    background: var(--surface-2, rgba(128, 128, 128, 0.1));
    font-size: 13px;
    color: var(--ink-2);
  }
  .intonation-hint {
    font-weight: 500;
    color: var(--ink-1);
  }
</style>