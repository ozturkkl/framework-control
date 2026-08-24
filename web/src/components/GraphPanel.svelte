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

<div class="card bg-base-200 p-3 h-full min-h-0 flex flex-col flex-1 w-full">
  {#if !showSettings}
    <div
      class="h-full min-h-0 flex flex-col flex-1"
      use:measureHeight={{ onChange: setMeasuredHeight }}
    >
      <div class="flex items-center justify-between mb-2 gap-2 shrink-0">
        <slot name="top" {openSettings} {closeSettings} />
      </div>
      <div class="relative flex-1 min-h-0 flex flex-col">
        <div class="relative w-full flex-1 min-h-[220px]">
          <slot name="graph" />
        </div>
        {#if $$slots.bottom}
          <div class="mt-2 shrink-0">
            <slot name="bottom" />
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div
      class="h-full min-h-0 flex-1 overflow-y-auto overflow-x-hidden"
      style={contentHeight ? `min-height:${contentHeight}px` : undefined}
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
