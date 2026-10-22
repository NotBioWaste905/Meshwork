<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type Actor, type ActorKind } from '$lib/api';
	import Avatar from '$lib/components/Avatar.svelte';
	import { liveRefresh, mergeById } from '$lib/liveRefresh';

	let actors = $state<Actor[]>([]);
	let loading = $state(true);
	let error = $state('');
	let saving = $state(false);

	let newName = $state('');
	let newKind = $state<ActorKind>('human');

	/** The row with an open merge or delete panel; at most one at a time. */
	let open = $state<{ id: string; action: 'merge' | 'delete' } | null>(null);
	let mergeInto = $state('');
	/** A name field has focus: a poll must not overwrite what is being typed. */
	let typing = $state(false);

	const tasks = (n: number) => `${n} task${n === 1 ? '' : 's'}`;
	const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

	/** Background polls skip while a panel is open or a name is edited; own writes always reload. */
	async function load(poll = false) {
		if (poll && (open || typing || saving)) return;
		try {
			actors = mergeById(actors, await api.listActors());
			error = '';
		} catch (e) {
			if (loading || actors.length === 0) error = `Failed to load actors: ${message(e)}`;
			else console.warn('Background refresh failed:', e);
		}
		loading = false;
	}

	// Agents may be created or reassigned elsewhere: poll while visible.
	onMount(() => {
		load();
		return liveRefresh(() => load(true));
	});

	/** Runs a write, then reloads the list; a failure is shown and the list stays as the server has it. */
	async function run(action: string, request: () => Promise<unknown>): Promise<boolean> {
		saving = true;
		let failure = '';
		try {
			await request();
		} catch (e) {
			failure = `Failed to ${action}: ${message(e)}`;
		}
		saving = false;
		await load();
		error = failure; // after the reload, which clears it
		return !failure;
	}

	async function add(event: SubmitEvent) {
		event.preventDefault();
		const name = newName.trim();
		if (saving || !name) return;
		if (await run('add actor', () => api.addActor({ name, kind: newKind }))) newName = '';
	}

	async function rename(actor: Actor, input: HTMLInputElement) {
		const name = input.value.trim();
		if (!name || name === actor.name) {
			input.value = actor.name;
			return;
		}
		if (!(await run('rename actor', () => api.updateActor(actor.id, { name })))) {
			input.value = actor.name;
		}
	}

	function onNameKeydown(event: KeyboardEvent, actor: Actor) {
		const input = event.currentTarget as HTMLInputElement;
		if (event.key === 'Enter') input.blur();
		else if (event.key === 'Escape') {
			input.value = actor.name;
			input.blur();
		}
	}

	function setKind(actor: Actor, kind: ActorKind) {
		run('change kind', () => api.updateActor(actor.id, { kind }));
	}

	function toggle(actor: Actor, action: 'merge' | 'delete') {
		if (open?.id === actor.id && open.action === action) {
			open = null;
			return;
		}
		open = { id: actor.id, action };
		mergeInto = actors.find((a) => a.id !== actor.id)?.id ?? '';
	}

	async function merge(actor: Actor) {
		if (!mergeInto) return;
		if (await run('merge actors', () => api.mergeActor(actor.id, mergeInto))) open = null;
	}

	async function remove(actor: Actor) {
		if (await run('delete actor', () => api.deleteActor(actor.id, actor.task_count > 0))) {
			open = null;
		}
	}
</script>

<svelte:head><title>Actors · Meshwork</title></svelte:head>

