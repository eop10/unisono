<script lang="ts">
  //! Лист опорной частоты (6.5): ввод с клавиатуры (запятая и точка),
  //! слайдер 400–480 шаг 0,1, список с пояснениями, «Готово».
  import Sheet from './Sheet.svelte';
  import { fmtHz, parseReference } from '../lib/format';
  import { REFERENCE_LIST, t } from '../lib/i18n';
  import { closeOverlay } from '../stores/overlay';
  import { setReference, settings } from '../stores/settings';

  const s = $derived($settings);
  const lang = $derived(s.language);
  let draft = $state('');

  $effect(() => {
    draft = fmtHz(s.referenceHz, lang);
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
  <div class="actions">
    <button class="btn primary" onclick={closeOverlay}>{t(lang, 'done')}</button>
  </div>
</Sheet>