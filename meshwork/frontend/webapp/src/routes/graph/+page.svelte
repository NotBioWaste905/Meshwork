<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import { LAST_GRAPH_KEY } from './lastGraph';

	let error = $state('');

	// Reopen the last graph, or create a new one.
	onMount(async () => {
		let graphId: string | null = null;
		try {
			graphId = localStorage.getItem(LAST_GRAPH_KEY);
		} catch {
			// storage unavailable, fall through to creating a graph
		}
		try {
			if (!graphId) {
				graphId = (await api.createGraph()).ID;
			}
			await goto(`/graph/${graphId}`, { replaceState: true });
		} catch (e) {
			console.error('Failed to create graph:', e);
			error = e instanceof Error ? e.message : String(e);
		}
	});
</script>

<div class="p-8 text-gray-600">
	{#if error}
		<p class="text-red-600">Failed to open a graph: {error}</p>
	{:else}
		<p>Opening graph…</p>
	{/if}
</div>
