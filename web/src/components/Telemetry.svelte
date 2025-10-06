<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import Panel from "./Panel.svelte";
    import { DefaultService } from "../api";

    export let healthy: boolean;
    
    let pid: NodeJS.Timeout | undefined = undefined;
    let termals: Record<string, string> = {};
    let high_temp_threshold: number = 100;

    async function refresh() {
        let resp = await DefaultService.getThermal();
        if (resp.ok) {
            termals = resp.temps
        }
        return "";
    }

    onMount(async () => {
        let resp = await DefaultService.getConfig();
        if (resp.ok) {
            high_temp_threshold = resp.config.high_temperature_threshold
        }
        pid = setInterval(refresh, 1000);
    });
    onDestroy(() => {
        if (pid) clearInterval(pid);
    });
</script>

<Panel title="Telemetry" expandable={healthy}>
    <div class="text-sm opacity-80">
        Live temps and fan RPM read locally via the service. Nothing leaves your
        machine.
    </div>
    <ul class="list-disc list-inside text-sm opacity-80 space-y-1">
        <li>Temps and fan RPM</li>
        <li>AC/battery status and basic battery info</li>
        <li>Time‑series charts & live updates</li>
    </ul>
    <div slot="expended-only" class="flex flex-col">
        {#await refresh()}
            <div>Loading...</div>
        {:then ignored}
            <table>
                <thead>
                    <th class="text-left">Sensor</th>
                    <th class="text-right">Temperature</th>
                </thead>
                <tbody>
                    {#each Object.keys(termals) as sensor}
                        <tr>
                            <td class="text-left text-sm opacity-80">{sensor}</td>
                            {#if parseInt(termals[sensor]) > high_temp_threshold}
                                <td class="text-right text-sm text-red-500 bold">
                                    {termals[sensor] ?? "N/A"}
                                </td>
                            {:else}
                                <td class="text-right text-sm opacity-80">{termals[sensor] ?? "N/A"}</td>
                            {/if}
                        </tr>
                    {/each}
                </tbody>
            </table>
        {:catch error}
            <div>Error from service: {error}</div>
        {/await}
    </div>
</Panel>
