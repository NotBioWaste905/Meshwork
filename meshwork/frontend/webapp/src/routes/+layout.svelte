<script lang="ts">
	import '../app.css';
	import Rail from '$lib/components/Rail.svelte';

	let { children } = $props();
</script>

<div class="bg-canvas flex min-h-screen">
	<Rail />
	<div class="min-w-0 flex-1">
		{@render children()}
	</div>
</div>
