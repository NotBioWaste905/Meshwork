<script lang="ts">
	import { Status } from '$lib/api';
	import ActorAvatar from '$lib/components/ActorAvatar.svelte';
	import GraphHeader from '$lib/components/GraphHeader.svelte';
	import PriorityMark from '$lib/components/PriorityMark.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { graphStore } from '$lib/graphStore.svelte';
	import { PRIORITY_ORDER, prLabel } from '$lib/priority';
	import { shortId, STATUS_META, STATUS_ORDER } from '$lib/status';

	type SortKey = 'number' | 'status' | 'priority' | 'name' | 'assignee';

	let search = $state('');
	let statusFilter = $state<'' | Status>('');
	let assigneeFilter = $state('');
	let tagFilter = $state('');
	let sortKey = $state<SortKey>('status');

	let assignees = $derived(graphStore.users);
	/** Sort key of a task's first assignee; unassigned tasks go last. */
	const assigneeKey = (t: { users: string[] }) =>
		t.users[0] ? graphStore.actorName(t.users[0]) : '~';
	let tags = $derived([...new Set(graphStore.tasks.flatMap((t) => t.tags))].sort());

	let rows = $derived(
		graphStore.tasks
			.filter((t) => {
				const q = search.trim().toLowerCase();
				return t.name.toLowerCase().includes(q) || shortId(t).toLowerCase().includes(q);
			})
			.filter((t) => !statusFilter || t.status === statusFilter)
			.filter((t) => !assigneeFilter || t.users.includes(assigneeFilter))
			.filter((t) => !tagFilter || t.tags.includes(tagFilter))
			.toSorted((a, b) => {
				if (sortKey === 'number') return (a.number ?? Infinity) - (b.number ?? Infinity);
				if (sortKey === 'name') return a.name.localeCompare(b.name);
				if (sortKey === 'assignee') return assigneeKey(a).localeCompare(assigneeKey(b));
				if (sortKey === 'priority')
					return PRIORITY_ORDER.indexOf(a.priority) - PRIORITY_ORDER.indexOf(b.priority);
				return STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status);
			})
	);

	const sortLabel: Record<SortKey, string> = {
		number: 'ID',
		status: 'status',
		priority: 'priority',
		name: 'name',
		assignee: 'assignee'
	};

	const select =
		'h-8 cursor-pointer rounded-full border border-line-strong bg-surface py-0 pr-8 pl-3.5 text-[13px] font-medium text-ink';
</script>

<svelte:head><title>Table · {graphStore.name} · Meshwork</title></svelte:head>

