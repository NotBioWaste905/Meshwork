<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Status, type Task } from '$lib/api';
	import ActorAvatar from '$lib/components/ActorAvatar.svelte';
	import AssigneePicker from '$lib/components/AssigneePicker.svelte';
	import Markdown from '$lib/components/Markdown.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import { graphStore } from '$lib/graphStore.svelte';
	import { shortId, STATUS_META, STATUS_ORDER } from '$lib/status';

	let taskId = $derived(page.params.taskId ?? '');
	let task = $derived(graphStore.byId.get(taskId));
	let base = $derived(`/graph/${graphStore.graphId}`);

	let blockers = $derived(
		(task?.depends_on ?? [])
			.map((id) => graphStore.byId.get(id))
			.filter((t): t is Task => t !== undefined)
	);
	let blocked = $derived(graphStore.dependents.get(taskId) ?? []);
	let openBlockers = $derived(task ? graphStore.openBlockers(task) : []);
	let isChore = $derived(task?.tags.includes('chore') ?? false);

	// Tasks that may become a blocker: not itself, not already one, not downstream (would be a cycle).
	let downstream = $derived.by(() => {
		const seen = new Set<string>();
		const queue = [taskId];
		while (queue.length > 0) {
			for (const t of graphStore.dependents.get(queue.pop()!) ?? []) {
				if (!seen.has(t.id)) {
					seen.add(t.id);
					queue.push(t.id);
				}
			}
		}
		return seen;
	});
	let candidates = $derived(
		graphStore.tasks.filter(
			(t) => t.id !== taskId && !task?.depends_on.includes(t.id) && !downstream.has(t.id)
		)
	);

	let editing = $state(false);
	let editName = $state('');
	let editDescription = $state('');
	let editTags = $state('');
	let saving = $state(false);
	let addingBlocker = $state(false);

	// An open edit form or blocker picker: hold background refresh until it closes.
	$effect(() => {
		if (editing || addingBlocker) return graphStore.hold();
	});

	// SvelteKit reuses this component when navigating task -> task: drop per-task UI state.
	$effect(() => {
		void taskId;
		editing = false;
		addingBlocker = false;
	});

	function startEdit() {
		if (!task) return;
		editName = task.name;
		editDescription = task.description;
		editTags = task.tags.join(', ');
		editing = true;
	}

	async function saveEdit(event: SubmitEvent) {
		event.preventDefault();
		if (!task || saving || !editName.trim()) return;
		saving = true;
		const tags = [
			...new Set(
				editTags
					.split(',')
					.map((t) => t.trim())
					.filter(Boolean)
			)
		];
		const saved = await graphStore.updateTask(task.id, {
			name: editName.trim(),
			description: editDescription,
			tags
		});
		saving = false;
		if (saved) editing = false;
	}

	function setStatus(status: Status) {
		if (task && status !== task.status) graphStore.updateTask(task.id, { status });
	}

	function setAssignee(actorId: string) {
		if (!task || actorId === (task.users[0] ?? '')) return;
		graphStore.updateTask(task.id, { users: actorId ? [actorId] : [] });
	}

	async function addBlocker(event: Event) {
		const select = event.currentTarget as HTMLSelectElement;
		const dependency = select.value;
		select.value = '';
		if (!dependency) return;
		await graphStore.connect(dependency, taskId);
		addingBlocker = false;
	}

	async function removeTask() {
		if (!task || !confirm(`Delete "${task.name}"? This cannot be undone.`)) return;
		if (await graphStore.deleteTask(task.id)) await goto(base);
	}

	// Neighbourhood sketch: up to 3 blockers on the left, up to 3 dependents on the right.
	const ROW = 50;
	let left = $derived(blockers.slice(0, 3));
	let right = $derived(blocked.slice(0, 3));
	let rows = $derived(Math.max(left.length, right.length, 1));
	let height = $derived(rows * ROW + 20);
	const rowY = (i: number, n: number) => (height - n * ROW) / 2 + i * ROW + ROW / 2;
