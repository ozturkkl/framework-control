<script lang="ts">
  // Shared telemetry line chart, styled similarly to Fan curve
  import { hashColor } from "../lib/seriesColors";
  export let series: Record<string, Array<[number, number]>> = {};
  export let yMin = 0;
  export let yMax = 100;

  // Time domain in ms
  let allTimes: number[] = [];
  let tMin: number | null = null;
  let tMax: number | null = null;
  $: allTimes = Object.values(series)
    .flat()
    .map((p) => p[0]);
  $: tMin = allTimes.length ? Math.min(...allTimes) : null;
  $: tMax = allTimes.length ? Math.max(...allTimes) : null;

  // SVG and layout
  const padding = { left: 36, right: 12, top: 12, bottom: 22 };
  let svgWidth = 400;
  let svgHeight = 180;

  function xToPx(x: number) {
    if (tMin == null || tMax == null || tMax === tMin) return padding.left;
    const w = svgWidth - padding.left - padding.right;
    return padding.left + ((x - tMin) / (tMax - tMin)) * w;
  }
  function yToPx(y: number) {
    const h = svgHeight - padding.top - padding.bottom;
    return padding.top + (1 - (y - yMin) / (yMax - yMin)) * h;
  }

  function buildPath(points: Array<[number, number]>) {
    if (!points.length) return "";
    return points
      .map((p, i) => `${i === 0 ? "M" : "L"}${xToPx(p[0])},${yToPx(p[1])}`)
      .join(" ");
  }

  function formatTick(ms: number) {
    const s = Math.round(ms / 1000);
    const m = Math.floor(s / 60);
    const ss = String(s % 60).padStart(2, "0");
    return `${m}:${ss}`;
  }

  // Compute nice time ticks (~6 ticks)
  $: timeTicks = (() => {
    if (tMin == null || tMax == null) return [] as number[];
    const span = Math.max(1, tMax - tMin);
    const target = 6;
    const stepCandidates = [5, 10, 15, 30, 60, 120, 300].map((s) => s * 1000);
    const approx = span / target;
    let step = stepCandidates[0];
    for (const c of stepCandidates)
      if (c >= approx) {
        step = c;
        break;
      }
    const first = Math.ceil(tMin / step) * step;
    const ticks: number[] = [];
    for (let t = first; t <= tMax; t += step) ticks.push(t);
    return ticks;
  })();
</script>

<div>
  <div class="px-3 pb-1">
    <svg
      class="w-full bg-base-100 rounded border border-base-300"
      viewBox={`0 0 ${svgWidth} ${svgHeight}`}
    >
      <!-- axes -->
      <g stroke="currentColor" class="opacity-30">
        <line
          x1={padding.left}
          y1={yToPx(yMin)}
          x2={svgWidth - padding.right}
          y2={yToPx(yMin)}
          stroke-width="1"
        />
        <line
          x1={padding.left}
          y1={padding.top}
          x2={padding.left}
          y2={svgHeight - padding.bottom}
          stroke-width="1"
        />
      </g>

      <!-- horizontal gridlines and labels -->
      {#each [0, 20, 40, 60, 80, 100] as d}
        <g>
          <line
            x1={padding.left}
            y1={yToPx(d)}
            x2={svgWidth - padding.right}
            y2={yToPx(d)}
            stroke="currentColor"
            class="opacity-10"
          />
          <text
            x={padding.left - 6}
            y={yToPx(d) + 4}
            text-anchor="end"
            class="fill-current opacity-60 text-[10px]">{d}°C</text
          >
        </g>
      {/each}

      <!-- vertical time gridlines -->
      {#each timeTicks as t}
        <g>
          <line
            x1={xToPx(t)}
            y1={padding.top}
            x2={xToPx(t)}
            y2={svgHeight - padding.bottom}
            stroke="currentColor"
            class="opacity-10"
          />
          <text
            x={xToPx(t)}
            y={svgHeight - padding.bottom + 16}
            text-anchor="middle"
            class="fill-current opacity-60 text-[10px]"
            >{formatTick(t - (tMin ?? 0))}</text
          >
        </g>
      {/each}

      <!-- series lines -->
      {#each Object.entries(series) as [name, pts]}
        <path
          d={buildPath(pts)}
          fill="none"
          stroke={hashColor(name)}
          stroke-width="2"
        />
      {/each}
    </svg>
  </div>
  {#if !allTimes.length}
    <div class="px-3 pb-3 text-xs opacity-70">No telemetry data yet.</div>
  {/if}
  <slot />
</div>