<main class="box-border flex max-w-4xl flex-col gap-8 px-10 pt-8 pb-12">
	<header class="flex flex-col gap-1">
		<h1 class="m-0 text-3xl font-semibold tracking-tight">Actors</h1>
		<p class="text-muted m-0 text-[15px]">
			People and AI agents who take tasks. Shared by all graphs.
		</p>
	</header>

	<form class="flex flex-wrap items-center gap-3" onsubmit={add}>
		<input
			type="text"
			class="field w-64 text-[15px]"
			placeholder="New actor name"
			aria-label="New actor name"
			autocomplete="off"
			bind:value={newName}
		/>
		<select class="field w-auto text-[15px]" aria-label="New actor kind" bind:value={newKind}>
			<option value="human">Human</option>
			<option value="agent">Agent</option>
		</select>
		<button type="submit" class="btn btn-primary" disabled={saving || !newName.trim()}>Add</button>
	</form>

	{#if error}
		<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm" role="alert">{error}</div>
	{/if}

	{#if !loading && actors.length === 0 && !error}
		<div class="card flex flex-col gap-2 p-6">
			<strong class="font-semibold">No actors yet</strong>
			<span class="text-muted text-sm">Add one above, or from the assignee picker of a task.</span>
		</div>
	{:else if actors.length > 0}
		<ul class="card m-0 list-none divide-y divide-[var(--color-line-soft)] p-0" aria-label="Actors">
			{#each actors as actor (actor.id)}
				<li class="flex flex-col gap-3 px-5 py-3">
					<div class="flex flex-wrap items-center gap-3">
						<Avatar name={actor.name} size={30} agent={actor.kind === 'agent'} />
						<input
							type="text"
							class="field h-9 min-w-40 flex-1 text-[15px]"
							aria-label="Name of {actor.name}"
							autocomplete="off"
							value={actor.name}
							disabled={saving}
							onfocus={() => (typing = true)}
							onblur={(e) => {
								typing = false;
								rename(actor, e.currentTarget);
							}}
							onkeydown={(e) => onNameKeydown(e, actor)}
						/>
						<select
							class="field h-9 w-auto text-sm"
							aria-label="Kind of {actor.name}"
							value={actor.kind}
							disabled={saving}
							onchange={(e) => setKind(actor, e.currentTarget.value as ActorKind)}
						>
							<option value="human">Human</option>
							<option value="agent">Agent</option>
						</select>
						<span class="text-muted w-20 text-right text-sm">{tasks(actor.task_count)}</span>
						<button
							type="button"
							class="btn btn-secondary btn-sm h-9 px-3"
							aria-expanded={open?.id === actor.id && open.action === 'merge'}
							disabled={saving || actors.length < 2}
							onclick={() => toggle(actor, 'merge')}>Merge…</button
						>
						<button
							type="button"
							class="btn btn-secondary btn-sm text-danger h-9 px-3"
							aria-expanded={open?.id === actor.id && open.action === 'delete'}
							disabled={saving}
							onclick={() => toggle(actor, 'delete')}>Delete…</button
						>
					</div>

					{#if open?.id === actor.id && open.action === 'merge'}
						<div class="bg-surface-3 flex flex-wrap items-center gap-2 rounded-lg p-3 text-sm">
							<span>Move {tasks(actor.task_count)} of <strong>{actor.name}</strong> to</span>
							<select
								class="field h-9 w-auto text-sm"
								aria-label="Merge {actor.name} into"
								bind:value={mergeInto}
							>
								{#each actors.filter((a) => a.id !== actor.id) as other (other.id)}
									<option value={other.id}
										>{other.name}{other.kind === 'agent' ? ' (agent)' : ''}</option
									>
								{/each}
							</select>
							<span>and delete {actor.name}.</span>
							<div class="flex-1"></div>
							<button
								type="button"
								class="btn btn-primary btn-sm h-9 px-3"
								disabled={saving || !mergeInto}
								onclick={() => merge(actor)}>Merge</button
							>
							<button
								type="button"
								class="btn btn-secondary btn-sm h-9 px-3"
								onclick={() => (open = null)}>Cancel</button
							>
						</div>
					{:else if open?.id === actor.id && open.action === 'delete'}
						<div class="bg-surface-3 flex flex-wrap items-center gap-2 rounded-lg p-3 text-sm">
							<span>
								Delete <strong>{actor.name}</strong>?
								{#if actor.task_count > 0}
									Their {tasks(actor.task_count)} will be left unassigned; to hand them to someone, merge
									instead.
								{/if}
							</span>
							<div class="flex-1"></div>
							<button
								type="button"
								class="btn btn-sm bg-danger h-9 px-3 text-white"
								disabled={saving}
								onclick={() => remove(actor)}>Delete</button
							>
							<button
								type="button"
								class="btn btn-secondary btn-sm h-9 px-3"
								onclick={() => (open = null)}>Cancel</button
							>
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</main>
