<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type Graph, type Task } from '$lib/api';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { waitingReason } from '$lib/graphStore.svelte';
	import { liveRefresh, mergeById } from '$lib/liveRefresh';
	import { session } from '$lib/session.svelte';
	import { shortId } from '$lib/status';
	import { ago, formatDateTime } from '$lib/time';

	const COLORS = ['#2349C7', '#0F766E', '#7A3FC4', '#B4530A'];

	let graphs = $state<Graph[]>([]);
	let loading = $state(true);
	let error = $state('');
	let query = $state('');

	/** Graphs created before graphs had names fall back to their short ID. */
	const title = (g: Graph) => g.name || `Graph ${g.id.slice(0, 8)}`;

	let matching = $derived(
		graphs.filter((g) => title(g).toLowerCase().includes(query.trim().toLowerCase()))
	);

	async function load() {
		try {
			// unchanged graphs keep their objects, so a quiet poll re-renders nothing
			graphs = mergeById(graphs, await api.listGraphs());
			error = '';
		} catch (e) {
			// a failed background poll keeps showing the list it has
			if (loading || graphs.length === 0) {
				error = `Failed to load graphs: ${e instanceof Error ? e.message : e}`;
			} else {
				console.warn('Background refresh failed:', e);
			}
		}
		loading = false;
	}

	// Agents create graphs and move tasks too: poll while visible, refresh on return to the tab.
	onMount(() => {
		load();
		return liveRefresh(load);
	});

	/** Open work others wait for from the signed-in actor, across all graphs. */
	let waiting = $state<{ task: Task; reason: string; graph: Graph }[]>([]);

	// Follows the graph list (polled) and the signed-in actor; empty when signed out.
	$effect(() => {
		const me = session.me;
		const list = graphs;
		if (!me) {
			waiting = [];
			return;
		}
		let stale = false;
		Promise.all(list.map(async (graph) => ({ graph, tasks: await api.getAllTasks(graph.id) })))
			.then((loaded) => {
				if (stale) return;
				waiting = loaded.flatMap(({ graph, tasks }) =>
					tasks.flatMap((task) => {
						const reason = waitingReason(task, tasks);
						return reason ? [{ task, reason, graph }] : [];
					})
				);
			})
			.catch((e) => console.warn('Cannot load tasks for Waiting on you:', e));
		return () => {
			stale = true;
		};
	});

	const pct = (done: number, total: number) => (total ? Math.round((done / total) * 100) : 0);
</script>

<svelte:head><title>Graphs · Meshwork</title></svelte:head>

{#snippet card(
	name: string,
	done: number,
	total: number,
	blocked: number,
	color: string,
	id: string,
	updatedAt: number | null
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
		<div class="text-muted text-xs font-medium tracking-wide uppercase">Goal</div>
		<div class="text-lg font-semibold">{name}</div>
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
		{#if updatedAt}
			<time
				class="text-muted text-[13px]"
				datetime={new Date(updatedAt).toISOString()}
				title="{formatDateTime(updatedAt)} · graph {id.slice(0, 8)}">Updated {ago(updatedAt)}</time
			>
		{:else}
			<span class="text-muted font-mono text-[13px]" title="Graph ID">{id.slice(0, 8)}</span>
		{/if}
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

	{#if session.me}
		<section aria-labelledby="attn" class="card flex flex-col gap-3 px-6 py-5">
			<div class="flex flex-wrap items-baseline gap-3">
				<h2 id="attn" class="m-0 text-[17px] font-semibold">Waiting on you</h2>
				<span class="text-muted text-sm">
					{waiting.length === 0
						? 'Nothing of yours is holding anyone up'
						: `${waiting.length} ${waiting.length === 1 ? 'task' : 'tasks'}`}
				</span>
			</div>
			{#if waiting.length > 0}
				<ul class="m-0 flex list-none flex-col p-0">
					{#each waiting as w (w.task.id)}
						<li class="border-line-soft flex flex-wrap items-center gap-x-4 gap-y-2 border-t py-3">
							<span class="text-muted w-14 font-mono text-[13px]">{shortId(w.task)}</span>
							<a
								href="/graph/{w.graph.id}/task/{w.task.id}"
								class="text-ink min-w-0 flex-[1_1_260px] text-[15px] font-medium no-underline"
								>{w.task.name}</a
							>
							<span class="text-muted text-sm">{title(w.graph)}</span>
							<StatusPill status={w.task.status} />
							<span class="text-danger min-w-36 text-right text-sm font-medium">{w.reason}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}

	<section
		aria-label="All graphs"
		class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-5"
	>
		{#each matching as g, i (g.id)}
			<a
				href="/graph/{g.id}"
				class="card text-ink hover:border-line-strong flex flex-col gap-4 p-5 no-underline"
			>
				{@render card(
					title(g),
					g.done_count,
					g.task_count,
					g.blocked_count,
					COLORS[i % COLORS.length],
					g.id,
					g.updated_at
				)}
			</a>
		{/each}
		{#if error}
			<div class="bg-st-blocked-bg text-danger col-span-full rounded-lg p-3 text-sm" role="alert">
				{error}
			</div>
		{:else if !loading && graphs.length === 0}
			<div class="card col-span-full flex flex-col items-start gap-2 p-6">
				<strong class="font-semibold">No graphs yet</strong>
				<span class="text-muted text-sm">A graph is one goal and every task that leads to it.</span>
				<a href="/graphs/new" class="btn btn-primary btn-sm mt-2">Create the first graph</a>
			</div>
		{:else if !loading && matching.length === 0}
			<p class="text-muted col-span-full m-0 text-sm">No graphs match “{query.trim()}”.</p>
		{/if}
	</section>
</main>
