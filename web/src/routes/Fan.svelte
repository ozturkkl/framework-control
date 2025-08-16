<script lang="ts">
  import { setFanDuty } from '../lib/api';
  let percent = 30;
  let busy = false;
  let message: string | null = null;

  async function apply(){
    busy = true; message = null;
    try{ await setFanDuty(percent, null); message = 'Applied'; }
    catch(e:any){ message = e.message || String(e); }
    finally{ busy = false; }
  }
</script>

<h2>Fan Control</h2>
<div style="display:flex;gap:16px;align-items:center">
  <input type="range" min="0" max="100" bind:value={percent} />
  <input type="number" min="0" max="100" bind:value={percent} />
  <button disabled={busy} on:click={apply}>Set Duty</button>
  {#if message}<span>{message}</span>{/if}
</div>


