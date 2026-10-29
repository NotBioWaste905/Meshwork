<script lang="ts">
	import TopBar from './TopBar.svelte';
	import ActorAvatar from './ActorAvatar.svelte';
	import { graphStore } from '$lib/graphStore.svelte';

	let { view }: { view: 'graph' | 'board' | 'table' } = $props();

	let base = $derived(`/graph/${graphStore.graphId}`);
	let tabs = $derived([
		{ id: 'graph', label: 'Graph', href: base },
		{ id: 'board', label: 'Board', href: `${base}/board` },
		{ id: 'table', label: 'Table', href: `${base}/table` }
	]);
	const MAX_AVATARS = 4;
	let shown = $derived(graphStore.users.slice(0, MAX_AVATARS));
	let hidden = $derived(graphStore.users.slice(MAX_AVATARS));

	// The task key prefix (`MW` in `MW-12`), edited in place.
	let editingKey = $state(false);
	let keyDraft = $state('');
	function startKeyEdit() {
		keyDraft = '';
		editingKey = true;
	}
	async function saveKey() {
		const key = keyDraft.trim().toUpperCase();
		if (key && key !== graphStore.graph?.key && !(await graphStore.setKey(key))) return;
		editingKey = false;
	}
	function focus(el: HTMLInputElement) {
		el.focus();
	}
</script>

<TopBar>
	{#snippet left()}
		<a href="/graphs" class="text-muted no-underline">Graphs</a>
		<span class="text-faint">/</span>
		<strong class="font-semibold">{graphStore.name}</strong>
		{#if editingKey}
			<form
				class="contents"
				onsubmit={(e) => {
					e.preventDefault();
					saveKey();
				}}
			>
				<input
					use:focus
					class="border-line-strong bg-surface h-6 w-20 rounded-md border px-1.5 font-mono text-xs uppercase"
					aria-label="Task key prefix"
					placeholder={graphStore.graph?.key}
					maxlength="10"
					pattern="[A-Za-z][A-Za-z0-9]*"
					title="1-10 letters and digits, starting with a letter"
					bind:value={keyDraft}
					onblur={() => (editingKey = false)}
					onkeydown={(e) => e.key === 'Escape' && (editingKey = false)}
				/>
			</form>
		{:else if graphStore.graph?.key}
			<button
				type="button"
				class="bg-surface-2 text-ink-3 hover:text-ink h-6 cursor-pointer rounded-md border-0 px-1.5 font-mono text-xs"
				title="Task keys look like {graphStore.graph.key}-1 · click to change the prefix"
				onclick={startKeyEdit}>{graphStore.graph.key}</button
			>
		{/if}
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
		{#if shown.length > 0}
			<div class="flex" role="group" aria-label="Assignees in this graph">
				{#each shown as user, i (user)}
					<span class={i > 0 ? '-ml-2' : ''}><ActorAvatar id={user} size={30} ring /></span>
				{/each}
				{#if hidden.length > 0}
					<span
						class="bg-surface-2 text-ink-3 -ml-2 inline-flex size-[30px] items-center justify-center rounded-full border-2 border-white text-[11px] font-semibold"
						title={hidden.map((id) => graphStore.actorName(id)).join(', ')}>+{hidden.length}</span
					>
				{/if}
			</div>
		{/if}
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
