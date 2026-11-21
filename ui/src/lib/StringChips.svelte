<script lang="ts">
  //! Чипы струн (6.3.6): буквы без октав, активная латунная с подчёркиванием,
  //! унисон-пара подсвечивается целиком; в хроматике компонент не рендерится.
  import { stringLetters } from '../lib/notes';
  import { t } from '../lib/i18n';
  import type { Language, Notation, Tuning } from '../lib/types';

  let {
    tuning,
    active,
    notation,
    lang,
    onSelect,
  }: {
    tuning: Tuning;
    active: number[];
    notation: Notation;
    lang: Language;
    onSelect: (indices: number[], midi: number) => void;
  } = $props();

  const letters = $derived(stringLetters(tuning.strings, notation, lang));
  const indicesOf = (midi: number) =>
    tuning.strings.map((m, i) => (m === midi ? i : -1)).filter((i) => i >= 0);
</script>

<div class="strings" role="group" aria-label={t(lang, 'strings')}>
  {#each tuning.strings as midi, i}
    <button
      class="str"
      class:on={active.includes(i)}
      aria-label={`${t(lang, 'aria_string')} ${letters[i]}`}
      aria-pressed={active.includes(i)}
      onclick={() => onSelect(indicesOf(midi), midi)}
    >{letters[i]}</button>
  {/each}
</div>