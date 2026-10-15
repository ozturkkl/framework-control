<script lang="ts">
    import Icon from "@iconify/svelte";
    import { tick } from "svelte";
    import { tooltip } from "../lib/tooltip";
    import type {
        FanControlConfig,
        FanControlPatch,
        FanProfile,
        FanProfileAutoSwitch,
    } from "../api";

    export let fan: FanControlConfig | undefined;
    // Built from the latest config, after the editor's pending changes are saved.
    export let update: (
        makePatch: (fan: FanControlConfig) => FanControlPatch,
    ) => Promise<void>;

    const NAME_MAX = 24;
    const SOURCES: {
        key: keyof FanProfileAutoSwitch;
        label: string;
        icon: string;
    }[] = [
        { key: "ac", label: "Plugged in", icon: "mdi:power-plug-outline" },
        { key: "battery", label: "On battery", icon: "mdi:battery-outline" },
    ];

    let isOpen = false;
    let rootEl: HTMLDivElement;
    let renamingId: string | null = null;
    let renameValue = "";
    let renameInput: HTMLInputElement;
    let confirmDeleteId: string | null = null;
    let alignEnd = true;
    let infoBtn: HTMLButtonElement;
    let infoTipVisible = false;
    let dragId: string | null = null;
    let dropTargetId: string | null = null;

    $: profiles = fan?.profiles ?? [];
    $: active = profiles.find((p) => p.id === fan?.active_profile);
    $: auto = fan?.auto_switch ?? {};
    $: sourcesFor = (id: string) => SOURCES.filter((s) => auto[s.key] === id);
    $: activeSources = active ? sourcesFor(active.id) : [];

    function close() {
        commitRename();
        isOpen = false;
        confirmDeleteId = null;
        infoTipVisible = false;
    }

    function open() {
        // Flip to left-aligned when the menu would overflow the left edge.
        const MENU_WIDTH = 256;
        alignEnd = rootEl.getBoundingClientRect().right - MENU_WIDTH >= 0;
        isOpen = true;
    }

    function onWindowClick(e: MouseEvent) {
        if (isOpen && !e.composedPath().includes(rootEl)) close();
    }

    function select(id: string) {
        close();
        if (id !== fan?.active_profile) update(() => ({ active_profile: id }));
    }

    function nextName(existing: FanProfile[]): string {
        const names = new Set(existing.map((p) => p.name));
        let n = existing.length + 1;
        while (names.has(`Profile ${n}`)) n++;
        return `Profile ${n}`;
    }

    async function createProfile() {
        const id = crypto.randomUUID().slice(0, 8);
        let created: FanProfile | undefined;
        await update((fan) => {
            const source = fan.profiles.find(
                (p) => p.id === fan.active_profile,
            );
            if (!source) return {};
            created = { ...source, id, name: nextName(fan.profiles) };
            return { profiles: [...fan.profiles, created], active_profile: id };
        });
        if (created) startRename(created);
    }

    async function startRename(p: FanProfile) {
        confirmDeleteId = null;
        renamingId = p.id;
        renameValue = p.name;
        await tick();
        renameInput?.select();
    }

    function commitRename() {
        if (!renamingId) return;
        const id = renamingId;
        const name = renameValue.trim().slice(0, NAME_MAX);
        renamingId = null;
        const current = profiles.find((p) => p.id === id);
        if (!current || !name || name === current.name) return;
        update((fan) => ({
            profiles: fan.profiles.map((p) =>
                p.id === id ? { ...p, name } : p,
            ),
        }));
    }

    function onRenameKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") commitRename();
        if (e.key === "Escape") {
            e.stopPropagation();
            renamingId = null;
        }
    }

    function deleteProfile(id: string) {
        confirmDeleteId = null;
        update((fan) => {
            const remaining = fan.profiles.filter((p) => p.id !== id);
            const auto = fan.auto_switch;
            const patch: FanControlPatch = {
                profiles: remaining,
                auto_switch: {
                    ac: auto.ac === id ? undefined : auto.ac,
                    battery: auto.battery === id ? undefined : auto.battery,
                },
            };
            if (id === fan.active_profile) {
                patch.active_profile = remaining[0].id;
            }
            return patch;
        });
    }

    function endDrag() {
        dragId = null;
        dropTargetId = null;
    }

    function dropOn(targetId: string) {
        const id = dragId;
        endDrag();
        if (!id || id === targetId) return;
        update((fan) => {
            const from = fan.profiles.findIndex((p) => p.id === id);
            const to = fan.profiles.findIndex((p) => p.id === targetId);
            if (from < 0 || to < 0) return {};
            const profiles = [...fan.profiles];
            profiles.splice(to, 0, ...profiles.splice(from, 1));
            return { profiles };
        });
    }

    function setAuto(key: keyof FanProfileAutoSwitch, id: string) {
        update((fan) => ({
            auto_switch: { ...fan.auto_switch, [key]: id || undefined },
        }));
    }
