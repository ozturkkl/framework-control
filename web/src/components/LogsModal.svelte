<script lang="ts">
    import { createEventDispatcher, onMount, onDestroy, tick } from "svelte";
    import Icon from "@iconify/svelte";
    import { DefaultService, OpenAPI } from "../api";

    const dispatch = createEventDispatcher();

    let logs = "";
    let loading = true;
    let errorMessage: string | null = null;
    let logContainer: HTMLDivElement;
    let refreshInterval: number;

    let showErrors = true;
    let showWarnings = true;
    let showInfo = true;

    let visibleLineCount = 50;
    let allLogLines: string[] = [];
    let isLoadingMore = false;
    let savedScrollTop = 0;
    let savedScrollHeight = 0;

    function classifyLine(line: string): "error" | "warning" | "info" {
        const upper = line.toUpperCase();
        if (
            upper.includes("ERROR") ||
            upper.includes("PANIC") ||
            upper.includes("FAILED WITH RESULT") ||
            upper.includes("EXITED, CODE=EXITED")
        ) {
            return "error";
        }
        if (upper.includes("WARN")) return "warning";
        return "info";
    }

    function escapeHtml(text: string): string {
        const div = document.createElement("div");
        div.textContent = text;
        return div.innerHTML;
    }

    function filterLines(
        logLines: string[],
        errors: boolean,
        warnings: boolean,
        info: boolean,
    ): string[] {
        if (!logLines.length) return [];

        const filterMap = { error: errors, warning: warnings, info };

        return logLines.filter((line) => filterMap[classifyLine(line)]);
    }

    function colorizeLine(line: string): string {
        const type = classifyLine(line);
        const escaped = escapeHtml(line);
        if (type === "error")
            return `<span class="text-error">${escaped}</span>`;
        if (type === "warning")
            return `<span class="text-warning">${escaped}</span>`;
        return escaped;
    }

    $: allLogLines = logs ? logs.split("\n") : [];
    $: filteredLines = filterLines(
        allLogLines,
        showErrors,
        showWarnings,
        showInfo,
    );
    $: visibleLines = filteredLines.slice(-visibleLineCount);
    $: processedLogs = visibleLines.map(colorizeLine).join("\n");

    function toggleFilter(
        filterSetter: (value: boolean) => void,
        currentValue: boolean,
    ) {
        if (!logContainer) return;
        savedScrollTop = logContainer.scrollTop;
        savedScrollHeight = logContainer.scrollHeight;

        filterSetter(!currentValue);

        requestAnimationFrame(() => {
            if (!logContainer) return;
            const distanceFromBottom = savedScrollHeight - savedScrollTop;
            logContainer.scrollTop =
                logContainer.scrollHeight - distanceFromBottom;
        });
    }

    async function handleScroll() {
        if (!logContainer || isLoadingMore) return;

        if (
            logContainer.scrollTop < 100 &&
            visibleLineCount < filteredLines.length
        ) {
            isLoadingMore = true;
            const oldScrollHeight = logContainer.scrollHeight;

            visibleLineCount += 50;

            await tick();
            logContainer.scrollTop +=
                logContainer.scrollHeight - oldScrollHeight;

            isLoadingMore = false;
        }
    }

    async function fetchLogs(isInitialLoad = false) {
        if (isInitialLoad) loading = true;
        errorMessage = null;

        try {
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            logs = await DefaultService.getLogs(auth);
        } catch (e: unknown) {
            const apiError = e as { body?: { message?: string } };
            errorMessage = apiError?.body?.message || "Failed to fetch logs";
        } finally {
            if (isInitialLoad) {
                loading = false;
                tick().then(() => {
                    if (logContainer) {
                        logContainer.scrollTop = logContainer.scrollHeight;
                    }
                });
            }
        }
    }

    onMount(() => {
        fetchLogs(true);
        refreshInterval = window.setInterval(() => fetchLogs(), 3000);
    });

    onDestroy(() => {
        if (refreshInterval) clearInterval(refreshInterval);
    });

    function close() {
        dispatch("close");
    }
</script>

<div class="modal modal-open">
    <div class="modal-box max-w-4xl h-[80vh] flex flex-col">
        <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-lg">Service Logs</h3>
            <button
                class="btn btn-sm btn-ghost btn-circle"
                on:click={close}
                aria-label="Close"
            >
                <Icon icon="mdi:close" class="w-5 h-5" />
            </button>
        </div>

        <div class="flex items-center gap-2 mb-3">
            <span class="text-xs opacity-70 mr-2">Filter:</span>
            <button
                class="btn btn-xs {showErrors ? 'btn-error' : 'btn-ghost'}"
                on:click={() =>
                    toggleFilter((v) => (showErrors = v), showErrors)}
            >
                <Icon icon="mdi:alert-circle" class="w-3 h-3" />
                Errors
            </button>
            <button
                class="btn btn-xs {showWarnings ? 'btn-warning' : 'btn-ghost'}"
                on:click={() =>
                    toggleFilter((v) => (showWarnings = v), showWarnings)}
            >
                <Icon icon="mdi:alert" class="w-3 h-3" />
                Warnings
            </button>
            <button
                class="btn btn-xs {showInfo ? 'btn-info' : 'btn-ghost'}"
                on:click={() => toggleFilter((v) => (showInfo = v), showInfo)}
            >
                <Icon icon="mdi:information" class="w-3 h-3" />
                Info
            </button>
        </div>

        {#if loading}
            <div class="flex-1 flex items-center justify-center">
                <div class="flex flex-col items-center gap-2">
                    <Icon
                        icon="mdi:loading"
                        class="w-8 h-8 animate-spin opacity-70"
                    />
                    <p class="text-sm opacity-70">Loading logs...</p>
                </div>
            </div>
        {:else if errorMessage}
            <div class="alert alert-error">
                <Icon icon="mdi:alert-circle" class="w-5 h-5" />
                <span>{errorMessage}</span>
            </div>
        {:else}
            <div
                class="flex-1 overflow-auto"
                bind:this={logContainer}
                on:scroll={handleScroll}
            >
                {#if visibleLineCount < filteredLines.length}
                    <div class="text-center py-2 text-xs opacity-50">
                        {isLoadingMore ? "Loading..." : "Scroll up for more"}
                    </div>
                {/if}
                <pre
                    class="text-xs bg-base-300 p-4 rounded-lg font-mono whitespace-pre-wrap break-words">{@html processedLogs}</pre>
            </div>
        {/if}

        <div class="modal-action mt-4">
            <button class="btn btn-sm" on:click={close}>Close</button>
        </div>
    </div>
    <button class="modal-backdrop" on:click={close} aria-label="Close logs"
    ></button>
</div>
