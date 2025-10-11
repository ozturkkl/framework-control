<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { DefaultService } from "./api";
  import DeviceHeader from "./components/DeviceHeader.svelte";
  import FanControl from "./components/FanControl.svelte";
  import PowerControl from "./components/PowerControl.svelte";
  import Telemetry from "./components/Telemetry.svelte";
  import Panel from "./components/Panel.svelte";
  import { OpenAPI } from "./api";
  import VersionMismatchModal from "./components/VersionMismatchModal.svelte";
  import { gtSemver } from "./lib/semver";

  let healthy: boolean = false;
  let cliPresent: boolean = true;
  let fanMode: "Auto" | "Manual" | "Curve" = "Auto";
  const installerUrl: string = import.meta.env?.VITE_INSTALLER_URL || "";

  let pollId: ReturnType<typeof setInterval> | null = null;

  // Hosted vs embedded detection
  const apiOrigin = new URL(OpenAPI.BASE || "/api", window.location.href)
    .origin;
  const isHosted = window.location.origin !== apiOrigin;

  // Service update info for mismatch gate
  let serviceCurrentVersion: string | null = null;
  let serviceLatestVersion: string | null = null;
  let showMismatchGate = false;

  // Helper to keep layout logic readable without changing values
  function panelGridClasses(
    pid: string,
    isHealthy: boolean,
    mode: "Auto" | "Manual" | "Curve"
  ): string {
    if (!isHealthy) return "md:col-span-4";
    if (!cliPresent)
      return "md:col-span-4 opacity-50 pointer-events-none select-none";
    if (pid === "telemetry")
      return mode === "Curve"
        ? "md:col-start-1 md:col-span-5 md:row-start-1 md:row-span-1"
        : "md:col-start-1 md:col-span-6 md:row-start-1 md:row-span-3";
    if (pid === "power")
      return mode === "Curve"
        ? "md:col-start-1 md:col-span-5 md:row-start-2 md:row-span-1"
        : "md:col-start-7 md:col-span-6 md:row-start-2 md:row-span-2";
    if (pid === "fan")
      return mode === "Curve"
        ? "md:col-start-6 md:col-span-7 md:row-start-1 md:row-span-2"
        : "md:col-start-7 md:col-span-6 md:row-start-1 md:row-span-1";
    return "md:col-span-4";
  }
  onMount(async () => {
    await pollHealthOnce();
    pollId = setInterval(async () => {
      await pollHealthOnce();
    }, 1000);
    // One-shot update check to decide mismatch gating (ignore paused setting)
    try {
      const res = await DefaultService.checkUpdate();
      serviceCurrentVersion =
        (res.current_version ?? null)?.toString().trim() || null;
      serviceLatestVersion =
        (res.latest_version ?? null)?.toString().trim() || null;
      const updateAvailable =
        serviceCurrentVersion && serviceLatestVersion
          ? gtSemver(serviceLatestVersion, serviceCurrentVersion)
          : false;
      showMismatchGate = isHosted && updateAvailable;
    } catch {}
  });

  async function pollHealthOnce() {
    try {
      const res = await DefaultService.health();
      healthy = true;
      cliPresent = res.cli_present;
    } catch {
      healthy = false;
    }
  }
  onDestroy(() => {
    if (pollId) clearInterval(pollId);
  });
</script>

<main class="min-h-screen flex items-center justify-center px-6 py-12">
  <div
    class="w-full max-w-6xl mx-auto space-y-4"
    inert={showMismatchGate}
    aria-hidden={showMismatchGate}
  >
    <section in:fade={{ duration: 200 }}>
      <DeviceHeader {healthy} {installerUrl} {cliPresent} />
    </section>

    <section class="grid gap-4 md:grid-cols-12 md:auto-rows-fr">
      {#each ["telemetry", "power", "fan"] as pid (pid)}
        <div
          animate:flip={{ duration: 200 }}
          class={"col-span-12 " + panelGridClasses(pid, healthy, fanMode)}
        >
          {#if pid === "telemetry"}
            <Panel title="Telemetry" expandable={healthy}>
              {#if healthy && cliPresent}
                <Telemetry />
              {:else}
                <div class="text-sm opacity-80">
                  Live temps and fan RPM read locally via the service. Nothing
                  leaves your machine.
                </div>
                <ul class="list-disc list-inside text-sm opacity-80 space-y-1">
                  <li>Temps, power, and fan RPM</li>
                  <li>AC/battery status and basic battery info</li>
                  <li>Time‑series charts & live updates</li>
                </ul>
              {/if}
            </Panel>
          {:else if pid === "power"}
            <Panel title="Power" expandable={healthy}>
              {#if healthy && cliPresent}
                <PowerControl />
              {:else}
                <div class="text-sm opacity-80">
                  Quick view of charger state and battery health at a glance,
                  powered by the Framework CLI. And RyzenAdj.
                </div>
                <ul class="list-disc list-inside text-sm opacity-80 space-y-1">
                  <li>TDP and thermal limit controls</li>
                  <li>Battery details like cycle count</li>
                  <li>Future: configurable charge rate/current limits</li>
                </ul>
              {/if}
            </Panel>
          {:else}
            <Panel title="Fan Control" expandable={healthy}>
              <div
                slot="header"
                class:hidden={!healthy}
                class="flex items-center gap-2"
              >
                <div class="join border border-primary/35">
                  <input
                    type="radio"
                    name="fan-mode"
                    aria-label="Auto"
                    class="btn btn-xs join-item"
                    value="Auto"
                    on:change={() => (fanMode = "Auto")}
                    checked={fanMode === "Auto"}
                  />
                  <input
                    type="radio"
                    name="fan-mode"
                    aria-label="Manual"
                    class="btn btn-xs join-item"
                    value="Manual"
                    on:change={() => (fanMode = "Manual")}
                    checked={fanMode === "Manual"}
                  />
                  <input
                    type="radio"
                    name="fan-mode"
                    aria-label="Curve"
                    class="btn btn-xs join-item"
                    value="Curve"
                    on:change={() => (fanMode = "Curve")}
                    checked={fanMode === "Curve"}
                  />
                </div>
              </div>
              {#if healthy}
                <FanControl bind:mode={fanMode} />
              {:else}
                <div class="text-sm opacity-80">
                  Choose Auto, a fixed duty, or a saved curve with hysteresis
                  and rate‑limit. Settings persist and apply at boot.
                </div>
                <ul class="list-disc list-inside text-sm opacity-80 space-y-1">
                  <li>Modes: Auto, Manual duty, or Curve</li>
                  <li>Piecewise points with hysteresis and rate limit</li>
                  <li>Config is persisted and applied at boot</li>
                </ul>
              {/if}
            </Panel>
          {/if}
        </div>
      {/each}
    </section>
  </div>
  {#if showMismatchGate}
    <VersionMismatchModal
      serviceCurrent={serviceCurrentVersion}
      serviceLatest={serviceLatestVersion}
      {apiOrigin}
      {installerUrl}
    />
  {/if}
</main>