</script>

<svelte:window
    on:click={onWindowClick}
    on:keydown={(e) => e.key === "Escape" && isOpen && close()}
/>

<div
    class="dropdown"
    class:dropdown-end={alignEnd}
    class:dropdown-open={isOpen}
    bind:this={rootEl}
>
    <button
        type="button"
        class="btn btn-xs border surface-border gap-1 flex-nowrap max-w-[8rem] font-normal"
        aria-haspopup="menu"
        aria-expanded={isOpen}
        title={active
            ? `Fan profile: ${active.name}${activeSources.length ? ` (used when ${activeSources.map((s) => s.label.toLowerCase()).join(" or ")})` : ""}`
            : "Fan profiles"}
        on:click={() => (isOpen ? close() : open())}
    >
        <Icon
            icon={activeSources.length > 1
                ? "mdi:power-plug-battery-outline"
                : (activeSources[0]?.icon ?? "mdi:tune-variant")}
            class="w-3.5 h-3.5 shrink-0 opacity-80"
        />
        <span class="profile-label min-w-0 items-center gap-1">
            <span class="truncate">{active?.name ?? "Profiles"}</span>
            <Icon
                icon="mdi:chevron-down"
                class="w-3 h-3 shrink-0 opacity-70"
            />
        </span>
    </button>

    {#if isOpen}
        <div
            class="dropdown-content z-30 mt-1 p-2 w-64 max-w-[calc(100vw-2rem)] bg-base-100 rounded-box border surface-border shadow-lg flex flex-col gap-1 text-sm"
            role="menu"
        >
            <div class="flex items-center gap-1 px-1 pb-1 text-xs">
                <span class="opacity-60">Fan profiles</span>
                <button
                    type="button"
                    class="btn btn-ghost btn-xs p-0.5 min-h-0 h-auto"
                    aria-label="About fan profiles"
                    bind:this={infoBtn}
                    on:mouseenter={() => (infoTipVisible = true)}
                    on:mouseleave={() => (infoTipVisible = false)}
                    on:focus={() => (infoTipVisible = true)}
                    on:blur={() => (infoTipVisible = false)}
                >
                    <Icon
                        icon="mdi:information-outline"
                        class="w-3.5 h-3.5 opacity-70"
                    />
                </button>
            </div>
            {#each profiles as p (p.id)}
                <div
                    class="group flex items-center gap-1 min-h-7 rounded-btn hover:bg-base-200"
                    class:bg-base-200={p.id === active?.id}
                    class:opacity-50={dragId === p.id}
                    class:ring-1={dropTargetId === p.id && dragId !== p.id}
                    class:ring-primary={dropTargetId === p.id}
                    role="listitem"
                    draggable={renamingId !== p.id}
                    on:dragstart|stopPropagation={(e) => {
                        // stopPropagation: svelte-dnd-action cancels native drags on panel wrappers.
                        // Firefox won't start a drag without data.
                        e.dataTransfer?.setData("text/plain", p.id);
                        dragId = p.id;
                    }}
                    on:dragover={(e) => {
                        if (!dragId) return;
                        e.preventDefault();
                        dropTargetId = p.id;
                    }}
                    on:drop|preventDefault={() => dropOn(p.id)}
                    on:dragend={endDrag}
                >
                    {#if renamingId === p.id}
                        <input
                            class="input input-xs input-bordered flex-1 min-w-0 my-0.5 ml-0.5"
                            maxlength={NAME_MAX}
                            aria-label="Profile name"
                            bind:this={renameInput}
                            bind:value={renameValue}
                            on:keydown={onRenameKeydown}
                            on:blur={commitRename}
                        />
                    {:else if confirmDeleteId === p.id}
                        <span class="flex-1 min-w-0 px-2 py-1 truncate text-xs"
                            >Delete “{p.name}”?</span
                        >
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs btn-square text-error"
                            aria-label="Confirm delete {p.name}"
                            title="Delete"
                            on:click={() => deleteProfile(p.id)}
                        >
                            <Icon icon="mdi:trash-can" class="w-3.5 h-3.5" />
                        </button>
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs btn-square"
                            aria-label="Cancel delete"
                            title="Cancel"
                            on:click={() => (confirmDeleteId = null)}
                        >
                            <Icon icon="mdi:close" class="w-3.5 h-3.5" />
                        </button>
                    {:else}
                        <button
                            type="button"
                            role="menuitemradio"
                            aria-checked={p.id === active?.id}
                            class="flex-1 min-w-0 flex items-center gap-1.5 px-2 py-1 text-left"
                            on:click={() => select(p.id)}
                        >
                            <Icon
                                icon="mdi:check"
                                class="w-3.5 h-3.5 shrink-0 {p.id ===
                                active?.id
                                    ? ''
                                    : 'invisible'}"
                            />
                            <span class="truncate">{p.name}</span>
                            {#each sourcesFor(p.id) as s (s.key)}
                                <span title="Used when {s.label.toLowerCase()}">
                                    <Icon
                                        icon={s.icon}
                                        class="w-3.5 h-3.5 shrink-0 opacity-60"
                                    />
                                </span>
                            {/each}
                        </button>
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs btn-square opacity-50 hover:opacity-100"
                            aria-label="Rename {p.name}"
                            title="Rename"
                            on:click={() => startRename(p)}
                        >
                            <Icon icon="mdi:pencil-outline" class="w-3.5 h-3.5" />
                        </button>
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs btn-square opacity-50 hover:opacity-100"
                            aria-label="Delete {p.name}"
                            title={profiles.length > 1
                                ? "Delete"
                                : "The last profile can't be deleted"}
                            disabled={profiles.length <= 1}
                            on:click={() => {
                                renamingId = null;
                                confirmDeleteId = p.id;
                            }}
                        >
                            <Icon
                                icon="mdi:trash-can-outline"
                                class="w-3.5 h-3.5"
                            />
                        </button>
                    {/if}
                </div>
            {/each}
            <button
                type="button"
                class="btn btn-ghost btn-xs justify-start gap-1.5 font-normal"
                on:click={createProfile}
            >
                <Icon icon="mdi:plus" class="w-3.5 h-3.5" />
                New profile from current settings
            </button>

            <div class="border-t surface-border mt-1 pt-2 px-1 flex flex-col gap-1.5">
                <div class="text-xs opacity-60">Switch automatically</div>
                {#each SOURCES as s (s.key)}
                    <label class="flex items-center justify-between gap-2">
                        <span class="inline-flex items-center gap-1.5 text-xs">
                            <Icon icon={s.icon} class="w-3.5 h-3.5 opacity-70" />
                            {s.label}
                        </span>
                        <select
                            class="select select-xs select-bordered w-32"
                            value={auto[s.key] ?? ""}
                            on:change={(e) =>
                                setAuto(s.key, e.currentTarget.value)}
                        >
                            <option value="">Don't switch</option>
                            {#each profiles as p (p.id)}
                                <option value={p.id}>{p.name}</option>
                            {/each}
                        </select>
                    </label>
                {/each}
            </div>
        </div>
        <div
            use:tooltip={{
                anchor: infoBtn,
                visible: infoTipVisible,
                attachGlobalDismiss: false,
            }}
            class="pointer-events-none bg-base-100 px-2 py-1 rounded-box border surface-border shadow text-xs w-60 text-center"
        >
            Changes save to the active profile. Automatic switching replaces a
            hand-picked profile.
        </div>
    {/if}
</div>

<style>
    /* Icon only in narrow panels so the "Fan Control" title stays visible. */
    .profile-label {
        display: none;
    }
    @container (min-width: 31rem) {
        .profile-label {
            display: inline-flex;
        }
    }
</style>
