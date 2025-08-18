<script lang="ts">
  import { onMount } from "svelte";
  import { DefaultService } from "../api";
  import type { Config, FanMode, FanCurveConfig, PartialConfig } from "../api";
  import { throttleDebounce } from "../lib/utils";

  let error: string | null = null;
  let success: string | null = null;

  let mode: FanMode = "Auto";
  let manualDutyPct = 50;
  let sensor: string = "APU";
  let currentFanCurve: FanCurveConfig | null = null;

  onMount(async () => {
    try {
      const { ok, config } = await DefaultService.getConfig();
      if (ok && config?.fan_curve) {
        currentFanCurve = config.fan_curve;
        mode = currentFanCurve.mode ?? mode;
        sensor = currentFanCurve.sensor ?? sensor;
        if (typeof currentFanCurve.manual_duty_pct === "number") {
          manualDutyPct = currentFanCurve.manual_duty_pct;
        }
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    }
  });

  const save = throttleDebounce(
    async () => {
      error = null;
      success = null;
      // Merge changes into the current fan curve so we send a full value (backend replaces fully)
      const base: FanCurveConfig = currentFanCurve ?? {
        enabled: false,
        mode: "Auto",
        sensor: "APU",
        points: [
          [40, 0],
          [60, 40],
          [75, 80],
          [85, 100],
        ],
        poll_ms: 2000,
        hysteresis_c: 2,
        rate_limit_pct_per_step: 100,
        manual_duty_pct: undefined,
      };
      const updated: FanCurveConfig = {
        ...base,
        enabled: mode !== "Auto",
        mode,
        sensor,
        manual_duty_pct: mode === "Manual" ? manualDutyPct : undefined,
      };
      const patch: PartialConfig = { fan_curve: updated };
      try {
        const res = await DefaultService.setConfig(patch);
        if (!res.ok) throw new Error("Failed to save config");
        currentFanCurve = updated;
        success = "Saved";
        setTimeout(() => (success = null), 1500);
      } catch (e: unknown) {
        error = e instanceof Error ? e.message : String(e);
      }
    },
    200,
    false,
    true
  );
</script>

<div class="space-y-4 relative">
  {#if error}
    <div class="alert alert-error text-sm">
      <span>{error}</span>
    </div>
  {/if}
  <span
    class="pointer-events-none select-none absolute top-2 right-2 inline-flex items-center justify-center w-6 h-6 rounded-full bg-green-500 text-white shadow transition duration-200 ease-out"
    style="opacity: {success ? 1 : 0}; transform: scale({success ? 1 : 0.9});"
    aria-hidden="true"
  >
    ✓
  </span>

  <div class="form-control">
    <div class="join">
      <input
        type="radio"
        name="mode"
        aria-label="Auto"
        class="btn join-item"
        value="auto"
        on:change={() => {
          mode = "Auto";
          save();
        }}
        checked={mode === "Auto"}
      />
      <input
        type="radio"
        name="mode"
        aria-label="Manual"
        class="btn join-item"
        value="manual"
        on:change={() => {
          mode = "Manual";
          save();
        }}
        checked={mode === "Manual"}
      />
      <input
        type="radio"
        name="mode"
        aria-label="Curve"
        class="btn join-item"
        value="curve"
        on:change={() => {
          mode = "Curve";
          save();
        }}
        checked={mode === "Curve"}
      />
    </div>
  </div>

  {#if mode === "Manual"}
    <div class="form-control">
      <label class="label" for="manual-duty">
        <span class="label-text">Manual duty: {manualDutyPct}%</span>
      </label>
      <input
        id="manual-duty"
        type="range"
        min="0"
        max="100"
        step="1"
        class="range"
        bind:value={manualDutyPct}
        on:input={save}
      />
    </div>
  {/if}

  {#if mode === "Curve"}
    <div class="text-sm opacity-80">Curve editor coming soon.</div>

    <div class="form-control">
      <label class="label" for="sensor-select">
        <span class="label-text">Sensor</span>
      </label>
      <select
        id="sensor-select"
        class="select select-bordered max-w-xs"
        bind:value={sensor}
        on:change={save}
      >
        <option value="APU">APU</option>
        <option value="CPU">CPU</option>
      </select>
    </div>
  {/if}
</div>

<style>
</style>
