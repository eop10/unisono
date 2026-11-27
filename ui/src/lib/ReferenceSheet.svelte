<script lang="ts">
  //! Лист опорной частоты (6.5): ввод с клавиатуры (запятая и точка),
  //! слайдер 400–480 шаг 0,1, список с пояснениями, «Готово».
  import Sheet from './Sheet.svelte';
  import { fmtHz, parseReference } from '../lib/format';
  import { REFERENCE_LIST, t } from '../lib/i18n';
  import { playTone, playingTone } from '../lib/tone';
  import { closeOverlay } from '../stores/overlay';
  import { setReference, settings } from '../stores/settings';
  import { listening, startListening, stopListening, tuner } from '../stores/tuner';
  import { onDestroy } from 'svelte';

  const s = $derived($settings);
  const lang = $derived(s.language);
  const isPlayingA4 = $derived($playingTone !== null && Math.abs($playingTone - s.referenceHz) < 0.05);
  let draft = $state('');
  let capturing = $state(false);
  let startedForCapture = false;

  $effect(() => {
    draft = fmtHz(s.referenceHz, lang);
  });

  $effect(() => {
    if (!capturing) return;
    const st = $tuner;
    if ((st.kind === 'tracking' || st.kind === 'intune') && st.f0 !== null && st.noteMidi !== null) {
      const a4 = st.f0 * Math.pow(2, (69 - st.noteMidi) / 12);
      const clamped = Math.max(400, Math.min(480, Math.round(a4 * 10) / 10));
      setReference(clamped);
      capturing = false;
      if (startedForCapture) {
        startedForCapture = false;
        void stopListening();
      }
    }
  });

  function toggleCapture() {
    if (capturing) {
      capturing = false;
      if (startedForCapture) {
        startedForCapture = false;
        void stopListening();
      }
      return;
    }
    capturing = true;
    if (!$listening) {
      startedForCapture = true;
      void startListening();
    }
  }

  onDestroy(() => {
    if (capturing && startedForCapture) {
      void stopListening();
    }
  });

  function commit() {
    const v = parseReference(draft);
    if (v !== null) setReference(v);
    else draft = fmtHz(s.referenceHz, lang);
  }
</script>

<Sheet title={t(lang, 'reference')} {lang} onClose={closeOverlay}>
  <input
    class="ref-input"
    data-autofocus
    inputmode="decimal"
    aria-label={t(lang, 'reference_a4')}
    bind:value={draft}
    onfocus={(e) => e.currentTarget.select()}
    onblur={commit}
    onkeydown={(e) => {
      if (e.key === 'Enter') {
        commit();
        e.currentTarget.blur();
      }
    }}
  />
  <input
    class="ref-slider"
    type="range"
    min="400"
    max="480"
    step="0.1"
    value={s.referenceHz}
    aria-label={t(lang, 'reference_a4')}
    oninput={(e) => setReference(Number(e.currentTarget.value))}
  />
  <div class="sheet-section">{t(lang, 'reference')}</div>
  {#each REFERENCE_LIST as ref}
    <button
      class="ref-row"
      class:on={Math.abs(s.referenceHz - ref.hz) < 0.05}
      onclick={() => setReference(ref.hz)}
    >
      <span class="v">{fmtHz(ref.hz, lang)}</span>
      <span class="why">{t(lang, ref.key)}</span>
    </button>
  {/each}
  <div class="actions" style="justify-content: space-between; gap: 8px; flex-wrap: wrap;">
    <div style="display: flex; gap: 8px; flex-wrap: wrap;">
      <button
        class="btn ghost"
        class:on={isPlayingA4}
        onclick={() => void playTone(s.referenceHz)}
      >
        {isPlayingA4 ? t(lang, 'tone_stop') : t(lang, 'tone_a4')}
      </button>
      <button
        class="btn ghost"
        class:on={capturing}
        onclick={toggleCapture}
      >
        {capturing ? t(lang, 'capture_a4_wait') : t(lang, 'capture_a4')}
      </button>
    </div>
    <button class="btn primary" onclick={closeOverlay}>{t(lang, 'done')}</button>
  </div>
</Sheet>