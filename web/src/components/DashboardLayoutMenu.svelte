<script lang="ts">
    import Icon from "@iconify/svelte";
    import { createEventDispatcher } from "svelte";

    export let editing = false;

    const dispatch = createEventDispatcher<{
        toggleEdit: void;
    }>();

    $: triggerLabel = editing
        ? "Finish customizing the dashboard"
        : "Customize dashboard layout";
</script>

<button
    type="button"
    class="btn btn-ghost btn-sm mr-0"
    class:btn-active={editing}
    aria-label={triggerLabel}
    aria-pressed={editing}
    title={triggerLabel}
    on:click={() => dispatch("toggleEdit")}
>
    <Icon
        icon={editing
            ? "mdi:view-dashboard-edit"
            : "mdi:view-dashboard-outline"}
        class="w-5 h-5"
    />
</button>
