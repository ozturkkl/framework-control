<script lang="ts">
    export let healthy = false;
    export let installerUrl: string = "";
    export let cliPresent: boolean = true;
    const repoLink = "https://github.com/ozturkkl/framework-control/tree/main";
    const MAIN_PAGE = `${import.meta.env.BASE_URL}assets/main-page.jpg`;
    const IMG_DESKTOP = `${import.meta.env.BASE_URL}assets/desktop.jpg`;
    const IMG_16 = `${import.meta.env.BASE_URL}assets/laptop-16.jpg`;
    const IMG_13 = `${import.meta.env.BASE_URL}assets/laptop-13.jpg`;
    const IMG_12 = `${import.meta.env.BASE_URL}assets/laptop-12.jpg`;

    import { DefaultService, type SystemInfo } from "../api";
    import { getScreenResolution } from "../lib/device";
    import Icon from "@iconify/svelte";
    import SettingsModal from "./SettingsModal.svelte";
    import LogsModal from "./LogsModal.svelte";
    import { gtSemver } from "../lib/semver";
    import { tooltip } from "../lib/tooltip";

    let triedToFetchVersions = false;
    let displayTitle = "Your Laptop";
    let openImage = IMG_13;

    $: {
        const t = displayTitle.toLowerCase();
        if (t.includes("desktop")) {
            openImage = IMG_DESKTOP;
        } else if (t.includes("16")) {
            openImage = IMG_16;
        } else if (t.includes("12")) {
            openImage = IMG_12;
        } else if (t.includes("13")) {
            openImage = IMG_13;
        } else {
            openImage = MAIN_PAGE;
        }
    }

    let bios: string | null = null;
    let sys: SystemInfo = {
        cpu: "",
        memory_total_mb: 0,
        os: "",
        dgpu: "",
    };

    const screenRes = getScreenResolution();

    $: if (healthy && !triedToFetchVersions) {
        (async () => {
            try {
                const v = await DefaultService.getVersions();
                if (v.mainboard_type) displayTitle = v.mainboard_type;
                if (v.uefi_version) bios = v.uefi_version;
            } catch {
                console.error("Failed to fetch versions");
            } finally {
                triedToFetchVersions = true;
            }
        })();
    }

    $: if (healthy && (!sys || !sys.cpu)) {
        (async () => {
            try {
                sys = await DefaultService.getSystemInfo();
            } catch {
                sys = {
                    cpu: "Unknown CPU",
                    memory_total_mb: 0,
                    os: "Unknown",
                    dgpu: "Unknown GPU",
                };
            }
        })();
    }

    function fmtMem(mb?: number) {
        if (!mb) return "—";
        const gb = Math.round(mb / 1024);
        return `${gb} GB`;
    }

    let infoCardClass =
        "inline-flex items-center gap-2 bg-base-200 hover:bg-base-300 transition-colors rounded-lg px-3 py-1.5  text-xs md:text-sm border border-primary/20 ";
    let infoCardIconClass = "w-4 h-4 md:w-5 md:h-5";
    let showSettings = false;
    let showLogs = false;
    let statusBtn: HTMLElement;
    let statusTipVisible = false;

    // Update check state (for settings dot)
    let currentServiceVersion: string | null = null;
    let latestServiceVersion: string | null = null;
    let updatesPaused: boolean = false;
    let hasUpdate: boolean = false;

    function loadUpdatePrefs() {
        updatesPaused = localStorage.getItem("fc_updates_paused") === "1";
    }
    loadUpdatePrefs();

    // Listen for updates paused changes
    if (typeof window !== "undefined") {
        window.addEventListener("fc_updates_paused_changed", () => {
            loadUpdatePrefs();
        });
    }
    // Trigger one check when becoming healthy (or if already healthy at mount)
    let prevHealthy = false;
    $: if (healthy && !prevHealthy) {
        prevHealthy = healthy;
        checkServiceUpdateOnce();
    }

    async function checkServiceUpdateOnce() {
        try {
            const j = await DefaultService.checkUpdate();
            currentServiceVersion =
                (j.current_version ?? null)?.toString().trim() || null;
            latestServiceVersion =
                (j.latest_version ?? null)?.toString().trim() || null;
        } catch {}
    }

    $: hasUpdate =
        currentServiceVersion && latestServiceVersion
            ? gtSemver(latestServiceVersion, currentServiceVersion) &&
              !updatesPaused
            : false;
