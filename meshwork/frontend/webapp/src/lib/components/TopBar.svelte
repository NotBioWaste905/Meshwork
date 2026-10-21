<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		left,
		center,
		right
	}: {
		left?: Snippet;
		center?: Snippet;
		right?: Snippet;
	} = $props();
</script>

<header
	class="border-line bg-surface box-border flex min-h-[60px] flex-none flex-wrap items-center gap-x-6 gap-y-2 border-b px-6 py-1.5"
>
	{#if left}<div class="flex items-center gap-2 text-[15px]">{@render left()}</div>{/if}
	{@render center?.()}
	<div class="flex-1"></div>
	{@render right?.()}
</header>
