<script lang="ts">
	import { untrack } from 'svelte';
	import {
		SvelteFlow,
		SvelteFlowProvider,
		Background,
		BackgroundVariant,
		MiniMap,
		MarkerType,
		type Connection,
		type NodeTypes
	} from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import Dagre from '@dagrejs/dagre';

	import TaskNode from '../TaskNode.svelte';
	import GraphControls from '../GraphControls.svelte';
	import GraphHeader from '$lib/components/GraphHeader.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { Status, type Task } from '$lib/api';
	import { criticalPath, graphStore, type TaskFilter } from '$lib/graphStore.svelte';
	import { shortId, STATUS_META } from '$lib/status';
	import { NODE_HEIGHT, NODE_WIDTH, type TaskFlowEdge, type TaskFlowNode } from '../types';

	const nodeTypes: NodeTypes = { task: TaskNode };

	const GREY = '#6B7684';
	const BLUE = '#2349C7';

	let nodes = $state.raw<TaskFlowNode[]>([]);
	let edges = $state.raw<TaskFlowEdge[]>([]);
	let selectedId = $state<string | null>(null);
	let showCritical = $state(false);

	let selected = $derived(selectedId ? graphStore.byId.get(selectedId) : undefined);
	let terminal = $derived(graphStore.terminals.length === 1 ? graphStore.terminals[0] : undefined);
	let chips = $derived<{ id: TaskFilter; label: string; count: number }[]>([
		{ id: 'all', label: 'All tasks', count: graphStore.tasks.length },
		{ id: 'blocked', label: 'Blocked', count: graphStore.blockedCount }
	]);

	function computeLayout(tasks: Task[]): Map<string, { x: number; y: number }> {
		const g = new Dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
		g.setGraph({ rankdir: 'LR', ranksep: 60, nodesep: 36 });
		const ids = new Set(tasks.map((t) => t.id));
		for (const task of tasks) g.setNode(task.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
		for (const task of tasks) {
			for (const dep of task.depends_on) if (ids.has(dep)) g.setEdge(dep, task.id);
		}
		Dagre.layout(g);
		return new Map(
			tasks.map((task) => {
				const p = g.node(task.id);
				// dagre positions are centres, svelte-flow wants the top-left corner
				return [task.id, { x: p.x - NODE_WIDTH / 2, y: p.y - NODE_HEIGHT / 2 }];
			})
		);
	}

	function rebuild(tasks: Task[], selectedTaskId: string | null, critical: boolean) {
		const previous = new Map(nodes.map((n) => [n.id, n.position]));
		const fresh = previous.size === 0 ? computeLayout(tasks) : null;
		const maxY = Math.max(0, ...[...previous.values()].map((p) => p.y));
		let added = 0;
		const dependents = graphStore.dependents;

		nodes = tasks.map((task) => {
			let position = previous.get(task.id) ?? fresh?.get(task.id);
			if (!position) position = { x: 40 + 200 * added++, y: maxY + NODE_HEIGHT + 40 };
			return {
				id: task.id,
				type: 'task',
				position,
				data: {
					...task,
					terminal: task.depends_on.length > 0 && !dependents.has(task.id),
					chore: task.tags.includes('chore'),
					selected: task.id === selectedTaskId,
					dim: !graphStore.matches(task)
				}
			};
		});

		const path = critical ? criticalPath(tasks) : [];
		const onPath = new Set(path.slice(1).map((id, i) => `${path[i]}>${id}`));
		const ids = new Set(tasks.map((t) => t.id));
		edges = tasks.flatMap((task) =>
			task.depends_on
				.filter((dep) => ids.has(dep))
				.map((dep) => {
					const emphasised = task.id === selectedTaskId || onPath.has(`${dep}>${task.id}`);
					const color = emphasised ? BLUE : GREY;
					return {
						id: `edge-${dep}-${task.id}`,
						source: dep,
						target: task.id,
						markerEnd: { type: MarkerType.ArrowClosed, color, width: 14, height: 14 },
						style: `stroke:${color};stroke-width:${emphasised ? 2.5 : 1.75}`
					};
				})
		);
	}

	// Rebuild from the shared store; positions of existing nodes are kept.
	$effect(() => {
		const tasks = graphStore.tasks;
		void graphStore.filter;
		const sel = selectedId;
		const critical = showCritical;
		untrack(() => rebuild(tasks, sel, critical));
	});

	// A different graph starts from a fresh layout.
	let layoutFor = '';
	$effect(() => {
		const id = graphStore.graphId;
		if (id !== layoutFor) {
			layoutFor = id;
			nodes = [];
			selectedId = null;
		}
	});

	function autoLayout() {
		const positions = computeLayout(graphStore.tasks);
		nodes = nodes.map((n) => ({ ...n, position: positions.get(n.id) ?? n.position }));
	}

	function onConnect(params: Connection) {
		graphStore.connect(params.source, params.target);
	}

	function onEdgeClick({ edge }: { edge: TaskFlowEdge }) {
		const dep = graphStore.byId.get(edge.source);
		const task = graphStore.byId.get(edge.target);
		if (!dep || !task) return;
		if (confirm(`Remove the dependency "${dep.name}" → "${task.name}"?`)) {
			graphStore.disconnect(edge.source, edge.target);
		}
	}

	function minimapColor(node: { id: string }): string {
		if (node.id === selectedId) return BLUE;
		const task = graphStore.byId.get(node.id);
		if (!task) return '#B9C1CB';
		const isTerminal = task.depends_on.length > 0 && !graphStore.dependents.has(task.id);
		return isTerminal ? '#12161C' : task.status === Status.DONE ? '#1E9E63' : '#B9C1CB';
	}

	function blockedNote(task: Task): string {
		const open = graphStore.openBlockers(task);
		if (task.depends_on.length === 0) return 'No dependencies.';
		if (open.length === 0) return 'All dependencies are done.';
		return `${task.status === Status.BLOCKED ? 'Blocked: w' : 'W'}aiting on ${open.length} of ${task.depends_on.length} dependencies.`;
	}
</script>

<svelte:head><title>{graphStore.name} · Meshwork</title></svelte:head>

<div class="flex h-screen flex-col overflow-hidden">
	<GraphHeader view="graph" />

	<div
		class="border-line bg-canvas flex h-[52px] flex-none items-center gap-2 border-b px-6"
		role="toolbar"
		aria-label="Filters"
	>
		{#each chips as chip (chip.id)}
			<button
				type="button"
				class="chip"
				aria-pressed={graphStore.filter === chip.id}
				style={chip.id === 'blocked' && graphStore.filter !== 'blocked'
					? 'color:var(--color-danger)'
					: ''}
				onclick={() => (graphStore.filter = chip.id)}>{chip.label} · {chip.count}</button
			>
		{/each}
		<button
			type="button"
			class="chip"
			aria-pressed={showCritical}
			onclick={() => (showCritical = !showCritical)}>Critical path</button
		>
		<div class="flex-1"></div>
		{#if graphStore.error}
			<span class="text-danger text-[13px]">{graphStore.error}</span>
		{:else}
			<span class="text-muted text-[13px]">
				{#if terminal}Terminal node: {terminal.name} ·
				{/if}{graphStore.doneCount} of {graphStore.tasks.length} done
			</span>
		{/if}
	</div>

	<div class="relative min-h-0 flex-1 overflow-hidden">
		<SvelteFlowProvider>
			<SvelteFlow
				bind:nodes
				bind:edges
				{nodeTypes}
				fitView
				deleteKey={null}
				onconnect={onConnect}
				onedgeclick={onEdgeClick}
				onnodeclick={({ node }) => (selectedId = node.id)}
				onpaneclick={() => (selectedId = null)}
				proOptions={{ hideAttribution: true }}
			>
				<Background
					variant={BackgroundVariant.Dots}
					gap={22}
					size={1.2}
					bgColor="#F4F6F8"
					patternColor="#C9D0D8"
				/>
				<GraphControls onLayout={autoLayout} />
				<MiniMap
					width={180}
					height={112}
					nodeColor={minimapColor}
					nodeBorderRadius={2}
					maskStrokeColor={BLUE}
					style="border:1px solid var(--color-line);border-radius:10px;background:#fff"
				/>
			</SvelteFlow>
		</SvelteFlowProvider>

		{#if graphStore.loaded && graphStore.tasks.length === 0}
			<div class="pointer-events-none absolute inset-0 flex items-center justify-center">
				<div class="card pointer-events-auto max-w-sm p-6 text-center">
					<h2 class="m-0 text-lg font-semibold">No tasks yet</h2>
					<p class="text-muted my-2 text-sm">
						Add the first task, then draw blockers between nodes.
					</p>
					<button
						type="button"
						class="btn btn-primary btn-sm"
						onclick={() => (graphStore.newTaskOpen = true)}>New task</button
					>
				</div>
			</div>
		{/if}

		{#if selected}
			<div
				class="border-line bg-surface absolute bottom-[72px] left-1/2 flex w-[300px] -translate-x-1/2 flex-col gap-2.5 rounded-xl border px-4 py-3.5 shadow-[0_8px_24px_rgba(18,22,28,.12)]"
			>
				<div class="flex items-baseline justify-between gap-2">
					<strong class="text-sm font-semibold">{selected.name}</strong>
					<span class="text-muted font-mono text-xs">{shortId(selected)}</span>
				</div>
				<div class="text-ink-3 flex items-center gap-2 text-[13px] leading-normal">
					<StatusPill status={selected.status} size="xs" dot />{blockedNote(selected)}
				</div>
				{#if selected.depends_on.length > 0}
					<div class="flex flex-col gap-1.5 text-[13px]">
						{#each selected.depends_on as id (id)}
							{@const dep = graphStore.byId.get(id)}
							{#if dep}
								<div class="flex justify-between gap-2">
									<span class="truncate">{shortId(dep)} {dep.name}</span>
									<span
										class="font-medium whitespace-nowrap"
										style="color:{STATUS_META[dep.status].fg}">{STATUS_META[dep.status].label}</span
									>
								</div>
							{/if}
						{/each}
					</div>
				{/if}
				<a
					href="/graph/{graphStore.graphId}/task/{selected.id}"
					class="text-sm font-semibold no-underline">Open task →</a
				>
			</div>
		{/if}

		<div
			class="border-line bg-surface text-ink-3 pointer-events-none absolute bottom-4 left-[420px] z-5 hidden h-10 items-center gap-5 rounded-[10px] border px-4 text-[13px] xl:flex"
		>
			<span class="flex items-center gap-2"
				><svg width="32" height="8" aria-hidden="true"
					><path d="M0 4H32" stroke="#6B7684" stroke-width="2" /></svg
				>Blocks</span
			>
			<span class="flex items-center gap-2 opacity-50" title="Not in the data model yet"
				><svg width="32" height="8" aria-hidden="true"
					><path d="M0 4H32" stroke="#6B7684" stroke-width="2" stroke-dasharray="6 4" /></svg
				>Part of</span
			>
			<span class="flex items-center gap-2 opacity-50" title="Not in the data model yet"
				><svg width="32" height="8" aria-hidden="true"
					><path
						d="M0 4H32"
						stroke="#0F766E"
						stroke-width="2"
						stroke-dasharray="2 4"
						stroke-linecap="round"
					/></svg
				>Affects (other graph)</span
			>
		</div>
	</div>
</div>

<style>
	:global(.svelte-flow) {
		--xy-background-color: var(--color-canvas);
	}
	:global(.svelte-flow__minimap) {
		margin: 16px;
	}
	:global(.svelte-flow__panel.bottom.left) {
		margin: 16px;
	}
</style>
