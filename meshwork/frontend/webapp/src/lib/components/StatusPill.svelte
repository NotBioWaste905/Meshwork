<script lang="ts">
	import type { Status } from '$lib/api';
	import { STATUS_META } from '$lib/status';

	let {
		status,
		size = 'sm',
		dot = false
	}: {
		status: Status;
		size?: 'xs' | 'sm' | 'lg';
		/** Show the status dot inside the pill (graph nodes, task header). */
		dot?: boolean;
	} = $props();

	let m = $derived(STATUS_META[status]);
</script>

<span
	class="inline-flex items-center gap-1.5 rounded-full font-semibold whitespace-nowrap"
	class:h-5={size === 'xs'}
	class:px-2={size === 'xs'}
	class:text-[11px]={size === 'xs'}
	class:tracking-wide={size === 'xs'}
	class:px-2.5={size === 'sm'}
	class:py-[3px]={size === 'sm'}
	class:text-xs={size === 'sm'}
	class:h-7={size === 'lg'}
	class:px-3={size === 'lg'}
	class:text-[13px]={size === 'lg'}
	style="background:{m.bg};color:{m.fg}"
>
	{#if dot}
		<span
			class="rounded-full"
			class:size-1.5={size === 'xs'}
			class:size-2={size !== 'xs'}
			style="background:{m.dot}"
		></span>
	{/if}
	{m.pill}
</span>
