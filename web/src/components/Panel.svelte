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
  const headerRowClass = "flex items-center justify-between mb-1 pl-3";
  const headerLeftClass = "flex items-center gap-3";
  const bodyClass = "flex flex-col p-2 h-full";
</script>

<div
  class={`card bg-base-100 shadow transition-all duration-300 h-full ${className}`}
>
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
  <div class="modal modal-open z-40">
    <div class="modal-box p-0 max-w-5xl">
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
    <button
      class="modal-backdrop"
      aria-label="Close overlay"
      on:click={() => (isExpanded = false)}
    ></button>
  </div>
{/if}
