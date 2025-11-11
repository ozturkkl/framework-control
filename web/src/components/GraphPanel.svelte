<script lang="ts">
  import Icon from "@iconify/svelte";
  import { measureHeight } from "../lib/measureHeight";

  // Internal settings state managed by wrapper; children open/close via slot props
  let showSettings = false;
  function openSettings() {
    showSettings = true;
  }
  function closeSettings() {
    showSettings = false;
  }

  // Keep settings view height equal to the graph content height
  let contentHeight: number | null = null;
  function setMeasuredHeight(h: number) {
    contentHeight = h;
  }
</script>

<div class="card bg-base-200 p-3">
  {#if !showSettings}
    <div use:measureHeight={{ onChange: setMeasuredHeight }}>
      <div class="flex items-center justify-between mb-2 gap-2">
        <slot name="top" {openSettings} {closeSettings} />
      </div>
      <div class="relative">
        <div class="w-full relative">
          <slot name="graph" />
        </div>
        <div class="mt-2">
          <slot name="bottom" />
        </div>
      </div>
    </div>
  {:else}
    <div
      class="h-full overflow-y-auto overflow-x-hidden"
      style={`height:${contentHeight ? contentHeight + "px" : "auto"}`}
    >
      <div class="min-h-full flex flex-col">
        <div
          class="sticky top-0 z-10 bg-base-200 flex items-center justify-between pb-1 gap-2"
        >
          <button
            class="btn btn-xs btn-ghost gap-1"
            on:click={closeSettings}
            aria-label="Back to graph"
          >
            <Icon icon="mdi:arrow-left" class="text-base" />
            Back
          </button>
          <slot name="settings-top-right" {openSettings} {closeSettings} />
        </div>
        <slot name="settings" {openSettings} {closeSettings} />
      </div>
    </div>
  {/if}
</div>
