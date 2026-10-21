<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import Rail from '$lib/components/Rail.svelte';

	let { children } = $props();

	// The sign-in screen is full-bleed, everything else lives in the app shell.
	let bare = $derived(page.url.pathname === '/login');
</script>

{#if bare}
	{@render children()}
{:else}
	<div class="bg-canvas flex min-h-screen">
		<Rail />
		<div class="min-w-0 flex-1">
			{@render children()}
		</div>
	</div>
{/if}
