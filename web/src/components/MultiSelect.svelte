<script lang="ts">
  import { createEventDispatcher, tick } from "svelte";

  export let items: string[] = [];
  export let selected: string[] = [];
  export let label: string = "Select";
  // Unique prefix per component instance to avoid ID collisions across multiple MultiSelects
  const instanceId = crypto.randomUUID();
  const buttonId = `ms-btn-${instanceId}`;
  const menuId = `ms-menu-${instanceId}`;

  const dispatch = createEventDispatcher<{ change: string[] }>();
  let isOpen = false;
  let rootEl: HTMLDivElement;
  let menuEl: HTMLDivElement;
  let alignEnd = true; // true → right aligned (`dropdown-end`), false → left aligned
  let alignmentDone = false;

  function setSelected(item: string, checked: boolean) {
    const set = new Set(selected);
    if (checked) set.add(item);
    else set.delete(item);
    selected = Array.from(set);
    dispatch("change", selected);
  }

  // Reactive lookup for performant + reactive checkbox states
  let selectedSet: Set<string> = new Set(selected);
  $: selectedSet = new Set(selected);

  function onCheckboxChange(item: string, event: Event) {
    const target = event.target as HTMLInputElement;
    setSelected(item, !!target?.checked);
  }

  async function onButtonClick() {
    isOpen = !isOpen;
    if (isOpen) {
      await tick();
      realignDropdown();
    }
  }

  function onWindowClick(e: MouseEvent) {
    const path = (e.composedPath && e.composedPath()) || [];
    if (!path.includes(rootEl)) {
      isOpen = false;
    }
  }

  function realignDropdown() {
    try {
      if (!rootEl || !menuEl) return;
      const trigger = rootEl.getBoundingClientRect();
      const menuWidth = menuEl.offsetWidth || 0;
      const vw = Math.max(
        document.documentElement.clientWidth,
        window.innerWidth || 0
      );

      // Predict edges for both alignments
      const leftAlign = { left: trigger.left, right: trigger.left + menuWidth };
      const rightAlign = {
        left: trigger.right - menuWidth,
        right: trigger.right,
      };

      // Choose alignment that keeps the menu fully within viewport if possible
      const leftFits = leftAlign.left >= 0 && leftAlign.right <= vw;
      const rightFits = rightAlign.left >= 0 && rightAlign.right <= vw;

      if (rightFits) {
        alignEnd = true;
      } else if (leftFits) {
        alignEnd = false;
      }
    } catch {}

    alignmentDone = true;
  }

  function selectAll(event?: Event) {
    event?.stopPropagation?.();
    selected = Array.from(new Set(items));
    dispatch("change", selected);
  }

  function clearAll(event?: Event) {
    event?.stopPropagation?.();
    selected = [];
    dispatch("change", selected);
  }
</script>

<svelte:window
  on:click={onWindowClick}
  on:keydown={(e) => e.key === "Escape" && (isOpen = false)}
  on:resize={() => isOpen && realignDropdown()}
/>

<div
  class="dropdown dropdown-top"
  class:dropdown-open={isOpen}
  class:dropdown-end={alignEnd}
  bind:this={rootEl}
>
  <button
    class="btn btn-xs btn-ghost gap-1 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-base-content/80 focus-visible:ring-offset-2 focus-visible:ring-offset-base-100 focus-visible:bg-base-200"
    aria-haspopup="listbox"
    aria-expanded={isOpen}
    aria-controls={menuId}
    id={buttonId}
    type="button"
    on:click={onButtonClick}
  >
    {#if selected.length === 0}
      {label}: None
    {:else if selected.length <= 2}
      {label}:
      {#each selected as s}
        <span class="badge badge-sm bg-primary text-primary-content text-xs"
          >{s}</span
        >
      {/each}
    {:else}
      {label}: {selected.length} selected
    {/if}
    <svg class="w-3 h-3 opacity-70" viewBox="0 0 20 20" aria-hidden="true">
      <path
        d="M5.5 7.5 10 12l4.5-4.5"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
      />
    </svg>
  </button>

  <div
    class="dropdown-content p-2 bg-base-100 rounded-box w-56 max-h-60 overflow-y-auto overflow-x-hidden flex flex-col gap-1 border-base-content/35 border shadow-lg"
    role="listbox"
    aria-multiselectable="true"
    aria-labelledby={buttonId}
    id={menuId}
    bind:this={menuEl}
    aria-hidden={!isOpen}
    tabindex="-1"
    style={`visibility:${isOpen && alignmentDone ? "visible" : "hidden"};pointer-events:${isOpen ? "auto" : "none"}`}
  >
    <div
      class="flex items-center justify-between gap-2 sticky -top-2 bg-base-100 z-10 -mx-2 -mt-2 px-2 pt-2 pb-2 mb-0 border-b border-base-content/20"
    >
      <button
        type="button"
        class="btn btn-xs"
        on:click|stopPropagation={selectAll}>Select all</button
      >
      <button
        type="button"
        class="btn btn-xs btn-ghost"
        on:click|stopPropagation={clearAll}>Clear</button
      >
    </div>
    {#each items as it, i (it)}
      <label
        class="label cursor-pointer items-center justify-between gap-2 w-full"
        for={`ms-${instanceId}-${i}`}
      >
        <span class="inline-flex items-center gap-2">
          <input
            id={`ms-${instanceId}-${i}`}
            type="checkbox"
            class="checkbox checkbox-xs"
            checked={selectedSet.has(it)}
            on:change={(e) => onCheckboxChange(it, e)}
          />
          <span class="text-xs">{it}</span>
        </span>
        <span class="ml-2 text-xs whitespace-nowrap">
          <slot name="itemRight" item={it} isSelected={selectedSet.has(it)} />
        </span>
      </label>
    {/each}
  </div>
</div>
