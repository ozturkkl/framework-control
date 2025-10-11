<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Icon from "@iconify/svelte";
  import { DefaultService } from "../api";

  let pollId: ReturnType<typeof setInterval> | null = null;
  let currentTdp: number | undefined;
  let currentThermal: number | undefined;
  let acPresent: boolean | undefined;
  let batteryPct: number | undefined;

  async function pollPower() {
    try {
      const resp = await DefaultService.getPower();
      acPresent = resp.ac_present;
      batteryPct = resp.battery?.percentage;

      const tdp = resp.tdp_watts;
      const therm = resp.thermal_limit_c;
      currentTdp = tdp && tdp > 0 ? tdp : undefined;
      currentThermal = therm && therm > 0 ? therm : undefined;
    } catch {}
  }

  onMount(async () => {
    await pollPower();
    pollId = setInterval(pollPower, 2000);
  });
  onDestroy(() => {
    if (pollId) clearInterval(pollId);
  });
</script>

<div class="pt-2">
  <div class="bg-base-200 min-w-0 rounded-xl">
    <div class="py-2 px-3 flex items-center justify-between text-xs">
      <div class="flex items-center gap-2">
        <span class="inline-flex items-center gap-1">
          <Icon
            icon="mdi:flash-outline"
            class={`w-4 h-4 ${Number(currentTdp) > 95 ? "brightness-200" : Number(currentTdp) > 60 ? "brightness-150" : "brightness-100"} text-success`}
          />
          <span class="tabular-nums text-xs">{currentTdp ?? "—"} W</span>
        </span>
        <span class="opacity-60">•</span>
        <span class="inline-flex items-center gap-1">
          <Icon
            icon="mdi:thermometer"
            class={`w-4 h-4 ${Number(currentThermal) > 95 ? "text-error" : Number(currentThermal) > 90 ? "text-warning" : "text-success"}`}
          />
          <span class="tabular-nums text-xs">{currentThermal ?? "—"} °C</span>
        </span>
      </div>
      <div class="text-[10px] flex items-center gap-1">
        <span class={`inline-flex items-center gap-1`}>
          <Icon
            icon={acPresent ? "mdi:battery-charging" : "mdi:battery"}
            class={`w-3.5 h-3.5 ${acPresent ? "animate-pulse" : ""}  ${acPresent ? "text-success" : ""}`}
          />
          <span class="tabular-nums text-xs">{batteryPct ?? "—"}%</span>
        </span>
        <span class="text-md opacity-60">•</span>
        <span
          class={`text-xs opacity-90 ${acPresent ? "text-success" : "text-secondary"}`}
          >{acPresent ? "Plugged in" : "On battery"}</span
        >
      </div>
    </div>
  </div>
</div>