</script>

<div class="card bg-base-100 shadow overflow-hidden">
    <div class="card-body p-4">
        <div
            class={healthy
                ? "flex flex-col md:flex-row gap-6"
                : "flex flex-col md:flex-row gap-8 items-center"}
        >
            <div
                class="rounded-box overflow-hidden shadow relative {healthy
                    ? 'hidden md:block w-56 shrink-0'
                    : 'w-1/2'}"
                style="aspect-ratio: 3 / 2;"
            >
                <img
                    src={MAIN_PAGE}
                    alt="Framework laptop lid closed"
                    class="absolute inset-0 w-full h-full object-cover"
                    style="opacity: {healthy ? 0 : 1}"
                />
                <img
                    src={openImage}
                    alt="Framework laptop lid open"
                    class="absolute inset-0 w-full h-full object-cover"
                    style="opacity: {healthy ? 1 : 0}"
                />
            </div>

            <div class="flex flex-col justify-evenly flex-1 min-w-0 w-full">
                {#if healthy}
                    <div class="flex items-center gap-3 justify-between">
                        <div
                            class="md:hidden rounded-lg overflow-hidden shadow relative w-20 h-14 shrink-0 bg-base-200"
                        >
                            <img
                                src={openImage}
                                alt="Lid open"
                                class="absolute inset-0 w-full h-full object-cover"
                            />
                        </div>
                        <h2 class="text-xl md:text-2xl font-semibold">
                            {displayTitle}
                        </h2>
                        <div class="flex items-center gap-0">
                            {#if cliPresent}
                                <button
                                    class="btn btn-success btn-xs mx-3 p-2 rounded-full h-0 w-0 min-h-0 md:w-auto md:h-auto md:py-1 md:mx-2"
                                    aria-label="Connected"
                                    bind:this={statusBtn}
                                    on:mouseenter={() =>
                                        (statusTipVisible = true)}
                                    on:mouseleave={() =>
                                        (statusTipVisible = false)}
                                    on:focus={() => (statusTipVisible = true)}
                                    on:blur={() => (statusTipVisible = false)}
                                >
                                    <span class="hidden md:inline"
                                        >Connected</span
                                    >
                                </button>
                            {:else}
                                <a
                                    class="btn btn-error btn-xs mx-3 p-2 rounded-full h-0 w-0 min-h-0 md:w-auto md:h-auto md:py-1 md:mx-2"
                                    href={installerUrl}
                                    aria-label="framework_tool missing — Reinstall"
                                    bind:this={statusBtn}
                                    on:mouseenter={() =>
                                        (statusTipVisible = true)}
                                    on:mouseleave={() =>
                                        (statusTipVisible = false)}
                                    on:focus={() => (statusTipVisible = true)}
                                    on:blur={() => (statusTipVisible = false)}
                                >
                                    <span class="hidden md:inline"
                                        >framework_tool missing — Reinstall</span
                                    >
                                </a>
                                <button
                                    class="btn btn-warning btn-sm mr-2"
                                    aria-label="View logs"
                                    on:click={() => (showLogs = true)}
                                >
                                    <Icon
                                        icon="mdi:text-box-search-outline"
                                        class="w-5 h-5"
                                    />
                                </button>
                            {/if}
                            <div
                                use:tooltip={{
                                    anchor: statusBtn,
                                    visible: statusTipVisible,
                                    attachGlobalDismiss: false,
                                }}
                                class="pointer-events-none bg-base-100 px-2 py-1 rounded border border-base-300 shadow text-xs text-center md:!hidden"
                            >
                                {#if cliPresent}
                                    Connected
                                {:else}
                                    framework_tool missing — Reinstall
                                {/if}
                            </div>
                            <button
                                class="btn btn-ghost btn-sm mr-0 relative"
                                aria-label="Open settings"
                                on:click={() => (showSettings = true)}
                            >
                                <Icon icon="mdi:cog-outline" class="w-5 h-5" />
                                {#if hasUpdate}
                                    <span
                                        class="absolute top-0.5 right-0.5 w-2 h-2 rounded-full bg-warning animate-pulse z-10 pointer-events-none"
                                    ></span>
                                {/if}
                            </button>
                            <a
                                class="btn btn-ghost btn-sm mr-0"
                                href={repoLink}
                                target="_blank"
                                rel="noreferrer noopener"
                                aria-label="Open GitHub repository"
                            >
                                <Icon icon="mdi:github" class="w-5 h-5" />
                            </a>
                        </div>
                    </div>
                    {#if showSettings}
                        <SettingsModal
                            on:close={() => (showSettings = false)}
                        />
                    {/if}
                    {#if showLogs}
                        <LogsModal on:close={() => (showLogs = false)} />
                    {/if}
                    <hr class="my-2 border border-primary/15" />
                    {#if sys}
                        <div class="flex flex-wrap gap-2">
                            <div class={infoCardClass}>
                                <Icon
                                    icon="lets-icons:cpu"
                                    class={infoCardIconClass}
                                />
                                <span>{sys.cpu || "Unknown CPU"}</span>
                            </div>
                            {#if sys.dgpu}
                                <div class={infoCardClass}>
                                    <Icon
                                        icon="hugeicons:gpu"
                                        class={infoCardIconClass}
                                    />
                                    <span>{sys.dgpu}</span>
                                </div>
                            {/if}
                            <div class={infoCardClass}>
                                <Icon
                                    icon="fluent:memory-16-regular"
                                    class={infoCardIconClass}
                                />
                                <span>{fmtMem(sys.memory_total_mb)} RAM</span>
                            </div>
                            <div class={infoCardClass}>
                                {#if sys.os === "Windows"}
                                    <Icon
                                        icon="mdi:microsoft-windows"
                                        class={infoCardIconClass}
                                    />
                                {:else if sys.os === "Linux"}
                                    <Icon
                                        icon="mdi:linux"
                                        class={infoCardIconClass}
                                    />
                                {:else}
                                    <Icon
                                        icon="clarity:hard-disk-line"
                                        class={infoCardIconClass}
                                    />
                                {/if}
                                <span>OS: {sys.os}</span>
                            </div>
                            {#if bios}
                                <div class={infoCardClass}>
                                    <Icon
                                        icon="ix:firmware"
                                        class={infoCardIconClass}
                                    />
                                    <span>BIOS: {bios}</span>
                                </div>
                            {/if}
                            {#if screenRes}
                                <div class={infoCardClass}>
                                    <Icon
                                        icon="jam:screen"
                                        class={infoCardIconClass}
                                    />
                                    <span
                                        >Screen: {screenRes.width}×{screenRes.height}</span
                                    >
                                </div>
                            {/if}
                        </div>
                    {/if}
                {:else}
                    <div class="space-y-4 lg:space-y-6">
                        <h1
                            class="text-3xl md:text-5xl font-extrabold leading-tight tracking-tight bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent animate-gradient"
                        >
                            Make your Framework come alive
                        </h1>
                        <p class="opacity-70 leading-relaxed">
                            Install the small background service to unlock live
                            telemetry and fan control. Currently only works on
                            Windows.
                        </p>
                        <div class="flex items-center gap-6 lg:gap-4 flex-wrap">
                            {#if installerUrl}
                                <a
                                    class="btn btn-primary btn-lg px-6"
                                    href={installerUrl}>Download Service</a
                                >
                            {:else}
                                <button class="btn btn-disabled btn-lg px-6"
                                    >Download coming soon</button
                                >
                            {/if}
                            <a
                                class="btn btn-ghost btn-lg px-4"
                                href={repoLink}
                                target="_blank"
                                rel="noreferrer noopener"
                                aria-label="Open GitHub repository"
                            >
                                <Icon icon="mdi:github" class="w-5 h-5" />
                            </a>
                            <span
                                class="text-sm opacity-60 min-w-[14.5rem] whitespace-wrap w-0 flex-1"
                                >Choose "More info" → "Run anyway". The page
                                will update automatically.</span
                            >
                        </div>
                        <div class="text-sm opacity-60 whitespace-normal">
                            *You may see a warning when installing; if you're
                            paranoid, you can build the installer yourself from
                            the repo.
                        </div>
                    </div>
                {/if}
            </div>
        </div>
    </div>
</div>

<style>
    @keyframes gradientShift {
        0% {
            filter: hue-rotate(0deg);
        }
        100% {
            filter: hue-rotate(360deg);
        }
    }
    .animate-gradient {
        animation: gradientShift 8s linear infinite;
    }
</style>
