<script lang="ts">
  //! Демо-панель: не часть интерфейса, только DEV/?demo (Prompt 1.3).
  //! Работает поверх реального потока: любое действие сначала останавливает
  //! прослушивание, иначе движок перезапишет демо-состояние.
  import { t } from '../lib/i18n';
  import type { TunerKind } from '../lib/types';
  import { currentTuning, settings } from '../stores/settings';
  import {
    demoReset,
    demoSetCents,
    demoSetKind,
    selectString,
    stopListening,
    tuner,
  } from '../stores/tuner';

  const lang = $derived($settings.language);
  const st = $derived($tuner);
  const tun = $derived($currentTuning);
  const kinds: TunerKind[] = ['tracking', 'intune', 'silent', 'uncertain', 'error', 'nopermission'];

  /** Демо берёт управление: сначала стоп реального захвата. */
  async function takeover(apply: () => void): Promise<void> {
    await stopListening();
    apply();
  }
</script>

<aside class="demo" aria-label="demo">
  <span class="badge">demo</span>
  <label for="demo-kind">{t(lang, 'settings')}</label>
  <select
    id="demo-kind"
    value={st.kind}
    onchange={(e) => void takeover(() => demoSetKind(e.currentTarget.value as TunerKind))}
  >
    {#each kinds as k}
      <option value={k}>{k}</option>
    {/each}
  </select>
  <label for="demo-cents">{t(lang, 'cents')}: {st.cents ?? '—'}</label>
  <input
    id="demo-cents"
    type="range"
    min="-50"
    max="50"
    step="1"
    value={st.cents ?? 0}
    oninput={(e) => void takeover(() => demoSetCents(Number(e.currentTarget.value)))}
  />
  {#if tun}
    <label for="demo-string">{t(lang, 'strings')}</label>
    <select
      id="demo-string"
      value={st.stringIndices[0] ?? 0}
      onchange={(e) => {
        const i = Number(e.currentTarget.value);
        const midi = tun.strings[i];
        const indices = tun.strings.map((m, j) => (m === midi ? j : -1)).filter((j) => j >= 0);
        void takeover(() => selectString(indices, midi));
      }}
    >
      {#each tun.strings as m, i}
        <option value={i}>{i + 1} · MIDI {m}</option>
      {/each}
    </select>
  {/if}
  <button class="btn ghost" onclick={() => void takeover(demoReset)}>reset</button>
</aside>
