<script lang="ts">
	import ActorAvatar from '$lib/components/ActorAvatar.svelte';
	import GraphHeader from '$lib/components/GraphHeader.svelte';
	import PriorityMark from '$lib/components/PriorityMark.svelte';
	import { graphStore } from '$lib/graphStore.svelte';
	import { session } from '$lib/session.svelte';
	import { shortId, STATUS_META, STATUS_ORDER } from '$lib/status';

	let columns = $derived(
		STATUS_ORDER.map((status) => ({
			status,
			cards: graphStore.visible.filter((t) => t.status === status)
		}))
	);
	let terminalIds = $derived(new Set(graphStore.terminals.map((t) => t.id)));
</script>

<svelte:head><title>Board · {graphStore.name} · Meshwork</title></svelte:head>

<div class="flex h-screen flex-col overflow-hidden">
	<GraphHeader view="board" />

	<div
		class="border-line flex h-[52px] flex-none items-center gap-2 border-b px-6"
		role="toolbar"
		aria-label="Filters"
	>
		<button
			type="button"
			class="chip"
			aria-pressed={graphStore.filter === 'all'}
			onclick={() => (graphStore.filter = 'all')}>All tasks · {graphStore.tasks.length}</button
		>
		{#if session.me}
			<button
				type="button"
				class="chip"
				aria-pressed={graphStore.filter === 'mine'}
				onclick={() => (graphStore.filter = 'mine')}>Mine · {graphStore.mineCount}</button
			>
		{/if}
		<button
			type="button"
			class="chip"
			aria-pressed={graphStore.filter === 'ready'}
			style={graphStore.filter === 'ready' ? '' : 'color:var(--color-st-done-fg)'}
			onclick={() => (graphStore.filter = 'ready')}>Ready · {graphStore.ready.size}</button
		>
		<button
			type="button"
			class="chip"
			aria-pressed={graphStore.filter === 'blocked'}
			onclick={() => (graphStore.filter = 'blocked')}>Blocked · {graphStore.blockedCount}</button
		>
		<span class="chip cursor-default" title="Only grouping by status is supported">
			Group by: Status
		</span>
		{#if graphStore.error}
			<span class="text-danger ml-auto text-[13px]">{graphStore.error}</span>
		{/if}
	</div>

	<div class="box-border flex min-h-0 flex-1 gap-4 overflow-x-auto px-6 py-5">
		{#each columns as col (col.status)}
			<section
				class="bg-column box-border flex min-w-52 flex-1 flex-col gap-3 overflow-y-auto rounded-[14px] p-3"
			>
				<h2
					class="text-ink-2 m-0 flex items-center gap-2 px-1 py-0.5 text-[13px] font-semibold tracking-wider"
				>
					<span class="size-2.5 rounded-full" style="background:{STATUS_META[col.status].dot}"
					></span>
					{STATUS_META[col.status].pill}
					<span class="text-muted font-normal">{col.cards.length}</span>
				</h2>
				{#each col.cards as task (task.id)}
					{@const note = graphStore.note(task)}
					{@const dark = terminalIds.has(task.id)}
					<a
						href="/graph/{graphStore.graphId}/task/{task.id}"
						class="flex flex-col gap-2 rounded-[10px] p-3 no-underline shadow-[0_1px_2px_rgba(18,22,28,.06)] {dark
							? 'bg-ink text-white'
							: 'bg-surface text-ink'} {task.status === 'BLOCKED'
							? 'border-danger-dot border-[1.5px]'
							: graphStore.ready.has(task.id)
								? 'border-st-done-dot border-2'
								: 'border-line border'}"
					>
						<div class="flex items-center justify-between">
							<span class="flex items-center gap-2">
								<span class="font-mono text-xs {dark ? 'text-faint' : 'text-muted'}"
									>{shortId(task)}</span
								>
								<PriorityMark priority={task.priority} {dark} />
							</span>
							{#if task.users[0]}<ActorAvatar id={task.users[0]} size={24} />{/if}
						</div>
						<div class="text-sm leading-snug font-medium">{task.name}</div>
						{#if note.text}
							<div
								class="text-xs font-medium"
								style="color:{note.warn
									? 'var(--color-danger)'
									: dark
										? '#AEB7C2'
										: 'var(--color-muted)'}"
							>
								{note.text}
							</div>
						{/if}
					</a>
				{/each}
			</section>
		{/each}
	</div>
</div>
