<script lang="ts">
	import Avatar from './Avatar.svelte';
	import { graphStore } from '$lib/graphStore.svelte';

	let { id, size = 24, ring = false }: { id: string; size?: number; ring?: boolean } = $props();

	let actor = $derived(graphStore.actorsById.get(id));
</script>

<Avatar name={actor?.name ?? id} {size} {ring} agent={actor?.kind === 'agent'} />
