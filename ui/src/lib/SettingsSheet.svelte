<script lang="ts">
  //! Настройки (6.6): строй, отображение, точность, обратная связь, о приложении.
  import Sheet from './Sheet.svelte';
  import { fmtHz } from '../lib/format';
  import { t } from '../lib/i18n';
  import { instrumentName, tuningName } from '../lib/types';
  import type { Language, Notation, Sensitivity, Theme, Tolerance } from '../lib/types';
  import { currentInstrument, currentTuning, resetSettings, settings } from '../stores/settings';
  import { closeOverlay, openOverlay } from '../stores/overlay';

  const s = $derived($settings);
  const lang = $derived(s.language);
  const inst = $derived($currentInstrument);
  const tun = $derived($currentTuning);

  function set<K extends keyof typeof s>(key: K, value: (typeof s)[K]) {
    settings.update((x) => ({ ...x, [key]: value }));
  }
</script>

<Sheet title={t(lang, 'settings')} {lang} onClose={closeOverlay}>
  <div class="sheet-section">{t(lang, 'section_tuning')}</div>
  <div class="row">
    <span class="lbl">{t(lang, 'reference_a4')}</span>
    <button class="btn ghost" onclick={() => openOverlay('reference')}>{fmtHz(s.referenceHz, lang)} {t(lang, 'hz')}</button>
  </div>
  <div class="row">
    <span class="lbl">{t(lang, 'instruments_tunings')}</span>
    <button class="btn ghost" onclick={() => openOverlay('instrument')}>
      {inst ? instrumentName(inst, lang) : ''}{tun ? ` · ${tuningName(tun, lang)}` : ''}
    </button>
  </div>

  <div class="sheet-section">{t(lang, 'section_display')}</div>
  <div class="row">
    <span class="lbl">{t(lang, 'notation')}</span>
    <span class="seg">
      <button class:on={s.notation === 'latin'} aria-pressed={s.notation === 'latin'} onclick={() => set('notation', 'latin' as Notation)}>{t(lang, 'notation_latin')}</button>
      <button class:on={s.notation === 'solfeggio'} aria-pressed={s.notation === 'solfeggio'} onclick={() => set('notation', 'solfeggio' as Notation)}>{t(lang, 'notation_solf')}</button>
    </span>
  </div>
  <div class="row">
    <span class="lbl">{t(lang, 'theme')}</span>
    <span class="seg">
      {#each ['system', 'light', 'dark'] as th}
        <button class:on={s.theme === th} aria-pressed={s.theme === th} onclick={() => set('theme', th as Theme)}>{t(lang, `theme_${th}` as 'theme_system')}</button>
      {/each}
    </span>
  </div>
  <div class="row">
    <span class="lbl">{t(lang, 'language')}</span>
    <span class="seg">
      <button class:on={s.language === 'ru'} aria-pressed={s.language === 'ru'} onclick={() => set('language', 'ru' as Language)}>Русский</button>
      <button class:on={s.language === 'en'} aria-pressed={s.language === 'en'} onclick={() => set('language', 'en' as Language)}>English</button>
    </span>
  </div>
  <div class="row">
    <span class="lbl">{t(lang, 'big_type')}</span>
    <button class="switch" role="switch" aria-checked={s.bigType} aria-label={t(lang, 'big_type')} onclick={() => set('bigType', !s.bigType)}></button>
  </div>

  <div class="sheet-section">{t(lang, 'section_precision')}</div>
  <div class="row">
    <span class="lbl">{t(lang, 'tolerance')}</span>
    <span class="seg">
      {#each [2, 3, 5] as tol}
        <button class:on={s.toleranceCents === tol} aria-pressed={s.toleranceCents === tol} onclick={() => set('toleranceCents', tol as Tolerance)}>{tol}</button>
      {/each}
    </span>
  </div>
  <div class="row">
    <span class="lbl">{t(lang, 'sensitivity')}</span>
    <span class="seg">
      {#each ['low', 'normal', 'high'] as se}
        <button class:on={s.sensitivity === se} aria-pressed={s.sensitivity === se} onclick={() => set('sensitivity', se as Sensitivity)}>{t(lang, `sens_${se}` as 'sens_low')}</button>
      {/each}
    </span>
  </div>
  <div class="row">
    <span class="lbl">{t(lang, 'low_range')}</span>
    <button class="switch" role="switch" aria-checked={s.lowRange} aria-label={t(lang, 'low_range')} onclick={() => set('lowRange', !s.lowRange)}></button>
  </div>

  <div class="sheet-section">{t(lang, 'section_feedback')}</div>
  <div class="row">
    <span class="lbl">{t(lang, 'haptics')}</span>
    <button class="switch" role="switch" aria-checked={s.haptics} aria-label={t(lang, 'haptics')} onclick={() => set('haptics', !s.haptics)}></button>
  </div>
  <div class="row">
    <span class="lbl">{t(lang, 'sound')}</span>
    <button class="switch" role="switch" aria-checked={s.sound} aria-label={t(lang, 'sound')} onclick={() => set('sound', !s.sound)}></button>
  </div>

  <div class="sheet-section">{t(lang, 'section_about')}</div>
  <p class="about">
    Unisono · {t(lang, 'version')} 0.1.0 · {t(lang, 'unsigned')}<br />
    {t(lang, 'licenses')} · {t(lang, 'privacy')}
  </p>
  <div class="actions" style="justify-content:flex-start">
    <button class="btn danger" onclick={resetSettings}>{t(lang, 'reset')}</button>
  </div>
</Sheet>