<script lang="ts">
  //! Индикатор уровня: 5 сегментов (ФТ-16 SHOULD, 6.3.1).
  import { t } from '../lib/i18n';
  import type { Language } from '../lib/types';

  let { db, lang }: { db: number | null; lang: Language } = $props();
  const thresholds = [-50, -42, -34, -26, -18];
  const lit = $derived(db === null ? 0 : thresholds.filter((th) => db >= th).length);
</script>

<div class="level" role="img" aria-label={`${t(lang, 'aria_level')}: ${lit}/5`}>
  {#each thresholds as _, i}
    <i class:lit={i < lit}></i>
  {/each}
</div>