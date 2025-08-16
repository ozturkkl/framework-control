<script lang="ts">
  import Telemetry from './routes/Telemetry.svelte';
  import Fan from './routes/Fan.svelte';
  import { onMount } from 'svelte';
  import { checkHealth } from './lib/api';

  let route: 'telemetry' | 'fan' = 'telemetry';
  function nav(to: typeof route) {
    route = to;
    history.replaceState({}, '', `#${to}`);
  }
  const hash = location.hash.slice(1);
  if (hash === 'fan') route = 'fan';
  window.addEventListener('hashchange', () => {
    const h = location.hash.slice(1);
    if (h === 'fan' || h === 'telemetry') route = h as any;
  });

  let serviceHealthy: boolean | null = null;
  const installerUrl: string = (import.meta as any).env?.VITE_INSTALLER_URL || '';

  onMount(async () => {
    serviceHealthy = await checkHealth();
  });
</script>

<header>
  <nav>
    <a href="#telemetry" class:active={route==='telemetry'} on:click={(e)=>{e.preventDefault();nav('telemetry')}}>Telemetry</a>
    <a href="#fan" class:active={route==='fan'} on:click={(e)=>{e.preventDefault();nav('fan')}}>Fan</a>
  </nav>
  <div>Framework Control</div>
  </header>

<main>
  {#if serviceHealthy === false}
    <div class="banner">
      <div>
        Framework Control service is not reachable on this device.
        {#if installerUrl}
          <a href={installerUrl}>Download the Windows installer (MSI)</a> and run it, then refresh this page.
        {:else}
          Please install the service and refresh this page. (Configure <code>VITE_INSTALLER_URL</code> to show a download link.)
        {/if}
      </div>
    </div>
  {/if}
  <section>
    {#if route==='telemetry'}
      <Telemetry />
    {:else}
      <Fan />
    {/if}
  </section>
</main>


