<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    DefaultService,
    OpenAPI,
    type PowerConfig,
    type PowerProfile,
    type PartialConfig,
    type BatteryInfo,
  } from "../api";
  import Icon from "@iconify/svelte";
  import { deepMerge } from "../lib/utils";
  import UiSlider from "./UiSlider.svelte";
  import { tooltip } from "../lib/tooltip";
  import { isLinux as getIsLinux } from "../lib/platform";

  const TDP_MIN = 5;
  const TDP_SAFE_AC_MAX = 120;
  const TDP_ABSOLUTE_AC_MAX = 145;
  const TDP_SAFE_BATTERY_MAX = 60;
  const TDP_ABSOLUTE_BATTERY_MAX = 120;
  const POWER_INFO_CONTAINER_CLASS =
    "flex flex-col h-44 my-0.5 px-6 justify-center gap-2";

  // Basic CPU vendor + OS detection to gate controls (header is always shown)
  let isIntel: boolean = false;
  let isLinux: boolean = getIsLinux();
  let detectedCpu: string | null = null;

  // Controls are only available when not Intel and not Linux (for now)
  $: controlsSupported = !isIntel && !isLinux;

  // Header actions + TDP/Thermal readouts should only show when the helper is installed
  $: showPowerValues =
    controlsSupported && hasCheckedInstallStatus && ryzenInstalled;

  let activeProfile: keyof PowerConfig = "ac";
  const ACTIVE_PROFILE_KEY = "fc.power.activeProfile";
  function setActiveProfile(profile: keyof PowerConfig) {
    activeProfile = profile;
    try {
      localStorage.setItem(ACTIVE_PROFILE_KEY, profile);
    } catch (_) {}
  }

  let installingRyzenAdj: boolean = false;
  let uninstallingRyzenAdj: boolean = false;
  let errorMessage: string | null = null;
  let ryzenInstalled: boolean = false;
  let infoPoll: ReturnType<typeof setInterval> | null = null;
  let agreed: boolean = false;
  let hasCheckedInstallStatus: boolean = false;
  let powerConfig = {
    ac: {
      tdp_watts: {
        enabled: false,
        value: 75,
      },
      thermal_limit_c: {
        enabled: false,
        value: 100,
      },
    },
    battery: {
      tdp_watts: {
        enabled: false,
        value: 60,
      },
      thermal_limit_c: {
        enabled: false,
        value: 90,
      },
    },
  };

  let currentTdp: number | undefined;
  let currentThermal: number | undefined;
  let acPresent: boolean | undefined;
  let batteryPct: number | undefined;
  let chargerWatts: number | undefined;
  let chargerRequestedWatts: number | undefined;
  let removeBtn: HTMLButtonElement;
  let removeTipVisible = false;
  let unlockBtn: HTMLButtonElement;
  let unlockTipVisible = false;
  let highTdpUnlocked = false;

  function recomputeHighTdpUnlocked() {
    const acVal = powerConfig.ac.tdp_watts.value;
    const batVal = powerConfig.battery.tdp_watts.value;
    highTdpUnlocked =
      (acVal != null && acVal > TDP_SAFE_AC_MAX) ||
      (batVal != null && batVal > TDP_SAFE_BATTERY_MAX);
  }

  async function setPower(
    profile: keyof PowerConfig,
    field: keyof PowerProfile,
    enabled: boolean,
    value: number,
  ) {
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      const patch: PartialConfig = {
        power: {
          [profile]: {
            [field]: { enabled, value },
          },
        },
      };
      await DefaultService.setConfig(auth, patch);
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  function updateChargerWattage(bat: BatteryInfo | undefined) {
    if (
      bat?.charge_input_current_ma != null &&
      bat.charger_voltage_mv != null
    ) {
      chargerWatts =
        (bat.charge_input_current_ma * bat.charger_voltage_mv) / 1_000_000;
    } else {
      chargerWatts = undefined;
    }
    if (bat?.charger_current_ma != null && bat.charger_voltage_mv != null) {
      chargerRequestedWatts =
        (bat.charger_current_ma * bat.charger_voltage_mv) / 1_000_000;
    } else {
      chargerRequestedWatts = undefined;
    }
  }

  async function pollPower() {
    try {
      const resp = await DefaultService.getPower();
      ryzenInstalled = !!resp.ryzenadj_installed;

      const bat = resp.battery;
      acPresent = bat?.ac_present;
      batteryPct = bat?.percentage;
      updateChargerWattage(bat);

      const tdp = resp.tdp_watts;
      const therm = resp.thermal_limit_c;
      currentTdp = tdp && tdp > 0 ? tdp : undefined;
      currentThermal = therm && therm > 0 ? therm : undefined;
    } catch (_) {
      ryzenInstalled = false;
    } finally {
      hasCheckedInstallStatus = true;
    }
  }

  onMount(async () => {
    hasCheckedInstallStatus = false;
    try {
      const sys = await DefaultService.getSystemInfo();
      detectedCpu = sys?.cpu || "";
      isIntel = detectedCpu.toLowerCase().includes("intel");
    } catch (_) {
      isIntel = false;
    }

    try {
      const saved = localStorage.getItem(ACTIVE_PROFILE_KEY);
      if (saved === "ac" || saved === "battery") {
        activeProfile = saved;
      }
    } catch (_) {}
    try {
      // Fetch config once at init and seed UI immediately
      const cfg = await DefaultService.getConfig();
      powerConfig = deepMerge(
        powerConfig,
        cfg.power as typeof powerConfig,
        true,
      );
      recomputeHighTdpUnlocked();
    } catch {}
    await pollPower();
    infoPoll = setInterval(pollPower, 2000);
  });
  onDestroy(() => {
    if (infoPoll) clearInterval(infoPoll);
  });

  async function installRyzenAdj() {
    if (!agreed) return;
    installingRyzenAdj = true;
    errorMessage = null;
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      await DefaultService.installRyzenadj(auth);
      // getInstalled a couple times to see if the installed will turn true
      for (let i = 0; i < 5; i++) {
        await pollPower();
        if (ryzenInstalled) break;
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    } catch (e) {
      errorMessage = "Failed to install, check your antivirus settings!";
    } finally {
      installingRyzenAdj = false;
    }
  }

  async function uninstallRyzenAdj() {
    uninstallingRyzenAdj = true;
    errorMessage = null;
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      await DefaultService.uninstallRyzenadj(auth);
      await pollPower();
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    } finally {
      uninstallingRyzenAdj = false;
    }
  }

  function onChangeTdp(ev: Event) {
    const value = powerConfig[activeProfile].tdp_watts.value;
    const enabled = powerConfig[activeProfile].tdp_watts.enabled;
    setPower(activeProfile, "tdp_watts", enabled, value);
  }
  function onChangeThermal(ev: Event) {
    const value = powerConfig[activeProfile].thermal_limit_c.value;
    const enabled = powerConfig[activeProfile].thermal_limit_c.enabled;
    setPower(activeProfile, "thermal_limit_c", enabled, value);
  }
