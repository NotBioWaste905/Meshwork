<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import NewTaskDialog from '$lib/components/NewTaskDialog.svelte';
	import { graphStore } from '$lib/graphStore.svelte';
	import { liveRefresh } from '$lib/liveRefresh';
	import { LAST_GRAPH_KEY } from '../lastGraph';

	let { children } = $props();

	let graphId = $derived(page.params.id ?? '');

	$effect(() => {
		const id = graphId; // tracked: reload when navigating to another graph
		untrack(() => graphStore.load(id));
	});

	// Pick up changes made by agents (MCP/REST): poll while visible, refresh on return to the tab.
	$effect(() => liveRefresh(() => graphStore.refresh()));

	// The new-task dialog is an unsaved form: don't refresh under it.
	$effect(() => {
		if (graphStore.newTaskOpen) return graphStore.hold();
	});

	$effect(() => {
		if (!graphStore.loaded) return;
		try {
			if (graphStore.notFound) {
				if (localStorage.getItem(LAST_GRAPH_KEY) === graphId)
					localStorage.removeItem(LAST_GRAPH_KEY);
			} else {
				localStorage.setItem(LAST_GRAPH_KEY, graphId);
			}
		} catch {
			// storage unavailable, nothing to remember
		}
		// Unknown graph: let /graphs show the list instead.
		if (graphStore.notFound) goto('/graphs', { replaceState: true });
	});
</script>

{@render children()}
<NewTaskDialog />
