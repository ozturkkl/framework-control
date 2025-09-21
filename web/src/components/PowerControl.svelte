<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { DefaultService, OpenAPI } from "../api";

  const dispatch = createEventDispatcher();

  let tdpWatts: number = 45;
  let thermalC: number = 90;
  let busy: boolean = false;
  let errorMessage: string | null = null;

  async function applyTdp() {
    busy = true;
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      const watts = Math.max(5, Math.min(120, Math.round(tdpWatts)));
      await DefaultService.setConfig(auth, {
        power: { tdp_watts: watts },
      });
      errorMessage = null;
      dispatch("applied", { kind: "tdp", value: watts });
    } catch (e) {
      errorMessage = "Failed to set TDP";
    } finally {
      busy = false;
    }
  }

  async function applyThermal() {
    busy = true;
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      const celsius = Math.max(50, Math.min(100, Math.round(thermalC)));
      await DefaultService.setConfig(auth, {
        power: { thermal_limit_c: celsius },
      });
      errorMessage = null;
      dispatch("applied", { kind: "thermal", value: celsius });
    } catch (e) {
      errorMessage = "Failed to set thermal limit";
    } finally {
      busy = false;
    }
  }
</script>

<div class="space-y-4">
  <div class="grid md:grid-cols-2 gap-4">
    <div class="card bg-base-200">
      <div class="card-body gap-3">
        <h3 class="card-title text-sm">TDP Limit (W)</h3>
        <input
          type="range"
          min="5"
          max="120"
          step="1"
          bind:value={tdpWatts}
          class="range range-sm"
        />
        <div class="flex items-center justify-between text-xs opacity-70">
          <span>{Math.round(tdpWatts)} W</span>
          <button
            class="btn btn-xs btn-primary"
            disabled={busy}
            on:click={applyTdp}>Apply</button
          >
        </div>
      </div>
    </div>
    <div class="card bg-base-200">
      <div class="card-body gap-3">
        <h3 class="card-title text-sm">Thermal Limit (°C)</h3>
        <input
          type="range"
          min="50"
          max="100"
          step="1"
          bind:value={thermalC}
          class="range range-sm"
        />
        <div class="flex items-center justify-between text-xs opacity-70">
          <span>{Math.round(thermalC)} °C</span>
          <button
            class="btn btn-xs btn-primary"
            disabled={busy}
            on:click={applyThermal}>Apply</button
          >
        </div>
      </div>
    </div>
  </div>
  <div class="card bg-base-200">
    <div class="card-body gap-3">
      <p class="text-xs opacity-70">
        Settings apply in real-time and may be reset by the OS after reboot.
      </p>
      {#if errorMessage}
        <div class="text-xs text-error">{errorMessage}</div>
      {/if}
    </div>
  </div>
</div>
