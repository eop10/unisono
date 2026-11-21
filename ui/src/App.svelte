<script lang="ts">
  //! Оболочка: тема, язык, каталог из ядра, листы, демо-панель (DEV/?demo).
  import { onMount } from 'svelte';
  import DemoPanel from './lib/DemoPanel.svelte';
  import InstrumentSheet from './lib/InstrumentSheet.svelte';
  import ReferenceSheet from './lib/ReferenceSheet.svelte';
  import SettingsSheet from './lib/SettingsSheet.svelte';
  import TuningEditor from './lib/TuningEditor.svelte';
  import TunerScreen from './lib/TunerScreen.svelte';
  import { t } from './lib/i18n';
  import { catalogError, loadCatalog } from './stores/catalog';
  import { overlay } from './stores/overlay';
  import { settings } from './stores/settings';

  const s = $derived($settings);
  const showDemo =
    import.meta.env.DEV || new URLSearchParams(window.location.search).has('demo');

  let systemLight = $state(false);
  onMount(() => {
    void loadCatalog();
    const mq = window.matchMedia('(prefers-color-scheme: light)');
    const apply = () => {
      systemLight = mq.matches;
    };
    apply();
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  });

  const theme = $derived(s.theme === 'system' ? (systemLight ? 'light' : 'dark') : s.theme);
  $effect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.lang = s.language;
  });
</script>

<div class="app-shell" class:big-type={s.bigType}>
  {#if $catalogError}
    <p class="catalog-error">{t(s.language, 'catalog_error')}</p>
  {:else}
    <TunerScreen />
  {/if}
</div>

{#if $overlay === 'instrument'}
  <InstrumentSheet />
{:else if $overlay === 'reference'}
  <ReferenceSheet />
{:else if $overlay === 'settings'}
  <SettingsSheet />
{:else if $overlay === 'editor'}
  <TuningEditor />
{/if}

{#if showDemo}
  <DemoPanel />
{/if}