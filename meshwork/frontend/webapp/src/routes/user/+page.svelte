<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ApiError, Status, type Task } from '$lib/api';
	import Avatar from '$lib/components/Avatar.svelte';
	import MockBadge from '$lib/components/MockBadge.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import {
		describeRelations,
		forgetKnownGraph,
		isMine,
		knownGraphIds,
		summarize
	} from '$lib/graphStore.svelte';
	import { CURRENT_USER, MOCK_RECENT_ACTIVITY } from '$lib/mock';
	import { shortId, STATUS_META } from '$lib/status';

	type Row = { task: Task; graphId: string; graph: string; note: string; warn: boolean };

	const COLORS = ['#2349C7', '#0F766E', '#7A3FC4', '#B4530A'];
	const GROUPS: { status: Status; title: string }[] = [
		{ status: Status.BLOCKED, title: 'Blocked' },
		{ status: Status.IN_PROGRESS, title: 'In progress' },
		{ status: Status.TODO, title: 'To do' },
		{ status: Status.REVIEW, title: 'In review' }
	];

	let rows = $state<Row[]>([]);
	let loading = $state(true);
	let tab = $state<'assigned' | 'done'>('assigned');
	let graphFilter = $state('');

	let mine = $derived(rows.filter((r) => !graphFilter || r.graphId === graphFilter));
	let assigned = $derived(mine.filter((r) => r.task.status !== Status.DONE));
	let done = $derived(mine.filter((r) => r.task.status === Status.DONE));
	let graphs = $derived([...new Map(rows.map((r) => [r.graphId, r.graph])).entries()]);
	let workload = $derived.by(() => {
		const counts = new Map<string, { name: string; n: number }>();
		for (const r of rows.filter((r) => r.task.status !== Status.DONE)) {
			const entry = counts.get(r.graphId) ?? { name: r.graph, n: 0 };
			entry.n++;
			counts.set(r.graphId, entry);
		}
		const list = [...counts.values()].sort((a, b) => b.n - a.n);
		const max = Math.max(1, ...list.map((l) => l.n));
		return list.map((l, i) => ({ ...l, pct: (l.n / max) * 100, color: COLORS[i % COLORS.length] }));
	});

	let stats = $derived([
		{ value: String(assigned.length), label: 'Assigned', color: 'var(--color-ink)' },
		{ value: '—', label: 'To review', color: 'var(--color-st-review-fg)' },
		{
			value: String(assigned.filter((r) => r.task.status === Status.BLOCKED).length),
			label: 'Blocked',
			color: 'var(--color-danger)'
		},
		{ value: String(done.length), label: 'Done', color: 'var(--color-st-done-fg)' }
	]);

	let list = $derived(tab === 'assigned' ? assigned : done);
	let groups = $derived(
		tab === 'assigned'
			? GROUPS.map((g) => ({ ...g, rows: assigned.filter((r) => r.task.status === g.status) }))
			: [{ status: Status.DONE, title: 'Done', rows: done }]
	);

	onMount(async () => {
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
		rows = loaded
			.filter((g) => g !== null)
			.flatMap((g) => {
				const graph = summarize(g.id, g.tasks).name;
				return g.tasks.filter(isMine).map((task) => {
					const note = describeRelations(task, g.tasks);
					return { task, graphId: g.id, graph, note: note.text, warn: note.warn };
				});
			});
		loading = false;
	});
</script>

<svelte:head><title>{CURRENT_USER.name} · Meshwork</title></svelte:head>

