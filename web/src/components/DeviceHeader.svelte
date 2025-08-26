<script lang="ts">
  export let healthy = false;
  export let installerUrl: string = "";
  export let cliPresent: boolean = true;
  const repoLink = "https://github.com/ozturkkl/framework-control/tree/main";
  const LID_CLOSED = `${import.meta.env.BASE_URL}assets/lid-closed.jpg`;
  const LID_OPEN = `${import.meta.env.BASE_URL}assets/lid-open.jpg`;
  import {
    DefaultService,
    type SystemInfoEnvelope as SystemInfo,
  } from "../api";
  import { parseFrameworkVersions, getScreenResolution } from "../lib/device";
  import Icon from "@iconify/svelte";

  let versionsText: string | null = null;
  let displayTitle = "Your Laptop";
  let bios: string | null = null;
  let sys: SystemInfo = {
    ok: false,
    cpu: "...",
    memory_total_mb: 0,
    os: "...",
    dgpu: "...",
  };

  const screenRes = getScreenResolution();

  $: if (healthy && versionsText === null) {
    (async () => {
      try {
        const v = await DefaultService.getVersions();
        versionsText = v.stdout ?? "";
        const s = parseFrameworkVersions(versionsText);
        if (s.mainboardType) displayTitle = s.mainboardType;
        if (s.uefiVersion) bios = s.uefiVersion;
      } catch {
        console.error("Failed to fetch versions");
      }
    })();
  }

  $: if (healthy && sys.ok === false) {
    (async () => {
      try {
        sys = await DefaultService.getSystemInfo();
      } catch {
        sys.ok = false;
      }
    })();
  }

  function fmtMem(mb?: number) {
    if (!mb) return "—";
    const gb = Math.round(mb / 1024);
    return `${gb} GB`;
  }

  let infoCardClass =
    "flex items-center gap-2 bg-base-200 rounded-xl px-3 py-2 text-sm md:text-base whitespace-normal break-words border border-primary/25";
  let infoCardIconClass = "w-10 h-10";
</script>

<div class="card bg-base-100 shadow overflow-hidden">
  <div class="card-body p-4">
    <div
      class={healthy
        ? "flex flex-col md:flex-row gap-6"
        : "flex flex-col md:flex-row gap-8 items-center"}
    >
      <div
        class="rounded-box overflow-hidden shadow w-full md:w-auto md:shrink-0 relative"
        style="width: {healthy
          ? '14rem'
          : '50%'}; aspect-ratio: 3 / 2; transition: width 500ms ease;"
      >
        <!-- Crossfade the two images -->
        <img
          src={LID_CLOSED}
          alt="Framework laptop lid closed"
          class="absolute inset-0 w-full h-full object-cover transition-opacity duration-500"
          style="opacity: {healthy ? 0 : 1}"
        />
        <img
          src={LID_OPEN}
          alt="Framework laptop lid open"
          class="absolute inset-0 w-full h-full object-cover transition-opacity duration-500"
          style="opacity: {healthy ? 1 : 0}"
        />
      </div>

      <div class="flex flex-col justify-evenly flex-1 min-w-0 w-full">
        {#if healthy}
          <div class="flex items-center gap-2 justify-between">
            <h2 class="text-xl md:text-2xl font-semibold">{displayTitle}</h2>
            <div class="flex items-center gap-2">
              {#if cliPresent}
                <span class="badge badge-success mr-0">Connected</span>
              {:else}
                <a
                  class="btn btn-error btn-sm mr-0 no-underline inline-flex items-center gap-2"
                  href={repoLink}
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  <Icon icon="mdi:alert-circle-outline" class="w-4 h-4" />
                  <span>framework_tool missing — Install</span>
                </a>
              {/if}
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
          <hr class="my-2 border border-primary/15" />
          {#if sys}
            <div class="flex flex-wrap gap-2">
              <div class={infoCardClass}>
                <Icon icon="lets-icons:cpu" class={infoCardIconClass} />
                <span>{sys.cpu || "Unknown CPU"}</span>
              </div>
              {#if sys.dgpu}
                <div class={infoCardClass}>
                  <Icon icon="hugeicons:gpu" class={infoCardIconClass} />
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
                {:else if sys.os === "macOS"}
                  <Icon icon="mdi:apple-ios" class={infoCardIconClass} />
                {:else if sys.os === "Linux"}
                  <Icon icon="mdi:linux" class={infoCardIconClass} />
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
                  <Icon icon="ix:firmware" class={infoCardIconClass} />
                  <span>BIOS: {bios}</span>
                </div>
              {/if}
              {#if screenRes}
                <div class={infoCardClass}>
                  <Icon icon="jam:screen" class={infoCardIconClass} />
                  <span>Screen: {screenRes.width}×{screenRes.height}</span>
                </div>
              {/if}
            </div>
          {/if}
        {:else}
          <div class="space-y-6">
            <h1
              class="text-3xl md:text-5xl font-extrabold leading-tight tracking-tight bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent animate-gradient"
            >
              Make your Framework come alive
            </h1>
            <p class="opacity-70 leading-relaxed">
              Install the small background service to unlock live telemetry and
              fan control. Currently only works on Windows.
            </p>
            <div
              class="flex items-center gap-2 md:gap-4 flex-wrap md:flex-nowrap"
            >
              {#if installerUrl}
                <a class="btn btn-primary btn-lg px-6" href={installerUrl}
                  >Download Service</a
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
                class="text-sm opacity-60 whitespace-normal max-w-[22rem] md:max-w-lg"
                >Choose "More info" → "Run anyway". The page will update
                automatically.</span
              >
            </div>
            <div
              class="text-sm opacity-60 whitespace-normal max-w-[22rem] md:max-w-lg ml-2"
            >
              *You may see a warning when installing; if you're paranoid, you
              can build the installer yourself from the repo.
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
