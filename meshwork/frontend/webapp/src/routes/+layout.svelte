<script lang="ts">
	import '../app.css';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Rail from '$lib/components/Rail.svelte';
	import { session } from '$lib/session.svelte';

	let { children } = $props();

	// The backend refused a request without a valid token: sign in first, then come back.
	$effect(() => {
		if (session.signInRequired && !session.me && page.url.pathname !== '/me') {
			const next = page.url.pathname + page.url.search;
			goto(`/me?next=${encodeURIComponent(next)}`, { replaceState: true });
		}
	});
</script>

<div class="bg-canvas flex min-h-screen">
	<Rail />
	<div class="min-w-0 flex-1">
		{@render children()}
	</div>
</div>
