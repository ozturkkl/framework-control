<script lang="ts">
  import { getPower, type PowerResponse } from '../lib/api';
  let power: PowerResponse | null = null;
  let error: string | null = null;

  async function refresh(){
    error = null;
    try{ power = await getPower(); }
    catch(e:any){ error = e.message || String(e); }
  }
  refresh();
  const t = setInterval(refresh, 1000);
  import { onDestroy } from 'svelte';
  onDestroy(()=>clearInterval(t));
</script>

<h2>Power</h2>
{#if error}
  <p style="color:#ff6b6b">{error}</p>
{:else if power}
  <div>
    <div>AC: {power.ac_present ? 'Connected' : 'Not connected'}</div>
    {#if power.battery}
      <div>Battery: {power.battery.charge_percentage}% {power.battery.charging ? '(charging)' : ''}</div>
      <div>Cycle count: {power.battery.cycle_count}</div>
    {:else}
      <div>No battery info</div>
    {/if}
  </div>
{:else}
  <div>Loading…</div>
{/if}


