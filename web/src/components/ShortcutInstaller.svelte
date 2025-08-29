<script lang="ts">
  import { onMount } from 'svelte';
  import { DefaultService } from '../api';
  import Icon from '@iconify/svelte';
  
  let shortcutsInstalled = false;
  let checking = true;
  let installing = false;
  let error = '';
  
  async function checkStatus() {
    try {
      const data = await DefaultService.getShortcutsStatus();
      shortcutsInstalled = data.installed || false;
    } catch {
      shortcutsInstalled = false;
    } finally {
      checking = false;
    }
  }
  
  async function installShortcuts() {
    installing = true;
    error = '';
    try {
      // Reuse the OpenAPI.TOKEN if present
      const auth = (await import('../api')).OpenAPI.TOKEN;
      const result = await DefaultService.createShortcuts(auth ? `Bearer ${auth}` : '');
      if (result.ok) {
        shortcutsInstalled = true;
      } else {
        error = 'Failed to create shortcuts';
      }
    } catch (e) {
      error = 'Error creating shortcuts';
      console.error('Failed to create shortcuts:', e);
    } finally {
      installing = false;
    }
  }
  
  onMount(() => { checkStatus(); });
</script>

<div class="flex items-center justify-end py-1">
  <button
    class={
      (!checking && shortcutsInstalled)
        ? 'btn btn-ghost btn-sm text-success gap-2'
        : 'btn btn-sm btn-primary'
    }
    on:click={installShortcuts}
    disabled={installing}
  >
    {#if !checking && shortcutsInstalled}
      <Icon icon="mdi:check" class="w-4 h-4" />
      Installed
    {:else}
      {installing ? 'Installing...' : 'Install'}
    {/if}
  </button>
</div>
{#if error}
  <div class="text-error text-xs mt-1">{error}</div>
{/if}
