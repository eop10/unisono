<script lang="ts">
  //! Нижний лист / модальное окно: фокус-ловушка, Escape, возврат фокуса (6.9).
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import type { Language } from '../lib/types';
  import type { Snippet } from 'svelte';

  let {
    title,
    lang,
    onClose,
    children,
  }: {
    title: string;
    lang: Language;
    onClose: () => void;
    children: Snippet;
  } = $props();

  let panel: HTMLDivElement | undefined = $state();
  let restore: Element | null = null;

  const FOCUSABLE =
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

  onMount(() => {
    restore = document.activeElement;
    const first = panel?.querySelector<HTMLElement>('[data-autofocus]') ??
      panel?.querySelector<HTMLElement>(FOCUSABLE);
    first?.focus();
    return () => {
      (restore as HTMLElement | null)?.focus?.();
    };
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onClose();
      return;
    }
    if (e.key !== 'Tab' || !panel) return;
    const nodes = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
      (n) => !n.hasAttribute('disabled'),
    );
    if (nodes.length === 0) return;
    const first = nodes[0];
    const last = nodes[nodes.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

<button class="scrim" aria-label={t(lang, 'close')} onclick={onClose}></button>
<div
  class="sheet"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  tabindex="-1"
  bind:this={panel}
  onkeydown={onKeydown}
>
  <div class="sheet-head">
    <span class="sheet-title">{title}</span>
    <button class="icon-btn" aria-label={t(lang, 'close')} onclick={onClose}>✕</button>
  </div>
  {@render children()}
</div>