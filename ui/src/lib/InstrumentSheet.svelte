<script lang="ts">
  //! Лист «Инструмент · Строй» (6.4): выбор инструмента не закрывает лист
  //! (кроме хроматики), выбор строя закрывает; «Новый строй» кроме хроматики.
  import Sheet from './Sheet.svelte';
  import { t } from '../lib/i18n';
  import { noteLetter } from '../lib/notes';
  import { instrumentName, tuningName } from '../lib/types';
  import { instruments, tuningsByInstrument } from '../stores/catalog';
  import { closeOverlay, openEditor } from '../stores/overlay';
  import { selectInstrument, selectTuning, settings } from '../stores/settings';

  const s = $derived($settings);
  const lang = $derived(s.language);
  const list = $derived($tuningsByInstrument.get(s.instrumentId) ?? []);
  const isChromatic = $derived(
    $instruments.find((i) => i.id === s.instrumentId)?.kind === 'Chromatic',
  );

  function pickInstrument(id: string) {
    selectInstrument(id);
    if ($instruments.find((i) => i.id === id)?.kind === 'Chromatic') closeOverlay();
  }
</script>

<Sheet title={t(lang, 'instrument_tuning')} {lang} onClose={closeOverlay}>
  <div class="inst-grid" role="group" aria-label={t(lang, 'instrument_tuning')}>
    {#each $instruments as i}
      <button
        class="inst-chip"
        class:on={i.id === s.instrumentId}
        aria-pressed={i.id === s.instrumentId}
        onclick={() => pickInstrument(i.id)}
      >{instrumentName(i, lang)}</button>
    {/each}
  </div>

  {#if !isChromatic}
    <div class="sheet-section">{t(lang, 'strings')}</div>
    {#each list as tun}
      <div class="tuning-row">
        <button
          class="tuning-row"
          style="border:0;padding:0;min-height:0"
          aria-pressed={tun.id === s.tuningId}
          onclick={() => {
            selectTuning(s.instrumentId, tun.id);
            closeOverlay();
          }}
        >
          <span>
            <span class="nm">{tuningName(tun, lang)}</span><br />
            <span class="notes">
              {tun.strings.map((m) => noteLetter(m, s.notation, lang)).join(' ')}
            </span>
          </span>
          {#if tun.id === s.tuningId}
            <span class="check" aria-hidden="true">✓</span>
          {/if}
        </button>
        {#if !tun.built_in}
          <button class="edit" onclick={() => openEditor(s.instrumentId, tun.id)}>{t(lang, 'edit')}</button>
        {/if}
      </div>
    {/each}
    <div class="actions" style="justify-content:flex-start">
      <button class="btn" onclick={() => openEditor(s.instrumentId, null)}>{t(lang, 'new_tuning')}</button>
    </div>
  {/if}
</Sheet>