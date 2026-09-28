<script context="module" lang="ts">
  export const PANEL_HEADER_OVERLAY_CLASS =
    "panel-header-overlay absolute top-[0.62rem] right-2 z-10 flex items-center justify-end gap-2 text-sm";
  export const PANEL_HEADER_TOGGLE_CLASS =
    "join border surface-border shrink-0 max-w-full whitespace-nowrap";
</script>

<script lang="ts">
  import Icon from "@iconify/svelte";
  import { createEventDispatcher, tick } from "svelte";
  import { fade } from "svelte/transition";
  import { dragHandle } from "svelte-dnd-action";

  export let title: string;
  export let className: string = "";
  export let editing: boolean = false;
  export let enabled: boolean = true;
  export let size: "half" | "full" = "half";
  export let canMovePrev: boolean = false;
  export let canMoveNext: boolean = false;
  export let live: boolean = true;

  const dispatch = createEventDispatcher<{
    disable: void;
    enable: void;
    toggleSize: void;
    movePrev: void;
    moveNext: void;
  }>();

  let cardEl: HTMLDivElement | undefined;
  let isExpanded = false;
  $: canExpand = enabled && live && !editing;
  $: if (isExpanded && !canExpand) isExpanded = false;
  $: minHeightClass = live ? "min-h-[24rem]" : "";

  const headerRowClass = "flex items-center justify-between gap-2 mb-1";
  const headerLeftClass =
    "flex flex-1 items-center flex-wrap gap-x-3 gap-y-1 min-w-0";
  const bodyClass = "flex flex-col p-2 h-full";
  const editBtnClass = "btn btn-ghost btn-xs";
  const enabledCardClass = "bg-base-100 shadow";
  const disabledCardClass =
    "bg-base-200/40 shadow-none border border-dashed surface-border";
  const expandedCardClass =
    "panel-expanded fixed z-50 inset-0 m-auto flex flex-col";

  function playFlip(from: DOMRect) {
    if (!cardEl || window.matchMedia("(prefers-reduced-motion: reduce)").matches)
      return;
    const to = cardEl.getBoundingClientRect();
    if (to.width < 1 || to.height < 1) return;
    cardEl.style.zIndex = "50";
    const animation = cardEl.animate(
      [
        {
          transform: `translate(${from.left - to.left}px, ${from.top - to.top}px) scale(${from.width / to.width}, ${from.height / to.height})`,
        },
        { transform: "none" },
      ],
      { duration: 200, easing: "ease" },
    );
    void animation.finished.finally(() => {
      if (cardEl && !cardEl.getAnimations().length) cardEl.style.zIndex = "";
    });
  }

  async function setExpanded(next: boolean) {
    if (next === isExpanded || !cardEl) return;
    const from = cardEl.getBoundingClientRect();
    isExpanded = next;
    await tick();
    playFlip(from);
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (!isExpanded || e.key !== "Escape") return;
    e.preventDefault();
    void setExpanded(false);
  }
</script>

<svelte:window on:keydown={onWindowKeydown} />

<div class="h-full" class:panel-has-maximize={canExpand}>
  {#if isExpanded}
    <button
      type="button"
      class="fixed inset-0 z-40 bg-black/50"
      aria-label="Close overlay"
      transition:fade={{ duration: 150 }}
      on:click={() => setExpanded(false)}
    ></button>
  {/if}
  <div
    bind:this={cardEl}
    class="card origin-top-left {isExpanded
      ? expandedCardClass
      : 'relative h-full'} {enabled
      ? enabledCardClass
      : disabledCardClass} {minHeightClass} {className}"
    class:panel-editing={editing}
  >
    {#if canExpand}
      <div class="absolute top-[0.62rem] right-2 z-10">
        <button
          type="button"
          class={editBtnClass}
          aria-label={isExpanded ? `Restore ${title}` : `Maximize ${title}`}
          title={isExpanded ? "Restore" : "Maximize"}
          aria-expanded={isExpanded}
          on:click={() => setExpanded(!isExpanded)}
        >
          <Icon
            icon={isExpanded ? "mdi:arrow-collapse" : "mdi:arrow-expand"}
            class="text-base"
          />
        </button>
      </div>
    {/if}
    <div class="{bodyClass} {isExpanded ? 'min-h-0' : ''}">
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
        <div
          class="flex flex-col flex-1 min-h-0 {isExpanded
            ? 'overflow-y-auto'
            : ''}"
        >
          <slot />
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .panel-expanded {
    width: min(90vw, calc(90vh * 16 / 9));
    height: min(90vh, calc(90vw * 6 / 5));
  }

  :global(html:has(.panel-expanded)) {
    overflow: hidden;
  }

  .panel-editing :global(.panel-header-overlay) {
    display: none;
  }

  .panel-has-maximize :global(.panel-header-overlay) {
    right: 3rem;
  }
</style>
