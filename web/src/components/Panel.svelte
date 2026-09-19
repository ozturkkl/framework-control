<script context="module" lang="ts">
  export const PANEL_HEADER_OVERLAY_CLASS =
    "panel-header-overlay absolute top-[0.62rem] right-2 z-10 flex items-center justify-end gap-2 text-sm";
  export const PANEL_HEADER_TOGGLE_CLASS =
    "join border border-primary/35 shrink-0 max-w-full whitespace-nowrap";
</script>

<script lang="ts">
  import Icon from "@iconify/svelte";
  import { createEventDispatcher } from "svelte";
  import { dragHandle } from "svelte-dnd-action";

  export let title: string;
  export let className: string = "";
  export let editing: boolean = false;
  export let enabled: boolean = true;
  export let size: "half" | "full" = "half";
  export let canMovePrev: boolean = false;
  export let canMoveNext: boolean = false;

  const dispatch = createEventDispatcher<{
    disable: void;
    enable: void;
    toggleSize: void;
    movePrev: void;
    moveNext: void;
  }>();

  const headerRowClass = "flex items-center justify-between gap-2 mb-1";
  const headerLeftClass =
    "flex flex-1 items-center flex-wrap gap-x-3 gap-y-1 min-w-0";
  const bodyClass = "flex flex-col p-2 h-full";
  const editBtnClass = "btn btn-ghost btn-xs";
</script>

<div
  class={`card relative h-full ${enabled ? "bg-base-100 shadow" : "bg-base-200/40 shadow-none border border-dashed border-base-content/20 min-h-32"} ${className}`}
  class:panel-editing={editing}
>
  <div class={bodyClass}>
    <div class="{headerRowClass} {editing ? 'pr-1' : 'pl-3'}">
      <div
        use:dragHandle
        class="flex flex-1 items-center self-stretch min-w-0 {editing
          ? 'cursor-grab gap-1 pl-1.5 select-none'
          : ''}"
        aria-label={editing ? `Drag to reorder ${title}` : undefined}
        title={editing ? `Drag to reorder ${title}` : undefined}
      >
        {#if editing}
          <Icon
            icon="mdi:drag-horizontal-variant"
            class="w-8 h-5 shrink-0 opacity-40"
          />
        {/if}
        <div class={headerLeftClass}>
          <h2 class="card-title pb-0.5 shrink-0" class:opacity-60={!enabled}>
            {title}
          </h2>
          {#if !enabled}
            <span class="badge badge-ghost badge-sm opacity-60">Disabled</span>
          {/if}
          {#if enabled}
            <slot name="header" />
          {/if}
        </div>
      </div>
      {#if editing}
        <div class="flex items-center gap-0.5 shrink-0">
          <button
            type="button"
            class={editBtnClass}
            aria-label="Move {title} earlier"
            title="Move earlier"
            disabled={!canMovePrev}
            on:click={() => dispatch("movePrev")}
          >
            <Icon icon="mdi:chevron-up" class="text-base" />
          </button>
          <button
            type="button"
            class={editBtnClass}
            aria-label="Move {title} later"
            title="Move later"
            disabled={!canMoveNext}
            on:click={() => dispatch("moveNext")}
          >
            <Icon icon="mdi:chevron-down" class="text-base" />
          </button>
          <button
            type="button"
            class={editBtnClass}
            aria-label={size === "full"
              ? `Use half width for ${title}`
              : `Use full width for ${title}`}
            title={size === "full" ? "Half width" : "Full width"}
            on:click={() => dispatch("toggleSize")}
          >
            <Icon
              icon={size === "full"
                ? "mdi:arrow-collapse-horizontal"
                : "mdi:arrow-expand-horizontal"}
              class="text-base"
            />
          </button>
          {#if enabled}
            <button
              type="button"
              class={editBtnClass}
              aria-label="Disable {title}"
              title="Disable"
              on:click={() => dispatch("disable")}
            >
              <Icon icon="mdi:eye-off-outline" class="text-base" />
            </button>
          {:else}
            <button
              type="button"
              class={editBtnClass}
              aria-label="Enable {title}"
              title="Enable"
              on:click={() => dispatch("enable")}
            >
              <Icon icon="mdi:eye-outline" class="text-base" />
            </button>
          {/if}
        </div>
      {/if}
    </div>
    {#if enabled}
      <slot />
    {/if}
  </div>
</div>

<style>
  .panel-editing :global(.panel-header-overlay) {
    display: none;
  }
</style>
