<script lang="ts">
	import { Priority } from '$lib/api';
	import { PRIORITY_LABEL } from '$lib/priority';

	/** Marks high and low priority tasks; normal ones show nothing. */
	let { priority, dark = false }: { priority: Priority; dark?: boolean } = $props();
</script>

{#if priority !== Priority.NORMAL}
	<span
		class="inline-flex items-center text-[11px] font-semibold whitespace-nowrap"
		style="color:{priority === Priority.HIGH
			? dark
				? 'var(--color-danger-dot)'
				: 'var(--color-danger)'
			: dark
				? 'var(--color-faint)'
				: 'var(--color-muted)'}"
		title="{PRIORITY_LABEL[priority]} priority"
		>{priority === Priority.HIGH ? '↑' : '↓'} {PRIORITY_LABEL[priority]}</span
	>
{/if}