</script>

<!-- Preload icons to avoid first-switch layout shift -->
<div aria-hidden="true" class="absolute opacity-0 pointer-events-none -z-10">
  <Icon icon="mdi:power-plug-outline" class="w-3.5 h-3.5" />
  <Icon icon="mdi:battery-outline" class="w-3.5 h-3.5" />
  <!-- Keeping them mounted ensures the icon set is loaded before first toggle -->
  <!-- This container is invisible and out of flow -->
  <!-- Do not remove -->
</div>

<!-- Overlay status positioned into the parent header area -->
<div
  class="absolute top-[0.62rem] left-24 right-11 flex items-center justify-between gap-2 text-sm"
>
  {#if showPowerValues}
    <div class="join border border-primary/35">
      <input
        type="radio"
        name="power-profile"
        aria-label="Plugged in"
        class="btn btn-xs join-item"
        value="ac"
        on:change={() => setActiveProfile("ac")}
        checked={activeProfile === "ac"}
      />
      <input
        type="radio"
        name="power-profile"
        aria-label="On battery"
        class="btn btn-xs join-item"
        value="battery"
        on:change={() => setActiveProfile("battery")}
        checked={activeProfile === "battery"}
      />
    </div>
    <div>
      <button
        class="btn btn-ghost btn-xs"
        aria-label={highTdpUnlocked
          ? "Disable high TDP values"
          : "Unlock higher TDP values"}
        bind:this={unlockBtn}
        on:mouseenter={() => (unlockTipVisible = true)}
        on:mouseleave={() => (unlockTipVisible = false)}
        on:focus={() => (unlockTipVisible = true)}
        on:blur={() => (unlockTipVisible = false)}
        on:click={() => (highTdpUnlocked = !highTdpUnlocked)}
      >
        <Icon
          icon={highTdpUnlocked
            ? "mdi:lock-open-variant-outline"
            : "mdi:lock-outline"}
          class="w-3.5 h-3.5"
        />
      </button>
      <button
        class="btn btn-ghost btn-xs"
        aria-label="Remove helper"
        bind:this={removeBtn}
        on:mouseenter={() => (removeTipVisible = true)}
        on:mouseleave={() => (removeTipVisible = false)}
        on:focus={() => (removeTipVisible = true)}
        on:blur={() => (removeTipVisible = false)}
        on:click={uninstallRyzenAdj}
        disabled={uninstallingRyzenAdj}
      >
        {#if uninstallingRyzenAdj}
          <Icon icon="mdi:loading" class="w-3.5 h-3.5 animate-spin" />
        {:else}
          <Icon icon="mdi:trash-can-outline" class="w-3.5 h-3.5" />
        {/if}
      </button>
    </div>

    <div
      use:tooltip={{
        anchor: unlockBtn,
        visible: unlockTipVisible,
        attachGlobalDismiss: false,
      }}
      class="pointer-events-none bg-base-100 px-2 py-1 rounded border border-base-300 shadow text-xs text-center"
    >
      Unlock higher values for TDP.<br />
      <span class="opacity-90 text-error">USE AT YOUR OWN RISK.</span>
    </div>
    <div
      use:tooltip={{
        anchor: removeBtn,
        visible: removeTipVisible,
        attachGlobalDismiss: false,
      }}
      class="pointer-events-none bg-base-100 px-2 py-1 rounded border border-base-300 shadow text-xs w-60 text-center"
    >
      Remove the RyzenAdj helper. You can reinstall later from here.
    </div>
  {/if}
</div>

<div class="my-auto">
  <div
    class="bg-base-200 min-w-0 rounded-xl mb-2 py-2 px-3 flex items-center gap-2 text-xs"
  >
    <div
      class="flex flex-wrap items-center gap-x-2 gap-y-1 min-w-0 justify-center mr-auto"
    >
      {#if showPowerValues}
        <span class="inline-flex items-center gap-1 whitespace-nowrap">
          <Icon
            icon="mdi:flash-outline"
            class={`w-4 h-4 ${Number(currentTdp) > 95 ? "brightness-200" : Number(currentTdp) > 60 ? "brightness-150" : "brightness-100"} text-success`}
          />
          <span class="tabular-nums text-xs">{currentTdp ?? "—"} W</span>
        </span>
        <span class="inline-flex items-center gap-1 whitespace-nowrap">
          <span class="opacity-60">•</span>
          <Icon
            icon="mdi:thermometer"
            class={`w-4 h-4 ${Number(currentThermal) > 95 ? "text-error" : Number(currentThermal) > 90 ? "text-warning" : "text-success"}`}
          />
          <span class="tabular-nums text-xs">{currentThermal ?? "—"} °C</span>
        </span>
      {/if}

      {#if acPresent && showPowerValues}
        <span class="opacity-60">•</span>
      {/if}

      {#if acPresent}
        <span class="inline-flex items-center gap-1 whitespace-nowrap">
          <Icon icon="mdi:power-plug-outline" class="w-3.5 h-3.5" />
          <span class="tabular-nums text-xs"
            >{chargerRequestedWatts != null
              ? Math.round(chargerRequestedWatts)
              : "—"}/{chargerWatts != null ? Math.round(chargerWatts) : "—"}
            W</span
          >
        </span>
      {/if}
    </div>
    <div class="flex gap-x-2 gap-y-1 justify-end whitespace-nowrap">
      <span class={`inline-flex items-center gap-1 whitespace-nowrap`}>
        <Icon
          icon={acPresent ? "mdi:battery-charging" : "mdi:battery"}
          class={`w-3.5 h-3.5 ${acPresent ? "animate-pulse" : ""}  ${acPresent ? "text-success" : ""}`}
        />
        <span class="tabular-nums text-xs">{batteryPct ?? "—"}%</span>
      </span>
      <span class="opacity-60">•</span>
      <span
        class={`text-xs opacity-90 ${acPresent ? "text-success" : "text-secondary"}`}
        >{acPresent ? "Plugged in" : "On battery"}</span
      >
    </div>
  </div>

  {#if isLinux}
    <div class={POWER_INFO_CONTAINER_CLASS}>
      <h3 class="text-lg font-bold mb-2 text-center mt-2">
        Linux version coming soon…
      </h3>
      <div class="text-sm opacity-80 text-center mb-2">
        Power controls are not available on Linux yet. Please check back soon.
      </div>
    </div>
  {:else if isIntel}
    <div class={POWER_INFO_CONTAINER_CLASS}>
      <h3 class="text-lg font-bold mb-2 text-center mt-2">
        Intel systems not yet supported
      </h3>
      <div class="text-sm opacity-80 text-center mb-2">
        Power controls are currently available only on AMD Ryzen systems via
        RyzenAdj. Your CPU appears to be{#if detectedCpu}: <b>{detectedCpu}</b
          >{/if}.
      </div>
    </div>
  {:else if !hasCheckedInstallStatus}
    <div class={POWER_INFO_CONTAINER_CLASS}>
      <h3 class="text-lg font-bold mb-2 text-center">Checking requirements…</h3>
      <div class="flex items-center justify-center gap-2 text-sm opacity-80">
        <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
        <span>Detecting current power helper status</span>
      </div>
    </div>
  {:else if !ryzenInstalled}
    <div class={POWER_INFO_CONTAINER_CLASS}>
      <h3 class="text-lg font-bold mb-2 text-center">Enable power controls</h3>
      <ul class="list-disc text-sm space-y-1 list-inside opacity-80">
        <li>
          This requires a small helper <a
            href="https://github.com/FlyGoat/RyzenAdj"
            target="_blank"
            rel="noopener noreferrer"
            class="btn-link px-0">RyzenAdj</a
          > to be installed.
        </li>
        <li>May trigger antivirus warnings on your system.</li>
        <li>
          Adjusting power settings can cause instability and crashes and may
          even (though rarely) damage your hardware. We take no responsibility!
        </li>
      </ul>
      <div class="mt-1 flex items-center justify-between">
        <label class="label cursor-pointer justify-start gap-2">
          <input
            type="checkbox"
            class="checkbox checkbox-sm"
            bind:checked={agreed}
          />
          <span class="label-text text-sm"
            >I agree to the above and <span class="text-primary"
              >understand the risks!</span
            ></span
          >
        </label>
        <button
          class="btn btn-primary btn-sm"
          disabled={!agreed || installingRyzenAdj}
          on:click={installRyzenAdj}
        >
          {#if installingRyzenAdj}
            <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
            Installing...
          {:else}
            <Icon icon="mdi:download-outline" class="w-4 h-4" />
            Install
          {/if}
        </button>
      </div>
      {#if errorMessage}
        <div class="text-xs text-error">{errorMessage}</div>
      {/if}
    </div>
  {:else}
    <div
      class="grid gap-3 [grid-template-columns:repeat(auto-fit,minmax(18rem,1fr))]"
    >
      <div
        class="transition-transform duration-100"
        class:scale-[0.985]={!powerConfig[activeProfile].tdp_watts.enabled}
      >
        <UiSlider
          label="TDP Limit"
          icon={activeProfile === "ac"
            ? "mdi:power-plug-outline"
            : "mdi:battery-outline"}
          unit="W"
          min={TDP_MIN}
          max={highTdpUnlocked
            ? activeProfile === "ac"
              ? TDP_ABSOLUTE_AC_MAX
              : TDP_ABSOLUTE_BATTERY_MAX
            : activeProfile === "ac"
              ? TDP_SAFE_AC_MAX
              : TDP_SAFE_BATTERY_MAX}
          step={1}
          hasEnabled={true}
          bind:enabled={powerConfig[activeProfile].tdp_watts.enabled}
          capMax={activeProfile === "battery"
            ? TDP_SAFE_BATTERY_MAX
            : TDP_SAFE_AC_MAX}
          allowPassingCapMax={highTdpUnlocked}
          bind:value={powerConfig[activeProfile].tdp_watts.value}
          on:change={onChangeTdp}
        />
      </div>
      <div
        class="transition-transform duration-100"
        class:scale-[0.985]={!powerConfig[activeProfile].thermal_limit_c
          .enabled}
      >
        <UiSlider
          label="Thermal Limit"
          icon={activeProfile === "ac"
            ? "mdi:power-plug-outline"
            : "mdi:battery-outline"}
          unit="°C"
          min={50}
          max={100}
          step={1}
          hasEnabled={true}
          bind:enabled={powerConfig[activeProfile].thermal_limit_c.enabled}
          bind:value={powerConfig[activeProfile].thermal_limit_c.value}
          on:change={onChangeThermal}
        />
      </div>
    </div>
  {/if}
</div>
