<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { DefaultService } from "./api";
  import DeviceHeader from "./components/DeviceHeader.svelte";
  import FanControl from "./components/FanControl.svelte";
  import PowerControl from "./components/PowerControl.svelte";
  import Sensors from "./components/Sensors.svelte";
  import Panel from "./components/Panel.svelte";
  import { OpenAPI } from "./api";
  import VersionMismatchModal from "./components/VersionMismatchModal.svelte";
  import { gtSemver } from "./lib/semver";

  let healthy: boolean = false;
  let cliPresent: boolean = true;
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
    <section>
      <DeviceHeader {healthy} {installerUrl} {cliPresent} />
    </section>

    <section
      class={"flex flex-wrap " +
        (healthy ? "items-start" : "items-stretch") +
        " gap-4"}
    >
      {#each ["telemetry", "fan", "power"] as pid (pid)}
        <div
          class={"w-full " +
            (healthy
              ? "lg:w-[calc(50%-0.51rem)]"
              : "lg:w-[calc(33.333%-0.667rem)]")}
        >
          {#if pid === "telemetry"}
            <Panel title="Sensors" expandable={healthy}>
              {#if healthy && cliPresent}
                <Sensors />
              {:else}
                <div class="flex-1 flex flex-col justify-evenly px-3 pb-1">
                  <div class="text-sm opacity-80">
                    Local sensor data and fan RPM are read by the service.
                  </div>
                  <ul
                    class="list-disc list-inside text-sm opacity-80 space-y-1"
                  >
                    <li>Temperature sensors and fan RPM</li>
                    <li>Historical charts with live updates</li>
                    <li>The sensor history window is adjustable</li>
                  </ul>
                </div>
              {/if}
            </Panel>
          {:else if pid === "fan"}
            <Panel title="Fan Control" expandable={healthy}>
              {#if healthy}
                <FanControl />
              {:else}
                <div class="flex-1 flex flex-col justify-evenly px-3 pb-1">
                  <div class="text-sm opacity-80">
                    Choose Auto, a fixed duty, or a saved curve with hysteresis
                    and rate‑limit. Settings persist and apply at boot.
                  </div>
                  <ul
                    class="list-disc list-inside text-sm opacity-80 space-y-1"
                  >
                    <li>Modes: Auto, Manual duty, or Curve</li>
                    <li>Piecewise points with hysteresis and rate limit</li>
                    <li>Config is persisted and applied at boot</li>
                  </ul>
                </div>
              {/if}
            </Panel>
          {:else if pid === "power"}
            <Panel title="Power" expandable={healthy}>
              {#if healthy && cliPresent}
                <PowerControl />
              {:else}
                <div class="flex-1 flex flex-col justify-evenly px-3 pb-1">
                  <div class="text-sm opacity-80">
                    Quick view of charger state and battery health at a glance,
                    powered by the Framework CLI. And RyzenAdj.
                  </div>
                  <ul
                    class="list-disc list-inside text-sm opacity-80 space-y-1"
                  >
                    <li>TDP and thermal limit controls</li>
                    <li>Battery details like cycle count</li>
                    <li>Future: configurable charge rate/current limits</li>
                  </ul>
                </div>
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
