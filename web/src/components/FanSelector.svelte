<script lang="ts">
    import { createEventDispatcher, tick } from "svelte";
    import Icon from "@iconify/svelte";

    export let activeFan: "all" | number = "all";
    export let fanLabels: string[] = [];
    export let overrideFans: Set<number> = new Set();

    const dispatch = createEventDispatcher<{
        select: "all" | number;
        clear: number;
    }>();

    const instanceId = crypto.randomUUID();
    const buttonId = `fan-sel-btn-${instanceId}`;
    const menuId = `fan-sel-menu-${instanceId}`;

    let isOpen = false;
    let rootEl: HTMLDivElement;
    let buttonEl: HTMLButtonElement;
    let menuEl: HTMLDivElement;

    $: currentLabel =
        activeFan === "all"
            ? "All fans"
            : (fanLabels[activeFan] ?? `Fan ${activeFan + 1}`);
    $: customized = activeFan !== "all" && overrideFans.has(activeFan);

    function close() {
        isOpen = false;
    }

    async function toggle() {
        isOpen = !isOpen;
        if (!isOpen) return;
        await tick();
        const checked = menuEl?.querySelector<HTMLButtonElement>(
            '[aria-checked="true"]',
        );
        (checked ?? menuButtons()[0])?.focus();
    }

    function choose(target: "all" | number) {
        close();
        dispatch("select", target);
        buttonEl?.focus();
    }

    function menuButtons(): HTMLButtonElement[] {
        if (!menuEl) return [];
        return Array.from(menuEl.querySelectorAll("button"));
    }

    function moveFocus(delta: number) {
        const buttons = menuButtons();
        if (buttons.length === 0) return;
        const i = buttons.findIndex((b) => b === document.activeElement);
        const next =
            i === -1
                ? delta > 0
                    ? 0
                    : buttons.length - 1
                : (i + delta + buttons.length) % buttons.length;
        buttons[next].focus();
    }

    function onWindowClick(e: MouseEvent) {
        if (isOpen && rootEl && !e.composedPath().includes(rootEl)) close();
    }

    function onWindowKeydown(e: KeyboardEvent) {
        if (!isOpen) return;
        if (e.key === "Escape") {
            e.preventDefault();
            close();
            buttonEl?.focus();
            return;
        }
        if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            moveFocus(e.key === "ArrowDown" ? 1 : -1);
        }
    }
</script>

<svelte:window on:click={onWindowClick} on:keydown={onWindowKeydown} />

<div class="relative shrink-0" class:z-50={isOpen} bind:this={rootEl}>
    <button
        bind:this={buttonEl}
        class="btn btn-xs btn-ghost gap-1 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-base-content/80 focus-visible:ring-offset-2 focus-visible:ring-offset-base-100 focus-visible:bg-base-200"
        type="button"
        id={buttonId}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        aria-controls={menuId}
        aria-label="Fan scope: {currentLabel}{customized ? ', custom' : ''}"
        on:click={toggle}
    >
        <Icon icon="mdi:fan" class="text-sm shrink-0" aria-hidden="true" />
        <span class="truncate max-w-[6.5rem]">{currentLabel}</span>
        {#if customized}
            <span
                class="inline-block w-1.5 h-1.5 rounded-full bg-primary shrink-0"
                aria-hidden="true"
            ></span>
        {/if}
        <svg
            class="w-3 h-3 opacity-70 shrink-0"
            viewBox="0 0 20 20"
            aria-hidden="true"
        >
            <path
                d="M5.5 7.5 10 12l4.5-4.5"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
            />
        </svg>
    </button>

    {#if isOpen}
        <div
            bind:this={menuEl}
            class="absolute left-0 top-full mt-1 p-0.5 bg-base-100 rounded-md w-max min-w-full border border-base-content/35 shadow-lg"
            role="menu"
            aria-labelledby={buttonId}
            id={menuId}
        >
            <button
                type="button"
                role="menuitemradio"
                aria-checked={activeFan === "all"}
                class="flex items-center w-full h-6 px-2 rounded-sm text-xs text-left hover:bg-base-200"
                class:bg-base-200={activeFan === "all"}
                on:click={() => choose("all")}
            >
                All fans
            </button>
            {#each fanLabels as label, i (i)}
                <div
                    class="flex items-center h-6 rounded-sm hover:bg-base-200"
                    class:bg-base-200={activeFan === i}
                    role="none"
                >
                    <button
                        type="button"
                        role="menuitemradio"
                        aria-checked={activeFan === i}
                        class="flex-1 h-full pl-2 pr-1 text-xs text-left whitespace-nowrap"
                        on:click={() => choose(i)}
                    >
                        {label}
                    </button>
                    {#if overrideFans.has(i)}
                        <span
                            class="inline-flex items-center gap-0.5 pr-1.5 shrink-0"
                        >
                            <button
                                type="button"
                                role="menuitem"
                                class="w-5 h-5 inline-flex items-center justify-center rounded-sm hover:bg-base-300"
                                aria-label="Follow all fans for {label}"
                                title="Follow all fans"
                                on:click={() => dispatch("clear", i)}
                            >
                                <Icon
                                    icon="mdi:backup-restore"
                                    class="text-sm opacity-80"
                                    aria-hidden="true"
                                />
                            </button>
                            <span
                                class="inline-block w-1.5 h-1.5 rounded-full bg-primary"
                                title="Custom"
                                aria-hidden="true"
                            ></span>
                        </span>
                    {/if}
                </div>
            {/each}
        </div>
    {/if}
</div>
