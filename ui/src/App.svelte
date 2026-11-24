<script lang="ts">
  //! Оболочка: тема, язык, каталог из ядра, настройки и пресеты из storage,
  //! затем движок и прослушивание. Демо-панель — только DEV/?demo вне
  //! production (Prompt 1.3): в собранном приложении её нет.
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import DemoPanel from './lib/DemoPanel.svelte';
  import InstrumentSheet from './lib/InstrumentSheet.svelte';
  import ReferenceSheet from './lib/ReferenceSheet.svelte';
  import SettingsSheet from './lib/SettingsSheet.svelte';
  import TuningEditor from './lib/TuningEditor.svelte';
  import TunerScreen from './lib/TunerScreen.svelte';
  import { t } from './lib/i18n';
  import { catalogError, loadCatalog } from './stores/catalog';
  import { overlay, openOverlay } from './stores/overlay';
  import { applySettingsToEngine, loadSettings, settings } from './stores/settings';
  import { loadPresets } from './stores/customs';
  import { startListening, stopListening } from './stores/tuner';
  import { onMenuSettings } from './ipc/commands';

  const s = $derived($settings);
  const showDemo =
    import.meta.env.DEV ||
    (import.meta.env.MODE !== 'production' &&
      new URLSearchParams(window.location.search).has('demo'));

  let systemLight = $state(false);
  onMount(() => {
    // Нативное «Настройки…» ⌘, открывает лист (6.7); в браузере тихо игнорируется.
    void onMenuSettings(() => openOverlay('settings')).catch(() => {});
    // Порядок старта (part 2): каталог → настройки → пресеты → движок → звук.
    void (async () => {
      await loadCatalog();
      if (get(catalogError)) return; // без каталога не слушаем (как раньше)
      await loadSettings();
      await loadPresets();
      await applySettingsToEngine();
      void startListening();
    })();
    const mq = window.matchMedia('(prefers-color-scheme: light)');
    const apply = () => {
      systemLight = mq.matches;
    };
    apply();
    mq.addEventListener('change', apply);

    // Жизненный цикл (7.1.4): onPause/hidden останавливает звук, onResume возобновляет.
    const onVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        void stopListening();
      } else if (document.visibilityState === 'visible') {
        if (!get(catalogError)) {
          void startListening();
        }
      }
    };
    document.addEventListener('visibilitychange', onVisibilityChange);

    return () => {
      mq.removeEventListener('change', apply);
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
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
