<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import Icon from "@iconify/svelte";

  export let title: string;
  export let className: string = "";
  export let expandable: boolean = true;

  let isExpanded = false;

  function toggle() {
    isExpanded = !isExpanded;
  }

  // Shared classes to keep collapsed/expanded views consistent
  const headerRowClass = "flex items-center justify-between";
  const headerLeftClass = "flex items-center gap-3";
  const bodyClass = "card-body flex flex-col gap-3 p-5";
  $: collapsedCardClass = `card bg-base-100 shadow transition-all duration-300 h-full ${className}`;
  $: expandedCardClass = `card bg-base-100 shadow-xl w-full max-w-5xl ${className}`;
</script>

<div class={collapsedCardClass}>
  <div class={bodyClass}>
    <div class={headerRowClass}>
      <div class={headerLeftClass}>
        <h2 class="card-title pb-0.5">{title}</h2>
        <slot name="header" />
      </div>
      {#if expandable}
        <button
          class="btn btn-ghost btn-xs"
          aria-label="Maximize"
          on:click={toggle}
        >
          <Icon
            icon={isExpanded ? "mdi:arrow-collapse" : "mdi:arrow-expand"}
            class="text-base"
          />
        </button>
      {/if}
    </div>
    {#if !isExpanded}
      <slot />
    {/if}
  </div>
</div>

{#if isExpanded}
  <div class="fixed inset-0 z-40" aria-modal="true" role="dialog">
    <button
      class="absolute inset-0 bg-base-content/40"
      on:click={() => (isExpanded = false)}
      in:fade
      out:fade
      aria-label="Close overlay"
    ></button>
    <div
      class="absolute inset-0 flex items-center justify-center p-4 pointer-events-none"
    >
      <div
        class={`${expandedCardClass} pointer-events-auto`}
        in:scale={{ start: 0.95, duration: 150 }}
        out:scale={{ start: 1, duration: 150 }}
      >
        <div class={bodyClass}>
          <div class={headerRowClass}>
            <div class={headerLeftClass}>
              <h2 class="card-title pb-0.5">{title}</h2>
              <slot name="header" />
            </div>
            <button
              class="btn btn-ghost btn-xs"
              on:click={() => (isExpanded = false)}
              aria-label="Close"
            >
              <Icon icon="mdi:close" class="text-base" />
            </button>
          </div>
          <slot />
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
</style>