<div class="flex h-screen flex-col overflow-hidden">
	<GraphHeader view="table" />

	<div
		class="border-line flex h-[52px] flex-none items-center gap-2 border-b px-6"
		role="toolbar"
		aria-label="Filters"
	>
		<label class="block">
			<span class="sr-only">Filter tasks</span>
			<input
				type="search"
				class="border-line-strong bg-surface h-[34px] w-60 rounded-lg border px-3 text-sm"
				placeholder="Filter tasks"
				bind:value={search}
			/>
		</label>
		<select class={select} aria-label="Status" bind:value={statusFilter}>
			<option value="">Status</option>
			{#each STATUS_ORDER as s (s)}<option value={s}>{STATUS_META[s].label}</option>{/each}
		</select>
		<select class={select} aria-label="Assignee" bind:value={assigneeFilter}>
			<option value="">Assignee</option>
			{#each assignees as a (a)}<option value={a}>{graphStore.actorName(a)}</option>{/each}
		</select>
		<select class={select} aria-label="Tag" bind:value={tagFilter}>
			<option value="">Tag</option>
			{#each tags as t (t)}<option value={t}>{t}</option>{/each}
		</select>
		<div class="flex-1"></div>
		{#if graphStore.error}
			<span class="text-danger text-[13px]">{graphStore.error}</span>
		{:else}
			<span class="text-muted text-[13px]"
				>{rows.length} tasks · sorted by {sortLabel[sortKey]}</span
			>
		{/if}
	</div>

	<div class="box-border min-h-0 flex-1 overflow-auto px-6 py-5">
		<div class="border-line bg-surface overflow-hidden rounded-[14px] border">
			<table class="w-full border-collapse">
				<thead>
					<tr>
						<th class="w-[90px]" aria-sort={sortKey === 'number' ? 'ascending' : 'none'}>
							<button type="button" class="sort" onclick={() => (sortKey = 'number')}>ID</button>
						</th>
						<th aria-sort={sortKey === 'name' ? 'ascending' : 'none'}>
							<button type="button" class="sort" onclick={() => (sortKey = 'name')}>TASK</button>
						</th>
						<th class="w-[140px]" aria-sort={sortKey === 'status' ? 'ascending' : 'none'}>
							<button type="button" class="sort" onclick={() => (sortKey = 'status')}>STATUS</button
							>
						</th>
						<th class="w-[100px]" aria-sort={sortKey === 'priority' ? 'ascending' : 'none'}>
							<button type="button" class="sort" onclick={() => (sortKey = 'priority')}
								>PRIORITY</button
							>
						</th>
						<th class="w-[200px]" aria-sort={sortKey === 'assignee' ? 'ascending' : 'none'}>
							<button type="button" class="sort" onclick={() => (sortKey = 'assignee')}
								>ASSIGNEE</button
							>
						</th>
						<th class="w-[180px]">BLOCKED BY</th>
						<th class="w-[200px]">TAGS</th>
						<th class="w-[160px]">PR</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as task (task.id)}
						{@const open = graphStore.openBlockers(task)}
						<tr>
							<td class="text-muted font-mono text-[13px] whitespace-nowrap">{shortId(task)}</td>
							<td>
								<a
									href="/graph/{graphStore.graphId}/task/{task.id}"
									class="text-ink font-medium no-underline">{task.name}</a
								>
								{#if graphStore.terminals.some((t) => t.id === task.id)}
									<span class="text-muted ml-2 text-xs">terminal</span>
								{:else if task.tags.includes('chore')}
									<span class="text-muted ml-2 text-xs">chore</span>
								{/if}
							</td>
							<td><StatusPill status={task.status} /></td>
							<td><PriorityMark priority={task.priority} /></td>
							<td>
								{#if task.users.length > 0}
									<span class="inline-flex items-center gap-2">
										<ActorAvatar id={task.users[0]} size={24} />{graphStore.actorName(
											task.users[0]
										)}
									</span>
								{:else}
									<span class="text-muted">Unassigned</span>
								{/if}
							</td>
							<td
								class="font-mono text-[13px]"
								style="color:{task.status === Status.BLOCKED
									? 'var(--color-danger)'
									: 'var(--color-ink-3)'}"
							>
								{open.length > 0 ? open.map((t) => shortId(t)).join(', ') : '—'}
							</td>
							<td>
								<span class="flex flex-wrap gap-1">
									{#each task.tags as tag (tag)}
										<span class="bg-surface-2 text-ink-3 rounded-md px-2 py-0.5 text-xs">{tag}</span
										>
									{/each}
								</span>
							</td>
							<td class="max-w-[160px] truncate text-[13px]">
								{#if task.pr_url}
									<a
										href={task.pr_url}
										target="_blank"
										rel="noopener noreferrer"
										title={task.pr_url}>{prLabel(task.pr_url)}</a
									>
								{:else}
									<span class="text-muted">—</span>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>
</div>

<style>
	th {
		text-align: left;
		font-size: 12px;
		font-weight: 600;
		letter-spacing: 0.04em;
		color: var(--color-ink-3);
		padding: 0 14px;
		height: 40px;
		background: var(--color-surface-3);
		border-bottom: 1px solid var(--color-line);
		white-space: nowrap;
	}
	td {
		padding: 0 14px;
		height: 52px;
		border-bottom: 1px solid var(--color-line-soft);
		font-size: 14px;
	}
	.sort {
		font: inherit;
		letter-spacing: inherit;
		color: inherit;
		background: none;
		border: 0;
		padding: 0;
		cursor: pointer;
	}
</style>
