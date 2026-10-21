<script lang="ts">
	import TopBar from './TopBar.svelte';
	import Avatar from './Avatar.svelte';
	import MockBadge from './MockBadge.svelte';
	import { graphStore } from '$lib/graphStore.svelte';
	import { MEMBERS } from '$lib/mock';

	let { view }: { view: 'graph' | 'board' | 'table' } = $props();

	let base = $derived(`/graph/${graphStore.graphId}`);
	let tabs = $derived([
		{ id: 'graph', label: 'Graph', href: base },
		{ id: 'board', label: 'Board', href: `${base}/board` },
		{ id: 'table', label: 'Table', href: `${base}/table` }
	]);
</script>

<TopBar>
	{#snippet left()}
		<a href="/graphs" class="text-muted no-underline">Graphs</a>
		<span class="text-faint">/</span>
		<strong class="font-semibold">{graphStore.name}</strong>
	{/snippet}
	{#snippet center()}
		<div
			role="tablist"
			aria-label="Projection"
			class="bg-surface-2 flex gap-0.5 rounded-[10px] p-[3px]"
		>
			{#each tabs as tab (tab.id)}
				<a
					href={tab.href}
					role="tab"
					aria-selected={view === tab.id}
					aria-current={view === tab.id ? 'page' : undefined}
					class="flex h-8 items-center rounded-lg px-4 text-sm no-underline {view === tab.id
						? 'bg-surface text-ink font-semibold shadow-[0_1px_2px_rgba(18,22,28,.12)]'
						: 'text-ink-3 font-medium'}">{tab.label}</a
				>
			{/each}
		</div>
	{/snippet}
	{#snippet right()}
		<div class="flex items-center gap-2" aria-label="Members">
			<div class="flex">
				{#each MEMBERS as member, i (member.name)}
					<span class={i > 0 ? '-ml-2' : ''}><Avatar name={member.name} size={30} ring /></span>
				{/each}
			</div>
			<MockBadge note="Members: demo data, the API has no users yet" />
		</div>
		<button
			type="button"
			class="btn btn-primary btn-sm"
			onclick={() => (graphStore.newTaskOpen = true)}
		>
			<svg
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="#fff"
				stroke-width="2.4"
				stroke-linecap="round"
				aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg
			>New task
		</button>
	{/snippet}
</TopBar>
