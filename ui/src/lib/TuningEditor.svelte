<script lang="ts">
  //! Редактор строя (5.7): имя 1–24, струны 1–8, MIDI 21–96, уникальность
  //! в пределах инструмента, лимит 20; дубли нот разрешены (унисон).
  import Sheet from './Sheet.svelte';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { t } from '../lib/i18n';
  import { noteLetter, noteOctave } from '../lib/notes';
  import { instrumentName } from '../lib/types';
  import type { Tuning } from '../lib/types';
  import { MIDI_MAX, MIDI_MIN, NAME_MAX, validateTuning } from '../lib/validate';
  import { instruments, tuningsFor } from '../stores/catalog';
  import { customs, newPresetId, savePreset } from '../stores/customs';
  import { closeOverlay, editorContext } from '../stores/overlay';
  import { selectTuning, settings } from '../stores/settings';

  const ctx = $derived($editorContext);
  const lang = $derived($settings.language);
  const notation = $derived($settings.notation);
  const inst = $derived($instruments.find((i) => i.id === ctx?.instrumentId) ?? null);
  const existing = $derived($customs.filter((c) => c.instrument_id === ctx?.instrumentId));
  const editing = $derived(existing.find((c) => c.id === ctx?.tuningId) ?? null);

  let name = $state('');
  let strings = $state<number[]>([]);
  let error = $state<ReturnType<typeof validateTuning>>(null);

  /** Новый строй по умолчанию — струны текущего строя инструмента (гитара → 6). */
  function defaultStrings(instrumentId: string): number[] {
    const list = tuningsFor(instrumentId);
    const s = get(settings);
    const current =
      s.instrumentId === instrumentId ? list.find((t) => t.id === s.tuningId) : undefined;
    return [...(current ?? list[0])?.strings ?? [69]];
  }

  onMount(() => {
    if (!ctx) return;
    if (editing) {
      name = 'Custom' in editing.name ? editing.name.Custom : '';
      strings = [...editing.strings];
    } else {
      strings = defaultStrings(ctx.instrumentId);
    }
  });

  const midiOptions: number[] = [];
  for (let m = MIDI_MIN; m <= MIDI_MAX; m++) midiOptions.push(m);

  function save() {
    if (!ctx) return;
    const err = validateTuning(
      name,
      strings,
      existing.map((e) => ({
        id: e.id,
        name: 'Custom' in e.name ? e.name.Custom : '',
      })),
      ctx.tuningId,
    );
    error = err;
    if (err !== null) return;
    const tuning: Tuning = {
      id: editing?.id ?? newPresetId(),
      instrument_id: ctx.instrumentId,
      name: { Custom: name.trim() },
      strings: [...strings],
      built_in: false,
    };
    savePreset(tuning);
    selectTuning(ctx.instrumentId, tuning.id);
    closeOverlay();
  }
</script>

{#if ctx && inst}
  <Sheet
    title={`${editing ? t(lang, 'editor_edit') : t(lang, 'editor_new')} · ${instrumentName(inst, lang)}`}
    {lang}
    onClose={closeOverlay}
  >
    <div class="field">
      <label for="te-name">{t(lang, 'name')}</label>
      <input id="te-name" data-autofocus maxlength={NAME_MAX} bind:value={name} oninput={() => (error = null)} />
    </div>
    <div class="field">
      <label for="te-add">{t(lang, 'strings')} ({strings.length}/8)</label>
      {#each strings as m, i}
        <div class="string-edit-row">
          <select
            aria-label={`${t(lang, 'aria_string')} ${i + 1}`}
            value={m}
            onchange={(e) => {
              strings[i] = Number(e.currentTarget.value);
              error = null;
            }}
          >
            {#each midiOptions as opt}
              <option value={opt}>{noteLetter(opt, notation, lang)}{noteOctave(opt)} · {opt}</option>
            {/each}
          </select>
          <button
            class="icon-btn"
            aria-label={t(lang, 'remove_string')}
            disabled={strings.length <= 1}
            onclick={() => {
              strings = strings.filter((_, j) => j !== i);
              error = null;
            }}
          >✕</button>
        </div>
      {/each}
      <button
        class="btn ghost"
        id="te-add"
        disabled={strings.length >= 8}
        onclick={() => {
          strings = [...strings, 69];
          error = null;
        }}
      >{t(lang, 'add_string')}</button>
    </div>
    {#if error}
      <p class="err" role="alert">{t(lang, error)}</p>
    {/if}
    <div class="actions">
      <button class="btn ghost" onclick={closeOverlay}>{t(lang, 'cancel')}</button>
      <button class="btn primary" onclick={save}>{t(lang, 'save')}</button>
    </div>
  </Sheet>
{/if}