<script lang="ts">
	import { tick } from 'svelte';
	import { Priority, Status } from '$lib/api';
	import { graphStore } from '$lib/graphStore.svelte';
	import { PRIORITY_LABEL, PRIORITY_ORDER } from '$lib/priority';
	import { shortId, STATUS_META } from '$lib/status';
	import AssigneePicker from './AssigneePicker.svelte';

	let { onCreated }: { onCreated?: (id: string) => void } = $props();

	let name = $state('');
	let description = $state('');
	let kind = $state<'task' | 'chore'>('task');
	let status = $state<Status>(Status.TODO);
	let priority = $state<Priority>(Priority.NORMAL);
	/** Actor ID, '' for unassigned. */
	let assignee = $state('');
	/** Actor ID, '' for nobody. (The API would default to the token's actor; the UI sends none yet.) */
	let reporter = $state('');
	let blockers = $state<string[]>([]);
	let tags = $state<string[]>([]);
	let search = $state('');
	let tagDraft = $state('');
	let saving = $state(false);
	let nameInput = $state<HTMLInputElement>();

	let suggestions = $derived(
		graphStore.tasks
			.filter((t) => !blockers.includes(t.id))
			.filter((t) => {
				const q = search.trim().toLowerCase();
				return (
					q !== '' && (t.name.toLowerCase().includes(q) || shortId(t).toLowerCase().includes(q))
				);
			})
			.slice(0, 5)
	);

	$effect(() => {
		if (graphStore.newTaskOpen) {
			name = '';
			description = '';
			kind = 'task';
			status = Status.TODO;
			priority = Priority.NORMAL;
			assignee = '';
			reporter = '';
			blockers = [];
			tags = [];
			search = '';
			tagDraft = '';
			tick().then(() => nameInput?.focus());
		}
	});

	function close() {
		graphStore.newTaskOpen = false;
	}

	function addTag() {
		const tag = tagDraft.trim();
		if (tag && !tags.includes(tag)) tags = [...tags, tag];
		tagDraft = '';
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (saving || !name.trim()) return;
		addTag();
		saving = true;
		const allTags = kind === 'chore' && !tags.includes('chore') ? [...tags, 'chore'] : tags;
		const created = await graphStore.addTask({
			name: name.trim(),
			description,
			status,
			priority,
			tags: allTags,
			users: assignee ? [assignee] : [],
			reporter: reporter || undefined,
			depends_on: blockers
		});
		saving = false;
		if (created) {
			close();
			onCreated?.(created.id);
		}
	}
</script>

<svelte:window onkeydown={(e) => graphStore.newTaskOpen && e.key === 'Escape' && close()} />

