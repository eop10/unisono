<script lang="ts">
  //! Шкала-нониус (6.3.2): грубая ±50 с рисками 5/25, тонкая ±5 шаг 1,
  //! метки допуска ±3, латунный ноль, игла с пружиной; reduced-motion — мгновенно.
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
  const BASE = 46;
  const PAD = 14;
  const x = (v: number) => W / 2 + v * ((W / 2 - PAD) / 50);

  const coarse: { v: number; major: boolean; len: number }[] = [];
  for (let v = -50; v <= 50; v += 5) {
    const major = v % 25 === 0;
    coarse.push({ v, major, len: major ? 13 : v % 10 === 0 ? 9 : 6 });
  }
  const fine: number[] = [];
  for (let v = -5; v <= 5; v++) fine.push(v);
  const labels: [number, string][] = [
    [-50, '−50'],
    [-25, '−25'],
    [0, '0'],
    [25, '+25'],
    [50, '+50'],
  ];

  let needle = $state(0);
  let velocity = 0;
  let raf = 0;
  let last = 0;
  const reduce =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  $effect(() => {
    const target = Math.max(-50, Math.min(50, cents ?? 0));
    if (reduce) {
      needle = target;
      return;
    }
    last = performance.now();
    const step = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      velocity += (target - needle) * 170 * dt;
      velocity *= Math.exp(-22 * dt);
      needle += velocity * dt;
      if (Math.abs(target - needle) > 0.01 || Math.abs(velocity) > 0.01) {
        raf = requestAnimationFrame(step);
      } else {
        needle = target;
      }
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  });
  onDestroy(() => cancelAnimationFrame(raf));

  const op = $derived(dim ? 0.35 : 1);
  const nx = $derived(x(Math.max(-50, Math.min(50, needle))));
  const now = $derived(Math.round(Math.max(-50, Math.min(50, cents ?? 0))));
</script>

<svg
  class="gauge"
  viewBox="0 0 {W} {H}"
  role="slider"
  tabindex="0"
  aria-label={t(lang, 'aria_gauge')}
  aria-valuemin={-50}
  aria-valuemax={50}
  aria-valuenow={now}
  aria-valuetext={cents === null ? '—' : `${fmtCents(cents)} ${t(lang, 'cents')}`}
>
  <line x1={PAD - 4} y1={BASE} x2={W - PAD + 4} y2={BASE} stroke="var(--line)" stroke-width="1" opacity={op} />
  {#each coarse as tick}
    <line
      x1={x(tick.v)}
      y1={BASE - tick.len}
      x2={x(tick.v)}
      y2={BASE - 1}
      stroke={tick.major ? 'var(--muted)' : 'var(--line2)'}
      stroke-width={tick.major ? 1.2 : 1}
      opacity={op}
    />
  {/each}
  {#each fine as v}
    <line
      x1={x(v)}
      y1={BASE + 1}
      x2={x(v)}
      y2={BASE + (Math.abs(v) === 5 ? 6 : 3)}
      stroke="var(--dim)"
      stroke-width="1"
      opacity={op}
    />
  {/each}
  {#each labels as [v, label]}
    <text
      x={x(v)}
      y={BASE + 21}
      text-anchor="middle"
      font-size="9"
      fill={v === 0 ? 'var(--brass)' : 'var(--muted)'}
      letter-spacing=".06em"
      opacity={op}
      font-family="var(--mono)">{label}</text>
  {/each}
  <text x={PAD} y={BASE + 34} font-size="8.5" fill="var(--muted)" letter-spacing=".12em" opacity={op} font-family="var(--mono)">{t(lang, 'gauge_flat')}</text>
  <text x={W - PAD} y={BASE + 34} font-size="8.5" fill="var(--muted)" letter-spacing=".12em" text-anchor="end" opacity={op} font-family="var(--mono)">{t(lang, 'gauge_sharp')}</text>
  <line x1={x(0)} y1={BASE - 20} x2={x(0)} y2={BASE + 8} stroke="var(--brass)" stroke-width="1" opacity={dim ? 0.25 : inTune ? 0.9 : 0.4} />
  {#each [-3, 3] as v}
    <line x1={x(v)} y1={BASE - 18} x2={x(v)} y2={BASE - 10} stroke="var(--brass)" stroke-width="1" opacity={dim ? 0.2 : 0.45} />
  {/each}
  {#if showNeedle}
    <line x1={nx} y1={BASE - 27} x2={nx} y2={BASE + 9} stroke={inTune ? 'var(--brass)' : 'var(--text)'} stroke-width="1.6" opacity={dim ? 0.5 : 1} />
    <circle cx={nx} cy={BASE + 9} r="2.2" fill={inTune ? 'var(--brass)' : 'var(--text)'} opacity={dim ? 0.5 : 1} />
  {/if}
</svg>