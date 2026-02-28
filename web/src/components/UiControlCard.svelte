<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import Icon from "@iconify/svelte";

    // Minimal composite slider with header, optional icon, value display,
    // and optional Enabled toggle.
    export let label: string;
    export let icon: string | null = null;
    export let unit: string = "";
    export let min: number = 0;
    export let max: number = 100;
    export let step: number = 1;
    export let value: number | string;
    export let hasEnabled: boolean = false;
    export let enabled: boolean = true;
    export let variant: "range" | "select" = "range";
    export let options: string[] = [];
    // Highlight and clamp above this cap when provided
    export let capMax: number | null = null;
    export let allowPassingCapMax: boolean = false;
    // Highlight and clamp below this cap when provided
    export let capMin: number | null = null;
    export let allowPassingCapMin: boolean = false;

    const dispatch = createEventDispatcher<{
        input: { value: number | string; enabled: boolean };
        change: { value: number | string; enabled: boolean };
    }>();

    function clamp(n: number, lo: number, hi: number) {
        return Math.max(lo, Math.min(hi, n));
    }

    $: if (
        variant === "range" &&
        allowPassingCapMax === false &&
        capMax != null
    ) {
        const current = Number(value);
        const clamped = clamp(current, min, capMax);
        if (clamped !== current) {
            value = clamped;
        }
    }

    $: if (
        variant === "range" &&
        allowPassingCapMin === false &&
        capMin != null
    ) {
        const current = Number(value);
        const clamped = clamp(current, capMin, max);
        if (clamped !== current) {
            value = clamped;
        }
    }

    $: capLeftPct =
        variant === "range" && capMax != null
            ? ((clamp(capMax, min, max) - min) / (max - min)) * 100
            : null;

    $: capMinPct =
        variant === "range" && capMin != null
            ? ((clamp(capMin, min, max) - min) / (max - min)) * 100
            : null;

    function handleInput(e: Event) {
        const v = Number((e.target as HTMLInputElement).value);
        if (capMin != null && v < capMin && !allowPassingCapMin) {
            value = capMin;
        } else if (capMax != null && v > capMax && !allowPassingCapMax) {
            value = capMax;
        } else {
            value = v;
        }
        dispatch("input", { value, enabled });
    }

    function handleChange(e: Event) {
        const v = Number((e.target as HTMLInputElement).value);
        if (capMin != null && v < capMin && !allowPassingCapMin) {
            value = capMin;
        } else if (capMax != null && v > capMax && !allowPassingCapMax) {
            value = capMax;
        } else {
            value = v;
        }
        dispatch("change", { value, enabled });
    }

    function handleSelectChange(e: Event) {
        value = (e.target as HTMLSelectElement).value;
        dispatch("change", { value, enabled });
    }

    function handleToggle(e: Event) {
        const el = e.target as HTMLInputElement;
        enabled = el.checked;
        dispatch("change", { value, enabled });
    }
</script>

<div class="flex flex-col rounded-xl bg-base-200 min-w-0 gap-2 py-2 px-3">
    <div class="flex items-center justify-between">
        <div
            class="flex items-center gap-1.5"
            class:opacity-60={hasEnabled && !enabled}
        >
            {#if icon}
                <Icon {icon} class="w-4 h-4 text-primary/80" />
            {/if}
            <h3 class="card-title text-sm">{label}</h3>
        </div>
        <div class="flex items-center gap-2 text-xs">
            <!-- Optional trailing content area for chips/menus placed by parent -->
            <slot name="header-trailing" />
            {#if variant !== "select"}
                <span
                    class="font-medium tabular-nums text-right whitespace-nowrap"
                    class:opacity-60={hasEnabled && !enabled}
                    >{Math.round(Number(value) * 100) / 100} {unit}</span
                >
            {/if}
            {#if hasEnabled}
                {#if variant !== "select"}
                    <span class:opacity-60={!enabled}>•</span>
                {/if}
                <label
                    class="label cursor-pointer gap-2 text-xs flex-row-reverse"
                >
                    <input
                        type="checkbox"
                        class="checkbox checkbox-xs"
                        class:checkbox-success={enabled}
                        bind:checked={enabled}
                        on:change={handleToggle}
                    />
                    <span class="label-text">Enabled</span>
                </label>
            {/if}
        </div>
    </div>
    <div
        class="flex items-center gap-3"
        class:opacity-60={hasEnabled && !enabled}
    >
        {#if variant === "select"}
            <div class="flex-1 flex items-center">
                <select
                    class="select select-sm select-bordered w-full"
                    bind:value
                    on:change={handleSelectChange}
                    disabled={hasEnabled && !enabled}
                >
                    {#if value != null && value !== "" && !options.includes(String(value))}
                        <option value={String(value)}>
                            {String(value)} (unavailable)
                        </option>
                    {/if}
                    {#each options as opt}
                        <option value={opt}>{opt}</option>
                    {/each}
                </select>
            </div>
        {:else}
            <div class="relative flex-1 flex items-center">
                {#if capMinPct != null}
                    <div
                        aria-hidden="true"
                        class="absolute top-1/2 -translate-y-1/2 h-1 rounded-full pointer-events-none bg-secondary/50 z-10"
                        style={`left: 0; right: ${100 - capMinPct}%;`}
                    />
                {/if}
                {#if capLeftPct != null}
                    <div
                        aria-hidden="true"
                        class="absolute top-1/2 -translate-y-1/2 h-1 rounded-full pointer-events-none bg-secondary/50 z-10"
                        style={`left: ${capLeftPct}%; right: 0;`}
                    />
                {/if}
                <input
                    type="range"
                    {min}
                    {max}
                    {step}
                    bind:value
                    class="range range-sm w-full relative z-20"
                    on:input={handleInput}
                    on:change={handleChange}
                />
            </div>
        {/if}
    </div>
</div>
