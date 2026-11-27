<script lang="ts">
  //! Мини-график стабильности высоты тона: последние ~40 замеров в центах.

  interface Props {
    history: number[];
    toleranceCents: number;
  }

  const { history, toleranceCents }: Props = $props();

  const W = 220;
  const H = 26;
  const RANGE = 50;

  const points = $derived.by(() => {
    if (history.length < 2) return '';
    const n = history.length;
    return history
      .map((c, i) => {
        const x = (i / (n - 1)) * W;
        const clamped = Math.max(-RANGE, Math.min(RANGE, c));
        const y = H / 2 - (clamped / RANGE) * (H / 2 - 3);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  });

  const bandTop = $derived(H / 2 - (Math.min(RANGE, toleranceCents) / RANGE) * (H / 2 - 3));
  const bandHeight = $derived(
    ((Math.min(RANGE, toleranceCents) * 2) / RANGE) * (H / 2 - 3),
  );
</script>

<svg
  class="sparkline"
  viewBox="0 0 {W} {H}"
  width={W}
  height={H}
  aria-hidden="true"
>
  <rect
    x="0"
    y={bandTop}
    width={W}
    height={bandHeight}
    fill="var(--intune)"
    opacity="0.1"
    rx="2"
  />
  <line
    x1="0"
    y1={H / 2}
    x2={W}
    y2={H / 2}
    stroke="var(--hairline)"
    stroke-width="1"
    stroke-dasharray="2 3"
  />
  {#if points}
    <polyline
      fill="none"
      stroke="var(--ink-2)"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      {points}
    />
  {/if}
</svg>

<style>
  .sparkline {
    display: block;
    margin: 2px auto 0;
    overflow: visible;
  }
</style>
