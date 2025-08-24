<script lang="ts">
  import { DefaultService } from "../api";
  import type { PartialConfig } from "../api";
  import { parseThermalOutput } from "../lib/thermal";
  import { createEventDispatcher } from "svelte";
  import { tweened } from "svelte/motion";
  import { get } from "svelte/store";

  export let token: string;
  const dispatch = createEventDispatcher();

  let progress = 0; // 0..100
  let info = "";
  let cancelled = false;
  let hasStarted = false;

  let prevMode: "manual" | "curve" | "disabled" = "curve";

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

  async function setMode(mode: "manual" | "curve" | "disabled") {
    const patch: PartialConfig = { fan: { mode } };
    try {
      await DefaultService.setConfig(token, patch);
    } catch {}
  }

  async function setManualDuty(duty: number) {
    const patch: PartialConfig = {
      fan: {
        mode: "manual",
        manual: { duty_pct: Math.max(0, Math.min(100, Math.round(duty))) },
      },
    };
    await DefaultService.setConfig(token, patch);
  }

  async function readStableRpm(): Promise<number> {
    // Simple sliding window with deviation check and a hard timeout
    const WINDOW = 5;
    const STDEV_MAX = 30; // accept when std-dev is small
    const POLL_MS = 500;
    const TIMEOUT_MS = 10000;

    const buf: number[] = [];
    const started = performance.now();

    while (performance.now() - started < TIMEOUT_MS) {
      if (cancelled) return 0;
      try {
        const res = await DefaultService.getThermal();
        if (res.ok) {
          const { rpms } = parseThermalOutput(res.stdout);
          const rpm = rpms.length ? Math.max(...rpms) : 0;
          if (rpm > 0) {
            buf.push(rpm);
            if (buf.length > WINDOW) buf.shift();
          }
        }
      } catch {}

      if (buf.length >= WINDOW) {
        let sum = 0;
        for (const v of buf) sum += v;
        const mean = sum / buf.length;
        let varSum = 0;
        for (const v of buf) {
          const d = v - mean;
          varSum += d * d;
        }
        const stdev = Math.sqrt(varSum / buf.length);
        console.log("stdev", stdev);
        if (stdev <= STDEV_MAX) {
          const sorted = [...buf].sort((a, b) => a - b);
          console.log("stdev is small, sorted", sorted);
          console.log("median", sorted[Math.floor(sorted.length / 2)]);
          return sorted[Math.floor(sorted.length / 2)];
        }
      }

      await new Promise((r) => setTimeout(r, POLL_MS));
    }

    if (!buf.length) return 0;
    const sorted = [...buf].sort((a, b) => a - b);
    return sorted[Math.floor(sorted.length / 2)];
  }

  async function start() {
    cancelled = false;
    progress = 20;
    info = "Starting calibration";
    // Snapshot current mode so we can restore it later
    try {
      const { ok, config } = await DefaultService.getConfig();
      if (ok && config?.fan?.mode) {
        prevMode = config.fan.mode;
      }
    } catch {}
    const duties = [100, 80, 60, 40, 20];
    const out: [number, number][] = [];
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
      const rpm = await readStableRpm();
      out.push([d, rpm]);
      progress = Math.round(((i + 2) / duties.length) * 100);
    }
    out.push([0, 0]);
    out.sort((a, b) => a[0] - b[0]);
    info = "Saving";
    // Save calibration at root; include mode to satisfy backend schema
    try {
      await DefaultService.setConfig(token, {
        fan: {
          calibration: {
            points: out,
            updated_at: Math.floor(Date.now() / 1000),
          },
        },
      });
    } catch {}
    dispatch("done", out);
  }

  function cancel() {
    cancelled = true;
  }

  function onStart() {
    hasStarted = true;
    info = 'Starting calibration';
    start().finally(() => {
      setMode(prevMode);
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
          To display the live RPM overlay accurately, we need to measure how your fan speed (RPM) maps to duty percentage. This takes about a minute and will briefly spin the fan at different speeds.
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
        <div class="font-semibold">Calibrating fan</div>
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
