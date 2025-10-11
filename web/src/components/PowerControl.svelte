<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    DefaultService,
    OpenAPI,
    type PowerConfig,
    type PowerProfile,
    type PartialConfig,
  } from "../api";
  import Icon from "@iconify/svelte";
  import { deepMerge } from "../lib/utils";
  import UiSlider from "./UiSlider.svelte";

  const TDP_MIN = 5;
  const TDP_MAX = 120;
  const TDP_BATTERY_MAX = 60;

  // Basic CPU vendor detection to gate AMD-only controls
  let isIntel: boolean = false;
  let detectedCpu: string | null = null;

  let activeProfile: keyof PowerConfig = "ac";
  const ACTIVE_PROFILE_KEY = "fc.power.activeProfile";
  function setActiveProfile(profile: keyof PowerConfig) {
    activeProfile = profile;
    try {
      localStorage.setItem(ACTIVE_PROFILE_KEY, profile);
    } catch (_) {}
  }

  let installingRyzenAdj: boolean = false;
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

  async function setPower(
    profile: keyof PowerConfig,
    field: keyof PowerProfile,
    enabled: boolean,
    value: number
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

  async function getRyzenAdjInstalled() {
    try {
      const resp = await DefaultService.getPower();
      ryzenInstalled = !!resp.ryzenadj_installed;
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
        true
      );
    } catch {}
    await getRyzenAdjInstalled();
    infoPoll = setInterval(getRyzenAdjInstalled, 2000);
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
      await getRyzenAdjInstalled();
    } catch (e) {
      errorMessage = "Failed to install, check your antivirus settings!";
    } finally {
      installingRyzenAdj = false;
    }
  }

  function onToggleTdp(ev: Event) {
    const value = powerConfig[activeProfile].tdp_watts.value;
    const enabled = powerConfig[activeProfile].tdp_watts.enabled;
    setPower(activeProfile, "tdp_watts", enabled, value);
  }
  function onChangeTdp(ev: Event) {
    const value = powerConfig[activeProfile].tdp_watts.value;
    const enabled = powerConfig[activeProfile].tdp_watts.enabled;
    setPower(activeProfile, "tdp_watts", enabled, value);
  }
  function onToggleThermal(ev: Event) {
    const value = powerConfig[activeProfile].thermal_limit_c.value;
    const enabled = powerConfig[activeProfile].thermal_limit_c.enabled;
    setPower(activeProfile, "thermal_limit_c", enabled, value);
  }
  function onChangeThermal(ev: Event) {
    const value = powerConfig[activeProfile].thermal_limit_c.value;
    const enabled = powerConfig[activeProfile].thermal_limit_c.enabled;
    setPower(activeProfile, "thermal_limit_c", enabled, value);
  }

  function onInputTdp(ev: Event) {
    if (activeProfile === "battery") {
      const current = powerConfig[activeProfile].tdp_watts.value;
      if (current > TDP_BATTERY_MAX) {
        powerConfig[activeProfile].tdp_watts.value = TDP_BATTERY_MAX;
      }
    }
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
  class="absolute top-[1.4rem] left-24 right-14 flex items-center justify-start gap-2 text-sm"
>
  {#if hasCheckedInstallStatus && ryzenInstalled}
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
  {/if}
</div>

<div class="my-auto">
  {#if isIntel}
    <div>
      <h3 class="text-lg font-bold mb-2 text-center">
        Intel systems not yet supported
      </h3>
      <div class="text-sm opacity-80 text-center">
        Power controls are currently available only on AMD Ryzen systems via
        RyzenAdj. Your CPU appears to be{#if detectedCpu}: <b>{detectedCpu}</b
          >{/if}.
      </div>
    </div>
  {:else if !hasCheckedInstallStatus}
    <div>
      <h3 class="text-lg font-bold mb-2 text-center">Checking requirements…</h3>
      <div class="flex items-center justify-center gap-2 text-sm opacity-80">
        <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
        <span>Detecting current power helper status</span>
      </div>
    </div>
  {:else if !ryzenInstalled}
    <div>
      <h3 class="text-lg font-bold mb-2 text-center">Enable power controls</h3>
      <ul class="list-disc pl-5 text-sm space-y-1 opacity-80">
        <li>This requires a small helper to be installed.</li>
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
      class="grid gap-3 [grid-template-columns:repeat(auto-fit,minmax(18rem,1fr))] pt-2"
    >
      <div
        class="transition-transform duration-100"
        class:scale-[0.985]={!powerConfig[activeProfile].tdp_watts.enabled}
      >
        <UiSlider
          label="TDP Limit"
          icon={activeProfile === "ac" ? "mdi:power-plug-outline" : "mdi:battery-outline"}
          unit="W"
          min={TDP_MIN}
          max={TDP_MAX}
          step={1}
          hasEnabled={true}
          bind:enabled={powerConfig[activeProfile].tdp_watts.enabled}
          capMax={activeProfile === "battery" ? TDP_BATTERY_MAX : null}
          bind:value={powerConfig[activeProfile].tdp_watts.value}
          on:input={onInputTdp}
          on:change={onChangeTdp}
        />
      </div>
      <div
        class="transition-transform duration-100"
        class:scale-[0.985]={!powerConfig[activeProfile].thermal_limit_c.enabled}
      >
        <UiSlider
          label="Thermal Limit"
          icon={activeProfile === "ac" ? "mdi:power-plug-outline" : "mdi:battery-outline"}
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
