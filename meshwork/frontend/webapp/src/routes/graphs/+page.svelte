<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ApiError, type Task } from '$lib/api';
	import MockBadge from '$lib/components/MockBadge.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import {
		blockingOthers,
		forgetKnownGraph,
		knownGraphIds,
		summarize,
		type GraphSummary
	} from '$lib/graphStore.svelte';
	import { MOCK_GRAPHS } from '$lib/mock';
	import { shortId } from '$lib/status';

	const COLORS = ['#2349C7', '#0F766E', '#7A3FC4', '#B4530A'];

	let graphs = $state<GraphSummary[]>([]);
	let waiting = $state<{ task: Task; blocks: number; graph: GraphSummary }[]>([]);
	let loading = $state(true);
	let query = $state('');

	let matching = $derived(
		graphs.filter((g) => g.name.toLowerCase().includes(query.trim().toLowerCase()))
	);
	let matchingMock = $derived(
		MOCK_GRAPHS.filter((g) => g.name.toLowerCase().includes(query.trim().toLowerCase()))
	);

	onMount(async () => {
		// There is no "list graphs" endpoint: show the graphs this browser has opened.
		const loaded = await Promise.all(
			knownGraphIds().map(async (id) => {
				try {
					return { id, tasks: await api.getAllTasks(id) };
				} catch (error) {
					if (error instanceof ApiError && error.status === 404) forgetKnownGraph(id);
					return null;
				}
			})
		);
		const found = loaded.filter((g) => g !== null);
		graphs = found.map((g) => summarize(g.id, g.tasks));
		waiting = found.flatMap((g, i) =>
			blockingOthers(g.tasks).map((w) => ({ ...w, graph: graphs[i] }))
		);
		loading = false;
	});

	const pct = (done: number, total: number) => (total ? Math.round((done / total) * 100) : 0);
</script>

<svelte:head><title>Graphs · Meshwork</title></svelte:head>

{#snippet card(
	name: string,
	goal: string,
	done: number,
	total: number,
	blocked: number,
	color: string,
	footer: string
)}
	<svg viewBox="0 0 260 70" class="h-[70px] w-full" aria-hidden="true" fill="none">
		<path
			d="M30 35 H80 M80 35 C100 35 100 15 120 15 H150 M80 35 C100 35 100 55 120 55 H150 M150 15 C170 15 170 35 190 35 M150 55 C170 55 170 35 190 35 M190 35 H226"
			stroke="#B9C1CB"
			stroke-width="2"
		/>
		<circle cx="24" cy="35" r="7" fill={color} /><circle cx="80" cy="35" r="7" fill={color} />
		<circle cx="120" cy="15" r="7" fill={color} opacity="0.7" /><circle
			cx="120"
			cy="55"
			r="7"
			fill={color}
			opacity="0.7"
		/>
		<circle cx="150" cy="15" r="7" fill="#fff" stroke="#8995A5" stroke-width="2" /><circle
			cx="150"
			cy="55"
			r="7"
			fill="#fff"
			stroke="#8995A5"
			stroke-width="2"
		/>
		<circle cx="190" cy="35" r="7" fill="#fff" stroke="#8995A5" stroke-width="2" /><rect
			x="226"
			y="26"
			width="26"
			height="18"
			rx="6"
			fill="#12161C"
		/>
	</svg>
	<div class="flex flex-col gap-1">
		<div class="text-lg font-semibold">{name}</div>
		<div class="text-muted text-sm">Goal: {goal}</div>
	</div>
	<div class="flex flex-col gap-2">
		<div class="text-ink-3 flex justify-between text-[13px]">
			<span>{done} of {total} done</span><span>{pct(done, total)}%</span>
		</div>
		<div class="bg-surface-2 h-1.5 overflow-hidden rounded-[3px]">
			<div class="h-full" style="width:{pct(done, total)}%;background:{color}"></div>
		</div>
	</div>
	<div class="flex items-center justify-between">
		<span
			class="rounded-full px-2.5 py-1 text-[13px] font-medium {blocked
				? 'bg-st-blocked-bg text-st-blocked-fg'
				: 'bg-st-done-bg text-st-done-fg'}">{blocked ? `${blocked} blocked` : 'On track'}</span
		>
		<span class="text-muted text-[13px]">{footer}</span>
	</div>
{/snippet}

