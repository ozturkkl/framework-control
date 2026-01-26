<script lang="ts">
    import Icon from "@iconify/svelte";
    import { onMount } from "svelte";

    export let serviceCurrent: string | null = null;
    export let serviceLatest: string | null = null;
    export let apiOrigin: string = "http://127.0.0.1:8090";
    export let repoUrl: string =
        "https://github.com/ozturkkl/framework-control/releases";

    const appUrl = apiOrigin.replace(/\/$/, "");

    let primaryBtnEl: HTMLAnchorElement | null = null;
    onMount(() => {
        // Move focus to primary action to avoid showing a focus border on the dialog box itself
        primaryBtnEl?.focus();
    });
</script>

<div class="modal modal-open">
    <div
        class="modal-box max-w-xl"
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div class="flex items-start gap-3">
            <Icon icon="mdi:alert" class="w-8 h-8 text-warning " />
            <div class="space-y-2">
                <h3 class="font-bold text-lg">
                    Please switch to the local app
                </h3>
                <p class="text-sm opacity-80">
                    You're using the hosted web app while your local service has
                    an update available. To avoid version mismatches, continue
                    in the local app or update the service.
                </p>
                <p class="text-xs opacity-70">
                    Service version {serviceCurrent || "?"}
                    {#if serviceLatest}
                        → Latest {serviceLatest}
                    {/if}
                </p>
            </div>
        </div>

        <div class="flex flex-col md:flex-row gap-2 md:justify-end mt-4">
            <a class="btn btn-primary" href={appUrl} bind:this={primaryBtnEl}>
                <Icon icon="mdi:open-in-new" class="w-4 h-4" />
                Open local app
            </a>
            <a
                class="btn btn-ghost"
                href={repoUrl}
                target="_blank"
                rel="noreferrer noopener"
            >
                <Icon icon="mdi:github" class="w-4 h-4" />
                Releases
            </a>
        </div>
    </div>
    <div class="modal-backdrop"></div>
</div>

<style>
</style>
