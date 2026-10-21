<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import TopBar from '$lib/components/TopBar.svelte';
	import { rememberKnownGraph } from '$lib/graphStore.svelte';
	import { LAST_GRAPH_KEY } from '../../graph/lastGraph';

	let goal = $state('');
	let saving = $state(false);
	let error = $state('');

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (saving || !goal.trim()) return;
		saving = true;
		error = '';
		try {
			const { ID } = await api.createGraph();
			// The goal becomes the first task: tasks that depend on nothing yet hang off it later.
			await api.addTask(ID, { name: goal.trim() });
			rememberKnownGraph(ID);
			try {
				localStorage.setItem(LAST_GRAPH_KEY, ID);
			} catch {
				// storage unavailable
			}
			await goto(`/graph/${ID}`);
		} catch (e) {
			error = `Failed to create graph: ${e instanceof Error ? e.message : e}`;
			saving = false;
		}
	}
</script>

<svelte:head><title>New graph · Meshwork</title></svelte:head>

<main class="dot-grid flex min-h-screen flex-col">
	<TopBar>
		{#snippet left()}
			<a href="/graphs" class="text-muted no-underline">Graphs</a>
			<span class="text-faint">/</span>
			<strong class="font-semibold">New graph</strong>
		{/snippet}
	</TopBar>

	<div class="box-border flex flex-1 flex-col items-center justify-center gap-8 px-6 py-12">
		<svg
			viewBox="0 0 440 150"
			class="h-auto w-full max-w-[440px]"
			role="img"
			aria-label="Empty graph: a dashed placeholder task leading to a goal node"
			fill="none"
		>
			<defs>
				<marker
					id="em"
					viewBox="0 0 10 10"
					refX="9"
					refY="5"
					markerWidth="7"
					markerHeight="7"
					orient="auto"><path d="M0 0L10 5L0 10z" fill="#6B7684" /></marker
				>
			</defs>
			<g stroke="#8995A5" stroke-width="2" stroke-dasharray="6 5" marker-end="url(#em)">
				<path d="M130 40 C190 40 190 75 250 75" /><path d="M130 110 C190 110 190 75 250 75" />
			</g>
			<g font-family="IBM Plex Sans, sans-serif" font-size="14" font-weight="500">
				<rect
					x="10"
					y="20"
					width="120"
					height="40"
					rx="10"
					fill="#fff"
					stroke="#8995A5"
					stroke-width="1.5"
					stroke-dasharray="5 4"
				/><text x="28" y="45" fill="#5B6572">Task</text>
				<rect
					x="10"
					y="90"
					width="120"
					height="40"
					rx="10"
					fill="#fff"
					stroke="#8995A5"
					stroke-width="1.5"
					stroke-dasharray="5 4"
				/><text x="28" y="115" fill="#5B6572">Task</text>
				<rect x="250" y="47" width="180" height="56" rx="12" fill="#12161C" /><text
					x="270"
					y="80"
					fill="#fff"
					font-size="16"
					font-weight="600">Your goal</text
				>
			</g>
		</svg>

		<div class="flex max-w-[520px] flex-col gap-2.5 text-center">
			<h1 class="m-0 text-[32px] leading-tight font-semibold tracking-tight">
				Start with the goal
			</h1>
			<p class="text-ink-3 m-0 text-base leading-normal">
				Name the one thing everything else leads to. You will break it down into tasks and blockers
				next.
			</p>
		</div>

		<form class="flex w-full max-w-[520px] flex-wrap items-end gap-3" onsubmit={create}>
			<div class="flex flex-[1_1_280px] flex-col gap-1.5">
				<label for="goal" class="text-sm font-medium">Goal</label>
				<input
					id="goal"
					type="text"
					class="field h-12"
					placeholder="Ship the billing service"
					required
					bind:value={goal}
				/>
			</div>
			<button type="submit" class="btn btn-primary h-12 text-base" disabled={saving}>
				{saving ? 'Creating…' : 'Create graph'}
			</button>
			{#if error}
				<div class="bg-st-blocked-bg text-danger w-full rounded-lg p-3 text-sm" role="alert">
					{error}
				</div>
			{/if}
		</form>
	</div>
</main>