<main class="box-border flex flex-col gap-8 px-10 pt-8 pb-12">
	<header class="flex flex-wrap items-center justify-between gap-4">
		<div class="flex flex-col gap-1">
			<h1 class="m-0 text-3xl font-semibold tracking-tight">Graphs</h1>
			<p class="text-muted m-0 text-[15px]">
				Each graph is one goal and every task that leads to it.
			</p>
		</div>
		<div class="flex flex-wrap items-center gap-3">
			<label class="relative block">
				<span class="sr-only">Search graphs</span>
				<input
					type="search"
					class="field w-60 text-[15px]"
					placeholder="Search graphs"
					bind:value={query}
				/>
			</label>
			<a href="/graphs/new" class="btn btn-primary">
				<svg
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="#fff"
					stroke-width="2.2"
					stroke-linecap="round"
					aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg
				>New graph
			</a>
		</div>
	</header>

	<section aria-labelledby="attn" class="card flex flex-col gap-3 px-6 py-5">
		<div class="flex items-baseline gap-3">
			<h2 id="attn" class="m-0 text-[17px] font-semibold">Waiting on you</h2>
			<span class="text-muted text-sm">
				{waiting.length === 0
					? 'Nothing you own is blocking someone else'
					: `${waiting.length} ${waiting.length === 1 ? 'task is' : 'tasks are'} blocking someone else`}
			</span>
		</div>
		{#if waiting.length > 0}
			<ul class="m-0 flex list-none flex-col p-0">
				{#each waiting as w (w.task.id)}
					<li class="border-line-soft flex flex-wrap items-center gap-x-4 gap-y-3 border-t py-3">
						<span class="text-muted w-14 font-mono text-[13px]">{shortId(w.task)}</span>
						<a
							href="/graph/{w.graph.id}/task/{w.task.id}"
							class="text-ink min-w-0 flex-[1_1_260px] text-[15px] font-medium no-underline"
							>{w.task.name}</a
						>
						<span class="text-muted text-sm">{w.graph.name}</span>
						<StatusPill status={w.task.status} />
						<span class="text-danger min-w-28 text-right text-sm font-medium"
							>Blocks {w.blocks} {w.blocks === 1 ? 'task' : 'tasks'}</span
						>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<section
		aria-label="All graphs"
		class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-5"
	>
		{#each matching as g, i (g.id)}
			<a
				href="/graph/{g.id}"
				class="card text-ink hover:border-line-strong flex flex-col gap-4 p-5 no-underline"
			>
				{@render card(g.name, g.goal, g.done, g.total, g.blocked, COLORS[i % COLORS.length], '')}
			</a>
		{/each}
		{#if !loading && graphs.length === 0}
			<div class="card col-span-full flex flex-col items-start gap-2 p-6">
				<strong class="font-semibold">No graphs opened in this browser yet</strong>
				<span class="text-muted text-sm"
					>The API cannot list graphs, so only graphs you opened here appear.</span
				>
				<a href="/graphs/new" class="btn btn-primary btn-sm mt-2">Create the first graph</a>
			</div>
		{/if}
	</section>

	{#if matchingMock.length > 0}
		<section aria-label="Example graphs" class="flex flex-col gap-3">
			<h2 class="text-ink-3 m-0 flex items-center gap-2 text-[15px] font-semibold">
				Examples <MockBadge note="Demo graphs: the API has no list of graphs" />
			</h2>
			<div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-5 opacity-80">
				{#each matchingMock as g (g.name)}
					<div class="card flex flex-col gap-4 p-5">
						{@render card(g.name, g.goal, g.done, g.total, g.blocked, g.color, g.updated)}
					</div>
				{/each}
			</div>
		</section>
	{/if}
</main>
