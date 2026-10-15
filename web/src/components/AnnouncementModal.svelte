<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import Icon from "@iconify/svelte";
    import { configStore, patch } from "../lib/config";
    import { portal } from "../lib/portal";
    import type { Announcement } from "../lib/announcement";

    export let announcement: Announcement;

    const dispatch = createEventDispatcher<{ close: void }>();

    let formEl: HTMLFormElement;
    let hideFuture = !!$configStore.config?.ui?.announcements?.disabled;
    let saving = false;
    let errorMessage: string | null = null;

    async function dismiss() {
        if (saving || !formEl.reportValidity()) return;
        saving = true;
        errorMessage = null;
        try {
            await patch({
                ui: {
                    announcements: {
                        disabled: hideFuture,
                        seen_date: announcement.date,
                    },
                },
            });
            dispatch("close");
        } catch (e) {
            errorMessage = e instanceof Error ? e.message : String(e);
        } finally {
            saving = false;
        }
    }

    function onWindowKeydown(e: KeyboardEvent) {
        if (e.key !== "Escape") return;
        e.preventDefault();
        e.stopImmediatePropagation();
        dismiss();
    }
</script>

<!-- Capture so Escape doesn't also reach a modal underneath (e.g. Settings). -->
<svelte:window on:keydown|capture={onWindowKeydown} />

<div class="modal modal-open" use:portal>
    <form
        class="modal-box max-w-2xl max-h-[min(80vh,60rem)] bg-base-200 p-0 flex flex-col overflow-hidden"
        role="dialog"
        aria-modal="true"
        aria-label="Announcement"
        tabindex="-1"
        bind:this={formEl}
        on:submit|preventDefault={dismiss}
    >
        <div class="announcement prose prose-sm max-w-none overflow-y-auto p-6">
            <!-- Sanitized by DOMPurify in lib/announcement. -->
            {@html announcement.html}
        </div>
        <div class="border-t surface-border px-6 py-4">
            {#if errorMessage}
                <div class="alert alert-error py-1 px-3 text-xs mb-3">
                    <Icon icon="mdi:alert-circle" class="w-4 h-4" />
                    <span>{errorMessage}</span>
                </div>
            {/if}
            <div class="flex flex-wrap items-center justify-between gap-2">
                <label class="label cursor-pointer gap-2 text-xs">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-xs"
                        bind:checked={hideFuture}
                    />
                    <span class="label-text">Don't show announcements in the future</span>
                </label>
                <button
                    type="submit"
                    class="btn btn-sm bg-base-100 surface-border"
                    disabled={saving}
                >
                    Close
                </button>
            </div>
        </div>
    </form>
    <button
        class="modal-backdrop"
        tabindex="-1"
        on:click={dismiss}
        aria-label="Close announcement"
    ></button>
</div>

<style>
    .announcement :global(iframe) {
        width: 100%;
        aspect-ratio: 16 / 9;
    }
</style>