</script>

<svelte:head
	><title>{task ? `${shortId(task)} ${task.name}` : 'Task'} · Meshwork</title></svelte:head
>

<div class="flex min-h-screen flex-col">
	<TopBar>
		{#snippet left()}
			<a href="/graphs" class="text-muted no-underline">Graphs</a><span class="text-faint">/</span>
			<a href={base} class="text-muted no-underline">{graphStore.name}</a><span class="text-faint"
				>/</span
			>
			<strong class="font-mono text-sm font-semibold">{task ? shortId(task) : '…'}</strong>
		{/snippet}
		{#snippet right()}
			<a href={base} class="btn btn-secondary btn-sm">Show in graph</a>
			<button
				type="button"
				class="btn btn-secondary btn-sm text-danger"
				disabled={!task}
				onclick={removeTask}>Delete</button
			>
		{/snippet}
	</TopBar>

	{#if !graphStore.loaded}
		<p class="text-muted p-8">Loading…</p>
	{:else if !task}
		<div class="p-8">
			<div class="card flex max-w-md flex-col items-start gap-2 p-6">
				<strong class="font-semibold">Task not found</strong>
				<span class="text-muted text-sm">It may have been deleted.</span>
				<a href={base} class="btn btn-primary btn-sm mt-2">Back to graph</a>
			</div>
		</div>
	{:else}
		<div class="box-border flex flex-wrap items-start gap-8 p-8">
			<main class="flex min-w-0 flex-[3_1_560px] flex-col gap-7">
				{#if graphStore.error}
					<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm" role="alert">
						{graphStore.error}
					</div>
				{/if}

				{#if editing}
					<form class="card flex flex-col gap-4 p-5" onsubmit={saveEdit}>
						<div class="flex flex-col gap-1.5">
							<label for="e-name" class="text-sm font-medium">Name</label>
							<input id="e-name" class="field" required bind:value={editName} />
						</div>
						<div class="flex flex-col gap-1.5">
							<label for="e-desc" class="text-sm font-medium"
								>Description <span class="text-muted font-normal">(Markdown)</span></label
							>
							<textarea id="e-desc" rows="5" class="field" bind:value={editDescription}></textarea>
						</div>
						<div class="flex flex-col gap-1.5">
							<label for="e-tags" class="text-sm font-medium">Tags (comma separated)</label>
							<input id="e-tags" class="field" bind:value={editTags} />
						</div>
						<div class="flex gap-3">
							<button type="submit" class="btn btn-primary btn-sm" disabled={saving}>
								{saving ? 'Saving…' : 'Save'}
							</button>
							<button
								type="button"
								class="btn btn-secondary btn-sm"
								onclick={() => (editing = false)}>Cancel</button
							>
						</div>
					</form>
				{:else}
					<div class="flex flex-col gap-3.5">
						<div class="flex items-center gap-3">
							<StatusPill status={task.status} size="lg" dot />
							<button
								type="button"
								class="text-accent text-sm font-medium hover:underline"
								onclick={startEdit}>Edit</button
							>
						</div>
						<h1 class="m-0 text-[32px] leading-tight font-semibold tracking-tight">{task.name}</h1>
						{#if task.description}
							<Markdown source={task.description} class="max-w-[68ch]" />
						{/if}
						{#if task.tags.length > 0}
							<div class="flex flex-wrap gap-2">
								{#each task.tags as tag (tag)}
									<span
										class="bg-surface-2 text-ink-3 inline-flex h-[26px] items-center rounded-md px-2.5 text-[13px]"
										>{tag}</span
									>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<section aria-labelledby="deps" class="card flex flex-col gap-2 px-6 py-5">
					<div class="flex flex-wrap items-baseline justify-between gap-3">
						<h2 id="deps" class="m-0 text-[17px] font-semibold">Blocked by</h2>
						<span class="text-muted text-[13px]">All must be done before this can start</span>
					</div>
					<ul class="m-0 list-none p-0">
						{#each blockers as b (b.id)}
							{@render row(b, true)}
						{:else}
							<li class="border-t border-line-soft py-3 text-sm text-muted">
								Nothing blocks this task.
							</li>
						{/each}
					</ul>
					{#if addingBlocker}
						<select
							class="field mt-1 h-9 max-w-sm text-sm"
							aria-label="Add blocker"
							onchange={addBlocker}
						>
							<option value="">Choose a task…</option>
							{#each candidates as c (c.id)}
								<option value={c.id}>{shortId(c)} {c.name}</option>
							{/each}
						</select>
					{:else}
						<button
							type="button"
							class="border-faint bg-surface text-accent mt-1 h-9 cursor-pointer self-start rounded-lg border border-dashed px-3.5 text-sm font-medium"
							onclick={() => (addingBlocker = true)}>+ Add blocker</button
						>
					{/if}
				</section>

				<section aria-labelledby="blocks" class="card flex flex-col gap-2 px-6 py-5">
					<h2 id="blocks" class="m-0 text-[17px] font-semibold">Blocks</h2>
					<ul class="m-0 list-none p-0">
						{#each blocked as b (b.id)}
							{@render row(b, false)}
						{:else}
							<li class="border-t border-line-soft py-3 text-sm text-muted">
								Nothing depends on this task.
							</li>
						{/each}
					</ul>
				</section>

				<section aria-labelledby="ver" class="card flex flex-col gap-3 px-6 py-5">
					<h2 id="ver" class="m-0 text-[17px] font-semibold">Verification</h2>
					<p class="text-ink-3 m-0 text-[15px] leading-normal">
						{task.status === Status.REVIEW
							? 'The result is waiting for a check. Approve it to mark the task done, or send it back to work.'
							: 'When the work is finished, send the task to review so someone checks the result before it is done.'}
					</p>
					<div class="flex flex-wrap gap-3">
						{#if task.status === Status.REVIEW}
							<button type="button" class="btn btn-primary" onclick={() => setStatus(Status.DONE)}
								>Approve</button
							>
							<button
								type="button"
								class="btn btn-secondary"
								onclick={() => setStatus(Status.IN_PROGRESS)}>Send back</button
							>
						{:else}
							<button
								type="button"
								class="btn btn-primary"
								disabled={openBlockers.length > 0 || task.status === Status.DONE}
								onclick={() => setStatus(Status.REVIEW)}>Send to review</button
							>
							{#if openBlockers.length > 0}
								<span class="text-danger self-center text-sm"
									>Available once {openBlockers.map((t) => shortId(t)).join(' and ')}
									{openBlockers.length === 1 ? 'is' : 'are'} done</span
								>
							{/if}
						{/if}
					</div>
				</section>
			</main>

			<aside class="flex max-w-full min-w-0 flex-[1_1_300px] flex-col gap-5">
				<section aria-labelledby="det" class="card flex flex-col gap-4 p-5">
					<h2 id="det" class="m-0 text-[15px] font-semibold">Details</h2>
					<dl
						class="m-0 grid grid-cols-[100px_minmax(0,1fr)] items-center gap-x-3 gap-y-3.5 text-sm"
					>
						<dt class="text-muted">Status</dt>
						<dd class="m-0">
							<select
								class="border-line-strong bg-surface h-8 rounded-lg border py-0 pr-8 pl-3 text-sm font-medium"
								aria-label="Status"
								value={task.status}
								onchange={(e) => setStatus(e.currentTarget.value as Status)}
							>
								{#each STATUS_ORDER as s (s)}
									<option value={s}>{STATUS_META[s].label}</option>
								{/each}
							</select>
						</dd>
						<dt class="text-muted">Assignee</dt>
						<dd class="m-0 flex items-center gap-2">
							{#if task.users[0]}<ActorAvatar id={task.users[0]} size={24} />{/if}
							<AssigneePicker small value={task.users[0] ?? ''} onchange={setAssignee} />
						</dd>
						<dt class="text-muted">Graph</dt>
						<dd class="m-0"><a href={base}>{graphStore.name}</a></dd>
						<dt class="text-muted">Type</dt>
						<dd class="m-0">{isChore ? 'Chore' : 'Task'}</dd>
					</dl>
				</section>

				<section aria-labelledby="nb" class="card flex flex-col gap-3 p-5">
					<h2 id="nb" class="m-0 text-[15px] font-semibold">Neighborhood</h2>
					<svg
						viewBox="0 0 280 {height}"
						class="h-auto w-full"
						aria-label="Blockers lead to this task, which leads to the tasks it blocks"
						fill="none"
					>
						<defs>
							<marker
								id="nm"
								viewBox="0 0 10 10"
								refX="9"
								refY="5"
								markerWidth="6"
								markerHeight="6"
								orient="auto"><path d="M0 0L10 5L0 10z" fill="#6B7684" /></marker
							>
						</defs>
						<g stroke="#6B7684" stroke-width="1.75" marker-end="url(#nm)">
							{#each left as t, i (t.id)}
								<path
									d="M82 {rowY(i, left.length)} C110 {rowY(i, left.length)} 110 {height /
										2} 128 {height / 2}"
								/>
							{/each}
							{#each right as t, i (t.id)}
								<path
									d="M198 {height / 2} C206 {height / 2} 206 {rowY(i, right.length)} 214 {rowY(
										i,
										right.length
									)}"
								/>
							{/each}
						</g>
						<g font-family="IBM Plex Sans, sans-serif" font-size="12" font-weight="500">
							{#each left as t, i (t.id)}
								{@render neighbor(t, 6, rowY(i, left.length), 76, 'Blocker')}
							{/each}
							<rect
								x="128"
								y={height / 2 - 18}
								width="70"
								height="36"
								rx="8"
								fill="#fff"
								stroke="#2349C7"
								stroke-width="2"
							/>
							<circle cx="140" cy={height / 2} r="4" fill={STATUS_META[task.status].dot} />
							<text x="149" y={height / 2 + 4} fill="#12161C">{shortId(task)}</text>
							{#each right as t, i (t.id)}
								{@render neighbor(t, 214, rowY(i, right.length), 60, 'Blocks')}
							{/each}
						</g>
					</svg>
				</section>
			</aside>
		</div>
	{/if}
</div>

{#snippet row(t: Task, removable: boolean)}
	<li class="border-line-soft flex flex-wrap items-center gap-x-3.5 gap-y-2 border-t py-3">
		<span class="text-muted w-14 flex-none font-mono text-[13px]">{shortId(t)}</span>
		<a
			href="{base}/task/{t.id}"
			class="text-ink min-w-0 flex-[1_1_200px] text-[15px] font-medium no-underline">{t.name}</a
		>
		<StatusPill status={t.status} />
		{#if t.users[0]}<ActorAvatar id={t.users[0]} size={26} />{/if}
		{#if removable}
			<button
				type="button"
				class="text-muted hover:text-danger px-1 text-lg"
				aria-label="Remove blocker {t.name}"
				onclick={() => graphStore.disconnect(t.id, taskId)}>×</button
			>
		{/if}
	</li>
{/snippet}

{#snippet neighbor(t: Task, x: number, y: number, width: number, relation: string)}
	<a
		href="{base}/task/{t.id}"
		class="group cursor-pointer outline-none"
		aria-label="{relation}: {shortId(t)} {t.name}"
	>
		<title>{shortId(t)} {t.name}</title>
		<rect
			{x}
			y={y - 18}
			{width}
			height="36"
			rx="8"
			fill="#fff"
			stroke="#C9D0D8"
			class="group-hover:fill-canvas group-hover:stroke-accent group-focus-visible:stroke-accent transition-colors group-focus-visible:stroke-2"
		/>
		<circle cx={x + 12} cy={y} r="4" fill={STATUS_META[t.status].dot} />
		<text x={x + 21} y={y + 4} fill="#12161C" class="group-hover:fill-accent">{shortId(t)}</text>
	</a>
{/snippet}
