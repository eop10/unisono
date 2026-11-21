<script lang="ts">
  //! Экран «Тюнер» (6.3): один экран — одно число; статус без дубля центов.
  import CentsGauge from './CentsGauge.svelte';
  import LevelMeter from './LevelMeter.svelte';
  import StringChips from './StringChips.svelte';
  import { fmtCents, fmtHz } from '../lib/format';
  import { t } from '../lib/i18n';
  import { noteLetter, noteOctave } from '../lib/notes';
  import { instrumentName, tuningName } from '../lib/types';
  import { currentInstrument, currentTuning, settings, showStrings } from '../stores/settings';
  import { selectString, tuner } from '../stores/tuner';
  import { openOverlay } from '../stores/overlay';

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
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
          <circle cx="12" cy="12" r="3.2" />
          <path d="M12 2.8v3M12 18.2v3M2.8 12h3M18.2 12h3M5.5 5.5l2.1 2.1M16.4 16.4l2.1 2.1M18.5 5.5l-2.1 2.1M7.6 16.4l-2.1 2.1" />
        </svg>
      </button>
    </div>
  </div>

  {#if st.kind === 'error' || st.kind === 'nopermission'}
    <div class="state-screen">
      <h2>{st.kind === 'error' ? t(lang, 'error_title') : t(lang, 'perm_title')}</h2>
      <p>{st.kind === 'error' ? t(lang, 'error_text') : t(lang, 'perm_text')}</p>
      {#if st.kind === 'error'}
        <button class="btn primary" onclick={() => openOverlay('settings')}>{t(lang, 'retry')}</button>
      {:else}
        <button class="btn primary" onclick={() => openOverlay('settings')}>{t(lang, 'perm_settings')}</button>
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
        <span class="cents" class:dim={st.kind === 'uncertain'} class:empty={idle}>
          {st.cents === null ? '—' : fmtCents(st.cents)}
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
      <LevelMeter db={st.kind === 'silent' ? -55 : st.db} {lang} />
      <div class="status" class:idle={st.kind === 'silent' || st.kind === 'uncertain'} class:tune={inTune}>
        <i class="dot"></i><span>{t(lang, statusKey)}</span>
      </div>
    </div>
  {/if}
</div>