<main class="box-border flex flex-col gap-7 px-10 pt-8 pb-12">
	<section
		aria-label="Profile"
		class="border-line bg-surface flex flex-wrap items-center gap-x-8 gap-y-6 rounded-2xl border p-7"
	>
		<Avatar name={CURRENT_USER.name} size={88} />
		<div class="flex min-w-0 flex-[1_1_260px] flex-col gap-1">
			<h1 class="m-0 text-3xl font-semibold tracking-tight">{CURRENT_USER.name}</h1>
			<div class="text-muted flex items-center gap-2 text-[15px]">
				{CURRENT_USER.role} · {CURRENT_USER.email}
				<MockBadge note="Identity is demo data: there is no auth or user API yet" />
			</div>
			<div class="text-muted text-sm">
				In {graphs.length}
				{graphs.length === 1 ? 'graph' : 'graphs'} · member since {CURRENT_USER.memberSince}
			</div>
		</div>
		<div class="grid flex-[1_1_420px] grid-cols-[repeat(4,minmax(88px,1fr))] gap-3">
			{#each stats as s (s.label)}
				<div class="bg-canvas flex flex-col gap-0.5 rounded-xl px-3.5 py-3">
					<span class="text-[28px] font-semibold" style="color:{s.color}">{s.value}</span>
					<span class="text-ink-3 text-[13px]">{s.label}</span>
				</div>
			{/each}
		</div>
	</section>

	<div class="flex flex-wrap items-start gap-7">
		<div class="flex min-w-0 flex-[3_1_560px] flex-col gap-5">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<div
					role="tablist"
					aria-label="Issue lists"
					class="bg-surface-2 flex gap-0.5 rounded-[10px] p-[3px]"
				>
					<button
						type="button"
						role="tab"
						aria-selected={tab === 'assigned'}
						class="h-[34px] cursor-pointer rounded-lg border-0 px-4 text-sm {tab === 'assigned'
							? 'bg-surface text-ink font-semibold shadow-[0_1px_2px_rgba(18,22,28,.12)]'
							: 'text-ink-3 bg-transparent font-medium'}"
						onclick={() => (tab = 'assigned')}>Assigned · {assigned.length}</button
					>
					<button
						type="button"
						role="tab"
						aria-selected="false"
						disabled
						title="Needs a reviewer field in the API"
						class="text-faint h-[34px] cursor-not-allowed rounded-lg border-0 bg-transparent px-4 text-sm font-medium"
						>To review</button
					>
					<button
						type="button"
						role="tab"
						aria-selected="false"
						disabled
						title="Needs an author field in the API"
						class="text-faint h-[34px] cursor-not-allowed rounded-lg border-0 bg-transparent px-4 text-sm font-medium"
						>Created</button
					>
					<button
						type="button"
						role="tab"
						aria-selected={tab === 'done'}
						class="h-[34px] cursor-pointer rounded-lg border-0 px-4 text-sm {tab === 'done'
							? 'bg-surface text-ink font-semibold shadow-[0_1px_2px_rgba(18,22,28,.12)]'
							: 'text-ink-3 bg-transparent font-medium'}"
						onclick={() => (tab = 'done')}>Done · {done.length}</button
					>
				</div>
				<select
					class="border-line-strong bg-surface h-9 rounded-lg border py-0 pr-8 pl-3.5 text-sm font-medium"
					aria-label="Graph"
					bind:value={graphFilter}
				>
					<option value="">All graphs</option>
					{#each graphs as [id, name] (id)}<option value={id}>{name}</option>{/each}
				</select>
			</div>

			{#if !loading && list.length === 0}
				<div class="card text-muted p-6 text-sm">
					Nothing {tab === 'assigned' ? 'assigned to you' : 'done yet'} in the graphs opened in this
					browser. Tasks are matched to you by the name {CURRENT_USER.name}.
				</div>
			{/if}

			{#each groups.filter((g) => g.rows.length > 0) as g (g.status)}
				<section class="border-line bg-surface overflow-hidden rounded-[14px] border">
					<h2
						class="border-line-soft bg-surface-3 m-0 flex items-center gap-2 border-b px-5 py-3 text-sm font-semibold"
					>
						<span class="size-2.5 rounded-full" style="background:{STATUS_META[g.status].dot}"
						></span>
						{g.title}<span class="text-muted font-normal">{g.rows.length}</span>
					</h2>
					<ul class="m-0 list-none p-0">
						{#each g.rows as r (r.task.id)}
							<li
								class="border-line-soft flex flex-wrap items-center gap-x-3.5 gap-y-1.5 border-t px-5 py-3.5 first:border-t-0"
							>
								<span class="text-muted w-14 flex-none font-mono text-[13px]"
									>{shortId(r.task)}</span
								>
								<div class="flex min-w-0 flex-[1_1_240px] flex-col gap-0.5">
									<a
										href="/graph/{r.graphId}/task/{r.task.id}"
										class="text-ink text-[15px] font-medium no-underline">{r.task.name}</a
									>
									{#if r.note}
										<span
											class="text-[13px]"
											style="color:{r.warn ? 'var(--color-danger)' : 'var(--color-muted)'}"
											>{r.note}</span
										>
									{/if}
								</div>
								<span class="bg-canvas text-ink-3 rounded-md px-2.5 py-[3px] text-[13px]"
									>{r.graph}</span
								>
								<span class="min-w-[84px] text-center"><StatusPill status={r.task.status} /></span>
							</li>
						{/each}
					</ul>
				</section>
			{/each}
		</div>

		<aside class="flex min-w-0 flex-[1_1_300px] flex-col gap-5">
			<section aria-labelledby="wl" class="card flex flex-col gap-3.5 p-5">
				<h2 id="wl" class="m-0 text-[15px] font-semibold">Workload by graph</h2>
				{#each workload as l (l.name)}
					<div class="flex flex-col gap-1.5">
						<div class="flex justify-between text-sm">
							<span>{l.name}</span><span class="text-muted">{l.n} open</span>
						</div>
						<div class="bg-surface-2 h-2 overflow-hidden rounded">
							<div class="h-full" style="width:{l.pct}%;background:{l.color}"></div>
						</div>
					</div>
				{:else}
					<span class="text-sm text-muted">No open tasks.</span>
				{/each}
			</section>

			<section aria-labelledby="ra" class="card flex flex-col gap-3.5 p-5">
				<h2 id="ra" class="m-0 flex items-center gap-2 text-[15px] font-semibold">
					Recent activity <MockBadge note="Demo data: the backend keeps no change log yet" />
				</h2>
				<ul class="m-0 flex list-none flex-col gap-3.5 p-0">
					{#each MOCK_RECENT_ACTIVITY as a (a.what)}
						<li class="flex flex-col gap-0.5">
							<span class="text-sm leading-snug">{a.what}</span>
							<span class="text-muted text-[13px]">{a.when}</span>
						</li>
					{/each}
				</ul>
			</section>
		</aside>
	</div>
</main>
