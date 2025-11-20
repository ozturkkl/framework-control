<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Icon from "@iconify/svelte";
  import UiSlider from "./UiSlider.svelte";
  import { tooltip } from "../lib/tooltip";
  import {
    type BatteryInfo,
    DefaultService,
    OpenAPI,
    type PowerResponse,
    type PartialConfig,
    type Config,
  } from "../api";
  import { throttleDebounce } from "../lib/utils";

  // Polling
  let poll: ReturnType<typeof setInterval> | null = null;
  let errorMessage: string | null = null;

  // Battery info
  let batteryInfo: BatteryInfo | undefined;

  // Derived display values for info bar
  $: isCharging = !!batteryInfo?.charging;
  $: acPresent = !!batteryInfo?.ac_present;
  $: presentWatts =
    batteryInfo?.present_rate_ma != null &&
    batteryInfo?.present_voltage_mv != null
      ? (batteryInfo.present_rate_ma * batteryInfo.present_voltage_mv) /
        1_000_000
      : undefined;
  $: cRate = (() => {
    if (!batteryInfo?.charger_current_ma || !batteryInfo.design_capacity_mah)
      return undefined;
    const value =
      batteryInfo.charger_current_ma / batteryInfo.design_capacity_mah;
    return value.toFixed(2);
  })();
  $: clMax = batteryInfo?.charge_limit_max_pct ?? undefined;
  $: chargerWatts =
    batteryInfo?.charge_input_current_ma != null &&
    batteryInfo?.charger_voltage_mv != null
      ? (batteryInfo.charge_input_current_ma * batteryInfo.charger_voltage_mv) /
        1_000_000
      : undefined;
  $: chargerRequestedWatts =
    batteryInfo?.charger_current_ma != null &&
    batteryInfo?.charger_voltage_mv != null
      ? (batteryInfo.charger_current_ma * batteryInfo.charger_voltage_mv) /
        1_000_000
      : undefined;

  function formatEtaMinutes(totalMinutes: number): string {
    if (!isFinite(totalMinutes) || totalMinutes <= 0) return "—";
    const m = Math.round(totalMinutes);
    const days = Math.floor(m / 1440);
    const hours = Math.floor((m % 1440) / 60);
    const mins = m % 60;
    if (days > 0) {
      // cap to two units
      return hours > 0 ? `${days}d ${hours}h` : `${days}d`;
    }
    if (hours > 0) {
      return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
    }
    return `${mins}m`;
  }

  // ETA (right side): to target when charging, to empty when discharging
  $: targetPct = clMax ?? 100;
  $: etaToTargetMinutes = (() => {
    if (!batteryInfo?.present_rate_ma || batteryInfo.present_rate_ma <= 0) {
      return undefined;
    }
    if (!isCharging) return undefined;
    const lfcc = batteryInfo.last_full_charge_capacity_mah;
    const currentPct = batteryInfo.percentage;
    const remainingMah = batteryInfo.remaining_capacity_mah;
    if (lfcc != null && currentPct != null) {
      const deltaPct = Math.max(0, targetPct - currentPct);
      const deltaMah = (lfcc * deltaPct) / 100;
      if (deltaMah <= 0) return 0;
      const hours = deltaMah / batteryInfo.present_rate_ma;
      return hours * 60;
    }
    if (lfcc != null && remainingMah != null) {
      const targetMah = Math.max(0, Math.floor((lfcc * targetPct) / 100));
      const deltaMah = Math.max(0, targetMah - remainingMah);
      if (deltaMah <= 0) return 0;
      const hours = deltaMah / batteryInfo.present_rate_ma;
      return hours * 60;
    }
    return undefined;
  })();
  $: etaToTargetWhileDischargingMinutes = (() => {
    if (!batteryInfo?.present_rate_ma || batteryInfo.present_rate_ma <= 0) {
      return undefined;
    }
    if (isCharging) return undefined;
    const lfcc = batteryInfo.last_full_charge_capacity_mah;
    const currentPct = batteryInfo.percentage;
    const remainingMah = batteryInfo.remaining_capacity_mah;
    if (lfcc != null && currentPct != null && currentPct > targetPct) {
      const deltaPct = currentPct - targetPct;
      const deltaMah = (lfcc * deltaPct) / 100;
      if (deltaMah <= 0) return 0;
      const hours = deltaMah / batteryInfo.present_rate_ma;
      return hours * 60;
    }
    if (
      lfcc != null &&
      remainingMah != null &&
      Math.round((remainingMah * 100) / lfcc) > targetPct
    ) {
      const targetMah = Math.max(0, Math.floor((lfcc * targetPct) / 100));
      const deltaMah = Math.max(0, remainingMah - targetMah);
      if (deltaMah <= 0) return 0;
      const hours = deltaMah / batteryInfo.present_rate_ma;
      return hours * 60;
    }
    return undefined;
  })();
  $: etaToEmptyMinutes = (() => {
    if (!batteryInfo?.present_rate_ma || batteryInfo.present_rate_ma <= 0) {
      return undefined;
    }
    if (isCharging) return undefined;
    const remainingMah = batteryInfo.remaining_capacity_mah;
    if (remainingMah == null) return undefined;
    const hours = remainingMah / batteryInfo.present_rate_ma;
    return hours * 60;
  })();

  // UI controls
  let clEnabled = false;
  const CL_MIN = 25;
  const CL_MAX = 100;
  let clValue: number = 100;

  // Rate limit (Advanced) inline controls
  let rateC: number = 1; // 0.00 - 1.00 C
  let socThresholdPct: number | undefined;
  let socChipBtn: HTMLElement | null = null;
  let socPopoverVisible = false;
  let rateEnabled: boolean = false;

  async function pollOnce() {
    try {
      const resp: PowerResponse = await DefaultService.getPower();
      batteryInfo = resp.battery;
      if (!batteryInfo) {
        errorMessage = "Failed to get battery info";
        return;
      }
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  async function applyChargeLimitConfig() {
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      const patch: PartialConfig = {
        battery: {
          charge_limit_max_pct: {
            enabled: !!clEnabled,
            value: Math.max(CL_MIN, Math.min(CL_MAX, clValue)),
          },
        },
      };
      await DefaultService.setConfig(auth, patch);
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  async function applyRateLimitConfig() {
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      const value = rateEnabled
        ? Math.max(0, Math.min(1.0, Math.round(rateC * 20) / 20))
        : 1.0;
      const patch: PartialConfig = {
        battery: {
          charge_rate_c: {
            enabled: !!rateEnabled,
            value,
          },
          charge_rate_soc_threshold_pct: socThresholdPct,
        },
      };
      await DefaultService.setConfig(auth, patch);
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  const rateLimitChange = throttleDebounce(
    applyRateLimitConfig,
    300,
    false,
    true
  );
  const chargeLimitChange = throttleDebounce(
    applyChargeLimitConfig,
    300,
    false,
    true
  );

  onMount(async () => {
    // Seed from config
    try {
      const cfg: Config = await DefaultService.getConfig();
      const bat = cfg.battery;
      if (bat) {
        if (bat.charge_limit_max_pct) {
          clEnabled = !!bat.charge_limit_max_pct.enabled;
          clValue = bat.charge_limit_max_pct.value ?? clValue;
        }
        if (bat.charge_rate_c) {
          rateEnabled = !!bat.charge_rate_c.enabled;
          rateC = bat.charge_rate_c.value ?? rateC;
        }
        socThresholdPct = bat.charge_rate_soc_threshold_pct ?? undefined;
      }
    } catch (_) {}
    await pollOnce();
    poll = setInterval(pollOnce, 2000);
  });
  onDestroy(() => {
    if (poll) clearInterval(poll);
  });

  $: if (errorMessage) {
    setTimeout(() => {
      errorMessage = null;
    }, 5000);
  }
</script>

<!-- Overlay summary (matches PowerControl height/spacing) -->
<div class="bg-base-200 min-w-0 rounded-xl mb-2">
  <div class="py-2 px-3 flex items-center justify-between text-xs">
    <div class="flex items-center gap-2">
      <span class="inline-flex items-center gap-1">
        <Icon
          icon={isCharging ? "mdi:battery-charging" : "mdi:flash-outline"}
          class={`w-4 h-4 ${isCharging ? "text-success" : "text-secondary"}`}
        />
        <span class="tabular-nums text-xs"
          >{isCharging ? "+" : "-"}{presentWatts != null
            ? (Math.round(presentWatts * 100) / 100).toFixed(2)
            : "—"} W</span
        >
        <span class="opacity-60">{isCharging ? "charge" : "discharge"}</span>
      </span>
      {#if acPresent}
        <span class="opacity-60">•</span>
        <span class="inline-flex items-center gap-1">
          <Icon icon="mdi:speedometer" class="w-4 h-4" />
          <span class="tabular-nums text-xs">{cRate ?? "—"} C</span>
        </span>
        <span class="opacity-60">•</span>
        <span class="inline-flex items-center gap-1">
          <Icon icon="mdi:power-plug-outline" class="w-4 h-4" />
          <span class="tabular-nums text-xs"
            >{chargerRequestedWatts != null
              ? Math.round(chargerRequestedWatts)
              : "—"}/{chargerWatts != null ? Math.round(chargerWatts) : "—"} W</span
          >
        </span>
      {/if}

      <span class="opacity-60">•</span>
      <span class="inline-flex items-center gap-1">
        <Icon icon="mdi:battery-charging-90" class="w-4 h-4" />
        <span class="tabular-nums text-xs"
          >{clMax != null ? clMax : "—"}% max</span
        >
      </span>
    </div>
    <div class="text-[10px] flex items-center gap-1">
      <Icon icon="mdi:clock-outline" class="w-4 h-4" />
      {#if acPresent && isCharging}
        <span class="tabular-nums"
          >{etaToTargetMinutes != null
            ? formatEtaMinutes(etaToTargetMinutes)
            : "—"}</span
        >
        <span class="opacity-60">to {targetPct}%</span>
      {:else if !acPresent}
        <span class="tabular-nums"
          >{etaToEmptyMinutes != null
            ? formatEtaMinutes(etaToEmptyMinutes)
            : "—"}</span
        >
        <span class="opacity-60">to empty</span>
      {:else if batteryInfo?.percentage != null && clMax != null && Math.abs(batteryInfo.percentage - clMax) > 1}
        <span class="tabular-nums"
          >{etaToTargetWhileDischargingMinutes != null
            ? formatEtaMinutes(etaToTargetWhileDischargingMinutes)
            : "—"}</span
        >
        <span class="opacity-60">to {clMax}%</span>
      {/if}
    </div>
  </div>
  <UiSlider
    label="Max Charge Limit"
    icon="mdi:battery-heart"
    unit="%"
    min={CL_MIN}
    max={CL_MAX}
    step={1}
    hasEnabled={true}
    bind:enabled={clEnabled}
    bind:value={clValue}
    on:change={chargeLimitChange}
  />

  <UiSlider
    label="Rate (C) (Advanced)"
    icon="mdi:flash-outline"
    unit="C"
    min={0}
    max={1}
    step={0.05}
    hasEnabled={true}
    bind:enabled={rateEnabled}
    bind:value={rateC}
    on:change={rateLimitChange}
  >
    <svelte:fragment slot="header-trailing">
      <button
        class="badge badge-ghost badge-sm select-none"
        class:opacity-60={!rateEnabled}
        disabled={!rateEnabled}
        aria-label="Set state of charge threshold"
        title="Set state of charge threshold"
        bind:this={socChipBtn}
        on:click={() => (socPopoverVisible = !socPopoverVisible)}
      >
        {#if socThresholdPct != null}
          ≤ SoC {socThresholdPct}%
        {:else}
          SoC
        {/if}
      </button>
    </svelte:fragment>
  </UiSlider>
  <div
    use:tooltip={{
      anchor: socChipBtn,
      visible: socPopoverVisible,
      onDismiss: () => (socPopoverVisible = false),
    }}
    class="bg-base-100 border border-base-300 rounded shadow p-2 w-44"
    role="dialog"
    aria-label="Set SoC threshold"
    tabindex="-1"
  >
    <div class="text-[10px] opacity-70 mb-1">≤ State of Charge (%)</div>
    <div class="flex items-center gap-2">
      <input
        class="input input-xs input-bordered w-16"
        type="number"
        min="0"
        max="100"
        step="1"
        bind:value={socThresholdPct}
        on:input={rateLimitChange}
        on:change={rateLimitChange}
      />
      <div class="flex gap-1">
        {#each [60, 70, 80, 90] as p}
          <button
            class="badge badge-outline badge-xs cursor-pointer"
            on:click={() => {
              socThresholdPct = p;
              rateLimitChange();
            }}
            aria-label={`Set ${p}%`}>{p}</button
          >
        {/each}
      </div>
    </div>
    <div class="mt-1 text-right">
      <button
        class="btn btn-ghost btn-xs px-1"
        on:click={() => {
          socThresholdPct = undefined;
          rateLimitChange();
          socPopoverVisible = false;
        }}
      >
        Clear
      </button>
    </div>
  </div>
  {#if errorMessage}
    <div class="text-[10px] text-error mt-2">{errorMessage}</div>
  {/if}
</div>

<style>
  .tabular-nums {
    font-variant-numeric: tabular-nums;
  }
</style>