{#if graphStore.newTaskOpen}
	<div class="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto p-6 pt-12">
		<button
			type="button"
			class="bg-ink/55 fixed inset-0 cursor-default"
			aria-label="Close dialog"
			tabindex="-1"
			onclick={close}
		></button>
		<div
			role="dialog"
			aria-modal="true"
			aria-labelledby="new-task-title"
			class="bg-surface relative flex w-full max-w-[640px] flex-col rounded-2xl shadow-[0_24px_64px_rgba(18,22,28,.35)]"
		>
			<div class="border-line-soft flex items-center justify-between border-b px-7 py-5">
				<h1 id="new-task-title" class="m-0 text-xl font-semibold tracking-tight">
					New task in {graphStore.name}
				</h1>
				<button
					type="button"
					aria-label="Close"
					onclick={close}
					class="text-ink-3 -m-2 -mr-3 flex size-11 items-center justify-center"
				>
					<svg
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg
					>
				</button>
			</div>

			<form id="new-task-form" class="flex flex-col gap-5 px-7 py-6" onsubmit={submit}>
				{#if graphStore.error}
					<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm">{graphStore.error}</div>
				{/if}
				<div class="flex flex-col gap-1.5">
					<label for="nt-name" class="text-sm font-medium"
						>Name <span class="text-danger">*</span></label
					>
					<input
						id="nt-name"
						type="text"
						class="field"
						required
						bind:value={name}
						bind:this={nameInput}
					/>
				</div>
				<div class="flex flex-col gap-1.5">
					<label for="nt-desc" class="text-sm font-medium"
						>Description <span class="text-muted font-normal">(Markdown)</span></label
					>
					<textarea
						id="nt-desc"
						rows="3"
						class="field resize-none"
						placeholder="What does done look like?"
						bind:value={description}
					></textarea>
				</div>

				<div class="grid grid-cols-2 gap-4">
					<div class="flex min-w-0 flex-col gap-1.5">
						<label for="nt-type" class="text-sm font-medium">Type</label>
						<select id="nt-type" class="field" bind:value={kind}>
							<option value="task">Task</option>
							<option value="chore">Chore (repeats)</option>
						</select>
					</div>
					<div class="flex min-w-0 flex-col gap-1.5">
						<label for="nt-status" class="text-sm font-medium">Status</label>
						<select id="nt-status" class="field" bind:value={status}>
							{#each [Status.TODO, Status.IN_PROGRESS] as s (s)}
								<option value={s}>{STATUS_META[s].label}</option>
							{/each}
						</select>
					</div>
					<div class="flex min-w-0 flex-col gap-1.5">
						<label for="nt-priority" class="text-sm font-medium">Priority</label>
						<select id="nt-priority" class="field" bind:value={priority}>
							{#each PRIORITY_ORDER as p (p)}
								<option value={p}>{PRIORITY_LABEL[p]}</option>
							{/each}
						</select>
					</div>
					<div class="flex min-w-0 flex-col gap-1.5">
						<label for="nt-assignee" class="text-sm font-medium">Assignee</label>
						<div class="flex">
							<AssigneePicker
								id="nt-assignee"
								value={assignee}
								onchange={(actorId) => (assignee = actorId)}
							/>
						</div>
					</div>
					<div class="flex min-w-0 flex-col gap-1.5">
						<label for="nt-reporter" class="text-sm font-medium">Reporter</label>
						<div class="flex">
							<AssigneePicker
								id="nt-reporter"
								emptyLabel="Nobody"
								value={reporter}
								onchange={(actorId) => (reporter = actorId)}
							/>
						</div>
					</div>
				</div>

				<div class="relative flex flex-col gap-1.5">
					<label for="nt-blockers" class="text-sm font-medium">Blocked by</label>
					<div
						class="border-line-strong flex min-h-11 flex-wrap items-center gap-1.5 rounded-[10px] border px-2.5 py-1.5"
					>
						{#each blockers as id (id)}
							{@const t = graphStore.byId.get(id)}
							{#if t}
								<span
									class="bg-surface-2 inline-flex h-[30px] items-center gap-1.5 rounded-lg pr-1.5 pl-2.5 text-[13px]"
								>
									<span class="text-muted font-mono">{shortId(t)}</span>{t.name}
									<button
										type="button"
										class="text-muted px-1"
										aria-label="Remove blocker {t.name}"
										onclick={() => (blockers = blockers.filter((b) => b !== id))}>×</button
									>
								</span>
							{/if}
						{/each}
						<input
							id="nt-blockers"
							type="text"
							class="min-w-40 flex-1 border-0 bg-transparent px-1 text-sm focus:ring-0 focus:outline-none"
							placeholder="Search tasks to add a blocker"
							autocomplete="off"
							bind:value={search}
						/>
					</div>
					{#if suggestions.length > 0}
						<ul
							class="border-line bg-surface absolute top-full right-0 left-0 z-10 m-0 mt-1 list-none rounded-[10px] border p-1 shadow-lg"
						>
							{#each suggestions as t (t.id)}
								<li>
									<button
										type="button"
										class="hover:bg-canvas flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm"
										onclick={() => {
											blockers = [...blockers, t.id];
											search = '';
										}}
									>
										<span class="text-muted font-mono text-xs">{shortId(t)}</span>{t.name}
									</button>
								</li>
							{/each}
						</ul>
					{/if}
					<span class="text-muted text-[13px]"
						>All blockers must be done before this task can start. Blockers must be in this graph.</span
					>
				</div>

				<div class="flex flex-col gap-1.5">
					<label for="nt-tags" class="text-sm font-medium">Tags</label>
					<div class="flex flex-wrap gap-1.5">
						{#each tags as tag (tag)}
							<span
								class="bg-surface-2 text-ink-3 inline-flex h-[26px] items-center rounded-md px-2.5 text-[13px]"
								>{tag}
								<button
									type="button"
									class="text-muted pl-1.5"
									aria-label="Remove tag {tag}"
									onclick={() => (tags = tags.filter((t) => t !== tag))}>×</button
								></span
							>
						{/each}
					</div>
					<input
						id="nt-tags"
						type="text"
						class="field"
						placeholder="Add a tag and press Enter"
						bind:value={tagDraft}
						onkeydown={(e) => {
							if (e.key === 'Enter') {
								e.preventDefault();
								addTag();
							}
						}}
					/>
				</div>
			</form>

			<div
				class="border-line-soft bg-surface-3 flex justify-end gap-3 rounded-b-2xl border-t px-7 py-4"
			>
				<button type="button" class="btn btn-secondary" onclick={close}>Cancel</button>
				<button type="submit" form="new-task-form" class="btn btn-primary" disabled={saving}>
					{saving ? 'Creating…' : 'Create task'}
				</button>
			</div>
		</div>
	</div>
{/if}
