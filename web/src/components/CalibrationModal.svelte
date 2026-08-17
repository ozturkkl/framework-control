<script lang="ts">
  import { DefaultService } from "../api";
  import type { FanOverride } from "../api";
  import { createEventDispatcher } from "svelte";
  import { tweened } from "svelte/motion";
  import { get } from "svelte/store";
  import { configStore, patch } from "../lib/config";

  type CalibrationFan = {
    index: number;
    points: [number, number][];
  };

  const dispatch = createEventDispatcher<{
    done: CalibrationFan[];
    cancel: void;
  }>();

  let progress = 0; // 0..100
  let info = "";
  let cancelled = false;
  let hasStarted = false;

  let prevMode: "manual" | "curve" | "disabled" = "curve";
  let prevOverrides: FanOverride[] = [];
  let restoreOverrides = false;

  // Simple tween with tunable power ease-out towards the actual progress
  const powOut = (p: number) => (t: number) => 1 - Math.pow(1 - t, p);
  const POW_P = 100; // 2..3 is a good range (higher = stronger ease-out)
  const animatedProgress = tweened(0, { duration: 0, easing: powOut(POW_P) });
  const MS_PER_PERCENT = 10000; // lower = faster

  $: {
    const from = get(animatedProgress);
    const to = Math.max(0, Math.min(100, progress));
    if (from !== to) {
      const delta = Math.abs(to - from);
      const duration = delta * MS_PER_PERCENT;
      animatedProgress.set(to, { duration, easing: powOut(POW_P) });
    }
  }

  async function restoreFanState() {
    try {
      await patch({
      fan: restoreOverrides
        ? { mode: prevMode, overrides: prevOverrides }
        : { mode: prevMode },
    });
    } catch {}
  }

  async function setManualDuty(duty: number) {
    await patch({
      fan: {
        mode: "manual",
        manual: { duty_pct: Math.max(0, Math.min(100, Math.round(duty))) },
      },
    });
  }

  function stdev(values: number[]): number {
    if (!values.length) return Number.POSITIVE_INFINITY;
    let sum = 0;
    for (const v of values) sum += v;
    const mean = sum / values.length;
    let varSum = 0;
    for (const v of values) {
      const d = v - mean;
      varSum += d * d;
    }
    return Math.sqrt(varSum / values.length);
  }

  function median(values: number[]): number {
    if (!values.length) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    return sorted[Math.floor(sorted.length / 2)];
  }

  async function readStableRpms(): Promise<number[]> {
    const SETTLE_MS = 1000;
    const WINDOW = 8;
    const STDEV_MAX = 30;
    const POLL_MS = 500;
    const TIMEOUT_MS = 15000;
    const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

    const bufs: number[][] = [];
    const started = performance.now();
    let pendingAccept = false;

    await sleep(SETTLE_MS);

    while (performance.now() - started < TIMEOUT_MS) {
      if (cancelled) return bufs.map(median);
      try {
        const res = await DefaultService.getThermal();
        const rpms = (res.fans ?? []).map((f) => f.rpm);
        while (bufs.length < rpms.length) bufs.push([]);
        for (let i = 0; i < rpms.length; i++) {
          bufs[i].push(rpms[i]);
          if (bufs[i].length > WINDOW) bufs[i].shift();
        }
      } catch {}

      const ready =
        bufs.length > 0 &&
        bufs.every((b) => b.length >= WINDOW && stdev(b) <= STDEV_MAX);
      if (ready) {
        if (pendingAccept) return bufs.map(median);
        pendingAccept = true;
      } else {
        pendingAccept = false;
      }

      await sleep(pendingAccept ? SETTLE_MS : POLL_MS);
    }

    return bufs.map(median);
  }

  async function start() {
    cancelled = false;
    progress = 20;
    info = "Starting calibration";
    try {
      const config = get(configStore).config;
      if (config?.fan?.mode) {
        prevMode = config.fan.mode;
      }
      prevOverrides = config?.fan?.overrides ?? [];
      restoreOverrides = true;
      await patch({
        fan: { mode: "manual", overrides: [] },
      });
    } catch {}
    const duties = [100, 80, 60, 40, 20];
    const perFan: [number, number][][] = [];
    for (let i = 0; i < duties.length; i++) {
      if (cancelled) {
        return;
      }
      const d = duties[i];
      info = `Your fan is calibrating, please wait... `;
      await setManualDuty(d);
      if (cancelled) {
        return;
      }
      const rpms = await readStableRpms();
      while (perFan.length < rpms.length) perFan.push([]);
      for (let f = 0; f < rpms.length; f++) {
        perFan[f].push([d, rpms[f]]);
      }
      progress = Math.round(((i + 2) / duties.length) * 100);
    }
    const fans: CalibrationFan[] = perFan.map((pts, index) => {
      const points: [number, number][] = [...pts, [0, 0]];
      points.sort((a, b) => a[0] - b[0]);
      return { index, points };
    });
    info = "Saving";
    try {
      await patch({
        fan: {
          calibration: {
            updated_at: Math.floor(Date.now() / 1000),
            fans,
          },
        },
      });
    } catch {}
    dispatch("done", fans);
  }

  function cancel() {
    cancelled = true;
  }

  function onStart() {
    hasStarted = true;
    info = 'Starting calibration';
    start().finally(async () => {
      await restoreFanState();
      if (cancelled) {
        dispatch('cancel');
      }
    });
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
  <div class="card bg-base-200 p-5 w-[460px] shadow-xl">
    {#if !hasStarted}
      <div class="space-y-3">
        <div class="font-semibold">Calibrate to enable Live RPM</div>
        <div class="text-sm opacity-80">
          To display the live RPM overlay accurately, we need to measure how each fan's speed (RPM) maps to duty percentage. This takes about a minute and will briefly spin the fans at different speeds.
        </div>
        <ul class="list-disc list-inside text-sm opacity-70">
          <li>Your current fan settings will be restored after calibration.</li>
          <li>You can cancel at any time.</li>
        </ul>
        <div class="mt-4 flex items-center justify-end gap-2">
          <button class="btn btn-sm" on:click={() => dispatch('cancel')}>Cancel</button>
          <button
            class="btn btn-sm btn-primary"
            on:click={onStart}
          >
            Start calibration
          </button>
        </div>
      </div>
    {:else}
      <div class="flex items-center justify-between mb-2">
        <div class="font-semibold">Calibrating fans</div>
      </div>
      <div class="text-sm opacity-80 mb-3">
        {info}
        {$animatedProgress.toFixed(0)}%
      </div>
      <progress class="progress w-full" value={$animatedProgress} max="100"
      ></progress>
      <div class="mt-3 flex items-center justify-end gap-2">
        <button class="btn btn-sm" on:click={cancel}>Cancel</button>
      </div>
    {/if}
  </div>
</div>
