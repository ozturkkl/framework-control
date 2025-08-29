<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import ShortcutInstaller from './ShortcutInstaller.svelte';
  const dispatch = createEventDispatcher();
  function close() { dispatch('close'); }
</script>

<div class="modal modal-open">
  <div class="modal-box max-w-2xl">
    <h3 class="font-bold text-lg">Settings</h3>
    <div class="divider my-2"></div>
    <div class="space-y-4">
      <section class="flex items-center justify-between gap-4">
        <div>
          <h4 class="font-semibold">Desktop Shortcuts</h4>
          <p class="text-xs opacity-70">Create Start Menu and Desktop shortcuts for quick access</p>
        </div>
        <ShortcutInstaller />
      </section>
    </div>
    <div class="modal-action">
      <button class="btn" on:click={close}>Close</button>
    </div>
  </div>
  <button class="modal-backdrop" on:click={close} aria-label="Close settings"></button>
</div>


