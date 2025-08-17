<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { checkHealth } from "./lib/api";
  import DeviceHeader from "./components/DeviceHeader.svelte";

  let serviceHealthy: boolean | null = null;
  const installerUrl: string =
    (import.meta as any).env?.VITE_INSTALLER_URL || "";

  let pollId: any = null;
  onMount(async () => {
    serviceHealthy = await checkHealth();
    pollId = setInterval(async () => {
      const ok = await checkHealth();
      serviceHealthy = ok;
    }, 4000);
  });
  onDestroy(() => {
    if (pollId) clearInterval(pollId);
  });
</script>

<main class="min-h-screen flex items-center justify-center px-6 py-12">
  <div class="w-full max-w-6xl mx-auto space-y-4">
    <section in:fade={{ duration: 200 }}>
      <DeviceHeader
        healthy={!!serviceHealthy}
        {installerUrl}
      />
    </section>

    <section class="grid gap-4 md:grid-cols-3">
      <div class={"card bg-base-100 shadow transition-all duration-500 "}>
        <div class="card-body flex flex-col justify-between">
          <h2 class="card-title">Telemetry</h2>
          <div class="text-sm opacity-80">
            Live temps and fan RPM read locally via the service. Nothing leaves
            your machine.
          </div>
          <ul class="list-disc list-inside text-sm opacity-80 space-y-1">
            <li>Temps and fan RPM</li>
            <li>AC/battery status and basic battery info</li>
            <li>Time‑series charts & live updates</li>
          </ul>
        </div>
      </div>
      <div class={"card bg-base-100 shadow transition-all duration-500 "}>
        <div class="card-body flex flex-col justify-between">
          <h2 class="card-title">Power</h2>
          <div class="text-sm opacity-80">
            Quick view of charger state and battery health at a glance, powered
            by the Framework CLI.
          </div>
          <ul class="list-disc list-inside text-sm opacity-80 space-y-1">
            <li>Charger presence, voltage/current, SoC</li>
            <li>Battery details like cycle count</li>
            <li>Future: configurable charge rate/current limits</li>
          </ul>
        </div>
      </div>
      <div class={"card bg-base-100 shadow transition-all duration-500 "}>
        <div class="card-body flex flex-col justify-between">
          <h2 class="card-title">Fan Control</h2>
          <div class="text-sm opacity-80">
            Choose Auto, a fixed duty, or a saved curve with hysteresis and
            rate‑limit. Settings persist and apply at boot.
          </div>
          <ul class="list-disc list-inside text-sm opacity-80 space-y-1">
            <li>Modes: Auto, Manual duty, or Curve</li>
            <li>Piecewise points with hysteresis and rate limit</li>
            <li>Config is persisted and applied at boot</li>
          </ul>
        </div>
      </div>
    </section>
  </div>
</main>
