<script lang="ts">
  //! Экран «Тюнер» (6.3): один экран — одно число; статус без дубля центов.
  import { onDestroy, onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import CentsGauge from './CentsGauge.svelte';
  import LevelMeter from './LevelMeter.svelte';
  import StringChips from './StringChips.svelte';
  import { fmtCents, fmtHz } from '../lib/format';
  import { t } from '../lib/i18n';
  import { noteLetter, noteOctave } from '../lib/notes';
  import { instrumentName, tuningName } from '../lib/types';
  import { currentInstrument, currentTuning, setReference, settings, showStrings } from '../stores/settings';
  import { listening, selectString, startListening, toggleListening, tuner } from '../stores/tuner';
  import { overlay, openOverlay } from '../stores/overlay';

  const s = $derived($settings);
  const lang = $derived(s.language);
  const inst = $derived($currentInstrument);
  const tun = $derived($currentTuning);
  const st = $derived($tuner);
  const stringsVisible = $derived($showStrings && tun !== null);

  const chipLabel = $derived(
    inst === null
      ? ''
      : inst.kind === 'Chromatic' || tun === null
        ? instrumentName(inst, lang)
        : `${instrumentName(inst, lang)} · ${tuningName(tun, lang)}`,
  );

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

  /** 6.7: Space — старт/стоп; `1..8` — ручная струна (повтор — авто);
   *  ↑/↓ — A4 ±0,1. В полях ввода и при открытом листе не перехватываем. */
  function onKeyDown(e: KeyboardEvent): void {
    const target = e.target as HTMLElement | null;
    const inField = target !== null && /^(INPUT|SELECT|TEXTAREA)$/.test(target.tagName);
    if (inField) return;
    const sheetOpen = get(overlay) !== null;

    if (e.code === 'Space') {
      // Фокус на кнопке: Space нажимает её сам, не дублируем.
      if (target && target.tagName === 'BUTTON') return;
      if (e.repeat || sheetOpen) return;
      e.preventDefault();
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

  /** macOS: системные настройки доступа к микрофону (7.2). */
  function openMicSettings(): void {
    void openUrl('x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone');
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
      {#if st.kind === 'error'}
        <button class="btn primary" onclick={() => void startListening()}>{t(lang, 'retry')}</button>
      {:else}
        <button class="btn primary" onclick={openMicSettings}>{t(lang, 'perm_settings')}</button>
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
      <CentsGauge
        cents={st.cents}
        dim={st.kind === 'uncertain'}
        {inTune}
        showNeedle={st.kind !== 'silent'}
        {lang}
      />
      <div class="centsrow">
        <!-- 6.3.2: в Uncertain цифры скрыты — «—», подпись «центов» не показывается. -->
        <span class="cents" class:empty={idle || st.kind === 'uncertain'}>
          {st.kind === 'uncertain' || st.cents === null ? '—' : fmtCents(st.cents)}
        </span>
        <span class="centslbl" class:hidden={st.kind === 'silent' || st.kind === 'uncertain'}>
          {t(lang, 'cents')}
        </span>
      </div>
    </div>

    {#if stringsVisible && tun}
      <StringChips
        tuning={tun}
        active={st.stringIndices}
        notation={s.notation}
        {lang}
        onSelect={selectString}
      />
    {/if}

    <div class="bottom">
      <button
        class="listen"
        class:on={$listening}
        aria-pressed={$listening}
        aria-label={t(lang, 'aria_listen')}
        onclick={() => void toggleListening()}
      >{$listening ? t(lang, 'stop') : t(lang, 'listen')}</button>
      <!-- Движок шлёт уровень даже в «Слушаю…» — индикатор живой (ФТ-16). -->
      <LevelMeter db={st.db} {lang} />
      <div class="status" class:idle={st.kind === 'silent' || st.kind === 'uncertain'} class:tune={inTune}>
        <i class="dot"></i><span>{t(lang, statusKey)}</span>
      </div>
    </div>
  {/if}
</div>