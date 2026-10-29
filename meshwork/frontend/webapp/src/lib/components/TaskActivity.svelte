<script lang="ts">
	import { untrack } from 'svelte';
	import { api, type TaskEvent } from '$lib/api';
	import { describe, taskLabel } from '$lib/activity';
	import { graphStore } from '$lib/graphStore.svelte';
	import { liveRefresh } from '$lib/liveRefresh';
	import { ago, formatDateTime } from '$lib/time';
	import ActorAvatar from './ActorAvatar.svelte';

	/** The task's change log, newest first: who changed what, and when. */
	let { graphId, taskId }: { graphId: string; taskId: string } = $props();

	const SHOWN = 8;

	let events = $state<TaskEvent[]>([]);
	let showAll = $state(false);

	const names = {
		actor: (id: string) => graphStore.actorName(id),
		task: (id: string) => taskLabel(id, graphStore.byId.get(id)?.name)
	};

	async function load(graph: string, task: string) {
		try {
			const loaded = await api.listEvents(graph, task);
			if (graph === graphId && task === taskId) events = loaded.reverse();
		} catch (e) {
			console.warn('Cannot load activity:', e);
		}
	}

	$effect(() => {
		const graph = graphId;
		const task = taskId;
		events = [];
		showAll = false;
		return liveRefresh(() => load(graph, task));
	});

	// First load, and again whenever the task changes (an edit here, or a refresh): its log has
	// a new entry.
	$effect(() => {
		void graphStore.byId.get(taskId)?.updated_at;
		untrack(() => load(graphId, taskId));
	});

	let visible = $derived(showAll ? events : events.slice(0, SHOWN));
</script>

<section aria-labelledby="activity" class="card flex flex-col gap-3 px-6 py-5">
	<h2 id="activity" class="m-0 text-[17px] font-semibold">Activity</h2>
	{#if events.length === 0}
		<p class="text-muted m-0 text-sm">No changes recorded yet.</p>
	{:else}
		<ol class="m-0 flex list-none flex-col gap-2.5 p-0">
			{#each visible as e (e.id)}
				<li class="flex items-start gap-2.5 text-sm">
					{#if e.actor}<ActorAvatar id={e.actor} size={20} />{:else}<span
							class="bg-surface-2 size-5 flex-none rounded-full"
						></span>{/if}
					<span class="min-w-0 flex-1 leading-5">
						<strong class="font-semibold"
							>{e.actor ? graphStore.actorName(e.actor) : 'Someone'}</strong
						>
						{describe(e, names)}
						<time
							class="text-muted ml-1 text-[13px] whitespace-nowrap"
							datetime={new Date(e.at).toISOString()}
							title={formatDateTime(e.at)}>· {ago(e.at)}</time
						>
					</span>
				</li>
			{/each}
		</ol>
		{#if events.length > SHOWN}
			<button
				type="button"
				class="text-accent self-start text-sm font-medium hover:underline"
				onclick={() => (showAll = !showAll)}
				>{showAll ? 'Show less' : `Show all ${events.length}`}</button
			>
		{/if}
	{/if}
</section>
