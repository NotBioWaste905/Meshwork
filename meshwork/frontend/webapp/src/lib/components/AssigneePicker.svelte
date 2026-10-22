<script lang="ts">
	import { tick } from 'svelte';
	import type { ActorKind } from '$lib/api';
	import { graphStore } from '$lib/graphStore.svelte';

	/** Picks one actor, or none; "New actor…" creates one in place. */
	let {
		value,
		onchange,
		id,
		small = false,
		label = 'Assignee',
		emptyLabel = 'Unassigned'
	}: {
		/** Actor ID, '' for unassigned. */
		value: string;
		onchange: (actorId: string) => void;
		id?: string;
		/** Compact height for inline property lists. */
		small?: boolean;
		/** Accessible name when there is no visible `<label for={id}>`. */
		label?: string;
		/** Text of the "nobody" option. */
		emptyLabel?: string;
	} = $props();

	const NEW = '__new__';

	let creating = $state(false);
	let name = $state('');
	let kind = $state<ActorKind>('human');
	let saving = $state(false);
	let nameInput = $state<HTMLInputElement>();
	let select = $state<HTMLSelectElement>();

	let height = $derived(small ? 'h-8 text-sm' : '');

	async function pick(event: Event) {
		const selected = (event.currentTarget as HTMLSelectElement).value;
		if (selected !== NEW) {
			onchange(selected);
			return;
		}
		if (select) select.value = value; // keep showing the current assignee meanwhile
		name = '';
		kind = 'human';
		creating = true;
		await tick();
		nameInput?.focus();
	}

	async function create() {
		const trimmed = name.trim();
		if (saving || !trimmed) return;
		saving = true;
		const actor = await graphStore.addActor(trimmed, kind);
		saving = false;
		if (actor) {
			creating = false;
			onchange(actor.id);
		}
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			create();
		} else if (event.key === 'Escape') {
			event.stopPropagation();
			creating = false;
		}
	}
</script>

{#if creating}
	<div class="flex min-w-0 flex-1 flex-wrap items-center gap-1.5">
		<input
			{id}
			bind:this={nameInput}
			type="text"
			class="field min-w-0 flex-1 {height}"
			placeholder="Name"
			aria-label="New actor name"
			autocomplete="off"
			bind:value={name}
			onkeydown={onKeydown}
		/>
		<select class="field w-auto {height}" aria-label="New actor kind" bind:value={kind}>
			<option value="human">Human</option>
			<option value="agent">Agent</option>
		</select>
		<button
			type="button"
			class="btn btn-primary btn-sm {small ? 'h-8 px-3' : ''}"
			disabled={saving || !name.trim()}
			onclick={create}>Add</button
		>
		<button
			type="button"
			class="btn btn-secondary btn-sm {small ? 'h-8 px-3' : ''}"
			onclick={() => (creating = false)}>Cancel</button
		>
	</div>
{:else}
	<select
		{id}
		bind:this={select}
		class="field min-w-0 flex-1 {height}"
		aria-label={id ? undefined : label}
		{value}
		onchange={pick}
	>
		<option value="">{emptyLabel}</option>
		{#each graphStore.actors as actor (actor.id)}
			<option value={actor.id}>{actor.name}{actor.kind === 'agent' ? ' (agent)' : ''}</option>
		{/each}
		{#if value && !graphStore.actorsById.has(value)}
			<option {value}>{value}</option>
		{/if}
		<option value={NEW}>New actor…</option>
	</select>
{/if}
