<script lang="ts">
    import { onMount } from "svelte";
    import { DefaultService, OpenAPI } from "../api";
    import Icon from "@iconify/svelte";
    import { isLinux } from "../lib/platform";

    let shortcutsCreated = false;
    let checking = true;
    let creatingShortcuts = false;
    let error = "";

    async function checkStatus() {
        try {
            const data = await DefaultService.getShortcutsStatus();
            shortcutsCreated = data.installed || false;
        } catch {
            shortcutsCreated = false;
        } finally {
            checking = false;
        }
    }

    async function createShortcuts() {
        if (creatingShortcuts) return;
        creatingShortcuts = true;
        error = "";
        try {
            // Reuse the OpenAPI.TOKEN if present
            const auth = OpenAPI.TOKEN;
            const result = await DefaultService.createShortcuts(
                auth ? `Bearer ${auth}` : "",
            );
            shortcutsCreated = true;
        } catch (e) {
            error = isLinux()
                ? "Error creating entry"
                : "Error creating shortcuts";
            console.error(
                isLinux()
                    ? "Failed to create entry:"
                    : "Failed to create shortcuts:",
                e,
            );
        } finally {
            creatingShortcuts = false;
        }
    }

    onMount(() => {
        checkStatus();
    });
</script>

<div class="flex items-center justify-end py-1">
    <button
        class={!checking && shortcutsCreated
            ? "btn btn-ghost btn-sm text-success gap-2"
            : "btn btn-sm btn-primary"}
        on:click={createShortcuts}
    >
        {#if !checking && shortcutsCreated}
            <Icon icon="mdi:check" class="w-4 h-4" />
            {isLinux() ? "Entry created" : "Shortcuts created"}
        {:else if creatingShortcuts}
            <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
            {isLinux() ? "Creating entry..." : "Creating shortcuts..."}
        {:else}
            <Icon
                icon="material-symbols:switch-access-shortcut-add-rounded"
                class="w-4 h-4"
            />
            {isLinux() ? "Create entry" : "Create shortcuts"}
        {/if}
    </button>
</div>
{#if error}
    <div class="text-error text-xs mt-1">{error}</div>
{/if}
