<script lang="ts">
  //! Стробоскоп (1.1): альтернатива шкале-нониусу — бегущие сегменты,
  //! скорость пропорциональна отклонению в центах, 0 центов — полная остановка.
  import { onDestroy } from 'svelte';
  import { fmtCents } from '../lib/format';
  import { t } from '../lib/i18n';
  import type { Language } from '../lib/types';

  let {
    cents,
    dim = false,
    inTune = false,
    showNeedle = true,
    lang,
  }: {
    cents: number | null;
    dim?: boolean;
    inTune?: boolean;
    showNeedle?: boolean;
    lang: Language;
  } = $props();

  const W = 600;
  const H = 88;
  const PAD = 14;
  const TRACK_W = W - PAD * 2;
  const PERIOD = 40;
  const BLOCK_W = 18;
  const COUNT = Math.ceil(TRACK_W / PERIOD) + 2;
  const blocks = Array.from({ length: COUNT }, (_, i) => i - 1);

  let phase = $state(0);
  let raf = 0;
  let last = 0;
  const reduce =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  $effect(() => {
    const active = showNeedle && !dim && cents !== null;
    const c = Math.max(-50, Math.min(50, cents ?? 0));
    if (!active) {
      return;
    }
    if (reduce) {
      phase = ((c * 0.4) % PERIOD + PERIOD) % PERIOD;
      return;
    }
    const speed = inTune && Math.abs(c) <= 0.4 ? 0 : c * 8;
    if (speed === 0) {
      return;
    }
    last = performance.now();
    const step = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      phase = (((phase + speed * dt) % PERIOD) + PERIOD) % PERIOD;
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  });
  onDestroy(() => cancelAnimationFrame(raf));

  const op = $derived(dim || !showNeedle ? 0.35 : 1);
  const now = $derived(Math.round(Math.max(-50, Math.min(50, cents ?? 0))));
  const ariaText = $derived(
    cents === null || dim ? '—' : `${fmtCents(cents)} ${t(lang, 'cents')}`,
  );
  const fill = $derived(inTune && showNeedle && !dim ? 'var(--ok, var(--brass))' : 'var(--text)');
</script>

<svg
  class="gauge strobe"
  viewBox="0 0 {W} {H}"
  role="slider"
  tabindex="0"
  aria-label={t(lang, 'aria_gauge')}
  aria-valuemin={-50}
  aria-valuemax={50}
  aria-valuenow={now}
  aria-valuetext={ariaText}
>
  <defs>
    <clipPath id="strobe-clip">
      <rect x={PAD} y="30" width={TRACK_W} height="28" rx="14" />
    </clipPath>
  </defs>
  <rect
    x={PAD}
    y="30"
    width={TRACK_W}
    height="28"
    rx="14"
    fill="var(--panel2)"
    stroke="var(--line)"
    stroke-width="1"
    opacity={op}
  />
  <g clip-path="url(#strobe-clip)" opacity={op}>
    {#each blocks as i}
      <rect
        x={PAD + i * PERIOD + phase}
        y="35"
        width={BLOCK_W}
        height="18"
        rx="4"
        fill={fill}
        opacity={inTune && showNeedle && !dim ? 0.95 : 0.72}
      />
    {/each}
  </g>
  <line
    x1={W / 2}
    y1="22"
    x2={W / 2}
    y2="66"
    stroke="var(--brass)"
    stroke-width="2"
    stroke-linecap="round"
  />
</svg>
