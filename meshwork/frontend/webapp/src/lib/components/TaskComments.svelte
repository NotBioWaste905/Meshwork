<script lang="ts">
	import { api, type Comment } from '$lib/api';
	import { graphStore } from '$lib/graphStore.svelte';
	import { liveRefresh } from '$lib/liveRefresh';
	import { session } from '$lib/session.svelte';
	import { ago, formatDateTime } from '$lib/time';
	import ActorAvatar from './ActorAvatar.svelte';
	import Markdown from './Markdown.svelte';

	/** The conversation on a task: notes and hand-offs by people and agents. */
	let { graphId, taskId }: { graphId: string; taskId: string } = $props();

	let comments = $state<Comment[]>([]);
	let draft = $state('');
	let saving = $state(false);
	let error = $state('');

	async function load(graph: string, task: string) {
		try {
			const loaded = await api.listComments(graph, task);
			// a slow answer for the previous task must not land on this one
			if (graph === graphId && task === taskId) comments = loaded;
		} catch (e) {
			console.warn('Cannot load comments:', e);
		}
	}

	// Reload on task change, and poll: agents comment too.
	$effect(() => {
		const graph = graphId;
		const task = taskId;
		comments = [];
		draft = '';
		error = '';
		load(graph, task);
		return liveRefresh(() => load(graph, task));
	});

	// Writing a comment: hold the graph's background refresh, like the other edit forms.
	$effect(() => {
		if (draft.trim()) return graphStore.hold();
	});

	async function send(event: SubmitEvent) {
		event.preventDefault();
		if (saving || !draft.trim()) return;
		saving = true;
		try {
			const added = await api.addComment(graphId, taskId, draft);
			comments = [...comments, added];
			draft = '';
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		saving = false;
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
			(event.currentTarget as HTMLTextAreaElement).form?.requestSubmit();
		}
	}
</script>

<section aria-labelledby="comments" class="card flex flex-col gap-4 px-6 py-5">
	<h2 id="comments" class="m-0 text-[17px] font-semibold">
		Comments{#if comments.length > 0}<span class="text-muted ml-2 font-normal"
				>{comments.length}</span
			>{/if}
	</h2>

	{#if comments.length > 0}
		<ul class="m-0 flex list-none flex-col gap-4 p-0">
			{#each comments as c (c.id)}
				<li class="flex gap-3">
					{#if c.author}<ActorAvatar id={c.author} size={28} />{:else}<span
							class="bg-surface-2 size-7 flex-none rounded-full"
						></span>{/if}
					<div class="flex min-w-0 flex-1 flex-col gap-1">
						<div class="flex flex-wrap items-baseline gap-2 text-sm">
							<strong class="font-semibold"
								>{c.author ? graphStore.actorName(c.author) : 'Deleted actor'}</strong
							>
							<time
								class="text-muted text-[13px]"
								datetime={new Date(c.created_at).toISOString()}
								title={formatDateTime(c.created_at)}>{ago(c.created_at)}</time
							>
						</div>
						<Markdown source={c.body} class="max-w-[68ch]" />
					</div>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="text-muted m-0 text-sm">No comments yet.</p>
	{/if}

	{#if session.me}
		<form class="flex flex-col gap-2" onsubmit={send}>
			<label for="new-comment" class="sr-only">New comment</label>
			<textarea
				id="new-comment"
				rows="3"
				class="field"
				placeholder="Leave a note, a question or a hand-off (Markdown; ⌘/Ctrl+Enter to send)"
				bind:value={draft}
				onkeydown={onKeydown}
			></textarea>
			{#if error}
				<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm" role="alert">{error}</div>
			{/if}
			<div class="flex items-center gap-3">
				<button type="submit" class="btn btn-primary btn-sm" disabled={saving || !draft.trim()}
					>{saving ? 'Sending…' : 'Comment'}</button
				>
				<span class="text-muted text-[13px]">as {session.me.name}</span>
			</div>
		</form>
	{:else if session.ready}
		<p class="text-muted m-0 text-sm"><a href="/me">Sign in</a> to comment.</p>
	{/if}
</section>
