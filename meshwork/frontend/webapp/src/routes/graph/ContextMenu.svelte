<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { useSvelteFlow, type XYPosition } from '@xyflow/svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import type { Status } from '$lib/api';
	import { graphStore } from '$lib/graphStore.svelte';
	import { STATUS_META, STATUS_ORDER } from '$lib/status';
	import { NODE_HEIGHT, NODE_WIDTH, type MenuTarget } from './types';

	let {
		target,
		onclose,
		onplace
	}: {
		target: MenuTarget;
		onclose: () => void;
		/** A task was created from the menu; put its node at `position` (flow coordinates). */
		onplace: (id: string, position: XYPosition) => void;
	} = $props();

	const MARGIN = 8;
	const { screenToFlowPosition } = useSvelteFlow();

	type Mode = 'root' | 'add' | 'rename' | 'delete';

	let mode = $state<Mode>('root');
	let draft = $state('');
	let busy = $state(false);
	let menu = $state<HTMLDivElement>();
	let width = $state(0);
	let height = $state(0);
	let viewportWidth = $state(0);
	let viewportHeight = $state(0);

	let task = $derived(target.kind === 'node' ? graphStore.byId.get(target.id) : undefined);
	// Keep the whole menu on screen when opened near the right or bottom edge.
	let left = $derived(
		Math.max(MARGIN, Math.min(target.x, (viewportWidth || Infinity) - width - MARGIN))
	);
	let top = $derived(
		Math.max(MARGIN, Math.min(target.y, (viewportHeight || Infinity) - height - MARGIN))
	);

	function items(): HTMLElement[] {
		return menu
			? [...menu.querySelectorAll<HTMLElement>('[role^="menuitem"]:not([disabled])')]
			: [];
	}

	async function focusFirst() {
		await tick();
		const input = menu?.querySelector<HTMLInputElement>('input');
		if (input) {
			input.focus();
			input.select();
		} else {
			items()[0]?.focus();
		}
	}

	onMount(focusFirst);

	function enter(next: Mode) {
		mode = next;
		draft = next === 'rename' ? (task?.name ?? '') : '';
		focusFirst();
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			onclose();
			return;
		}
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
		if (event.target instanceof HTMLInputElement) return;
		const list = items();
		if (list.length === 0) return;
		event.preventDefault();
		const at = list.indexOf(document.activeElement as HTMLElement);
		const next =
			event.key === 'Home'
				? 0
				: event.key === 'End'
					? list.length - 1
					: (at + (event.key === 'ArrowDown' ? 1 : -1) + list.length) % list.length;
		list[next].focus();
	}

	/** Outside clicks and scroll/zoom of the canvas close the menu; the flow position is stale. */
	function onoutside(event: Event) {
		if (menu && !menu.contains(event.target as Node)) onclose();
	}

	async function run(action: () => Promise<unknown>) {
		if (busy) return;
		busy = true;
		await action();
		busy = false;
		onclose();
	}

	function addTask(event: SubmitEvent) {
		event.preventDefault();
		const name = draft.trim();
		if (!name || target.kind !== 'pane') return;
		// centre the new node on the clicked point
		const at = screenToFlowPosition({ x: target.x, y: target.y });
		const position = { x: at.x - NODE_WIDTH / 2, y: at.y - NODE_HEIGHT / 2 };
		run(async () => {
			const created = await graphStore.addTask({ name });
			if (created) onplace(created.id, position);
		});
	}

	function setStatus(status: Status) {
		if (!task) return;
		if (status === task.status) return onclose();
		const id = task.id;
		run(() => graphStore.updateTask(id, { status }));
	}

	function rename(event: SubmitEvent) {
		event.preventDefault();
		const name = draft.trim();
		if (!task || !name) return;
		if (name === task.name) return onclose();
		const id = task.id;
		run(() => graphStore.updateTask(id, { name }));
	}

	function remove() {
		if (!task) return;
		const id = task.id;
		run(() => graphStore.deleteTask(id));
	}
</script>

<svelte:window
	bind:innerWidth={viewportWidth}
	bind:innerHeight={viewportHeight}
	onpointerdowncapture={onoutside}
	onwheelcapture={onoutside}
	onresize={onclose}
	onblur={onclose}
/>

<div
	bind:this={menu}
	bind:offsetWidth={width}
	bind:offsetHeight={height}
	role="menu"
	tabindex="-1"
	aria-label={task ? `Task ${task.name}` : 'Canvas'}
	class="ctx border-line bg-surface fixed z-50 w-56 rounded-[10px] border p-1 text-[13px] shadow-[0_8px_24px_rgba(18,22,28,.16)]"
	style="left:{left}px;top:{top}px"
	{onkeydown}
	oncontextmenu={(event) => event.preventDefault()}
>
	{#if target.kind === 'pane'}
		{#if mode === 'add'}
			<form class="flex flex-col gap-1.5 p-1.5" onsubmit={addTask}>
				<label class="text-muted text-xs font-medium" for="ctx-new-task">New task here</label>
				<input
					id="ctx-new-task"
					class="field h-9! text-sm!"
					placeholder="Task name"
					autocomplete="off"
					bind:value={draft}
					disabled={busy}
				/>
				<div class="flex justify-end gap-1.5">
					<button type="button" class="item w-auto!" onclick={onclose}>Cancel</button>
					<button
						type="submit"
						class="btn btn-primary btn-sm h-8! px-3!"
						disabled={busy || !draft.trim()}>Add</button
					>
				</div>
			</form>
		{:else}
			<button type="button" role="menuitem" class="item" onclick={() => enter('add')}>
				<span aria-hidden="true" class="text-muted w-4 text-center">+</span>Add task here
			</button>
		{/if}
	{:else if task}
		{#if mode === 'rename'}
			<form class="flex flex-col gap-1.5 p-1.5" onsubmit={rename}>
				<label class="text-muted text-xs font-medium" for="ctx-rename">Rename task</label>
				<input
					id="ctx-rename"
					class="field h-9! text-sm!"
					autocomplete="off"
					bind:value={draft}
					disabled={busy}
				/>
				<div class="flex justify-end gap-1.5">
					<button type="button" class="item w-auto!" onclick={() => enter('root')}>Cancel</button>
					<button
						type="submit"
						class="btn btn-primary btn-sm h-8! px-3!"
						disabled={busy || !draft.trim()}>Save</button
					>
				</div>
			</form>
		{:else if mode === 'delete'}
			<div class="flex flex-col gap-2 p-1.5">
				<p class="m-0 leading-normal">
					Delete <strong class="font-semibold">{task.name || 'Unnamed task'}</strong>? Its
					dependency edges are removed too.
				</p>
				<div class="flex justify-end gap-1.5">
					<button type="button" role="menuitem" class="item w-auto!" onclick={() => enter('root')}
						>Cancel</button
					>
					<button
						type="button"
						role="menuitem"
						class="item danger w-auto! font-semibold"
						disabled={busy}
						onclick={remove}>Delete</button
					>
				</div>
			</div>
		{:else}
			<div class="text-muted truncate px-2.5 pt-1.5 pb-1 text-xs font-medium" role="presentation">
				{task.name || 'Unnamed task'}
			</div>
			{#each STATUS_ORDER as status (status)}
				<button
					type="button"
					role="menuitemradio"
					aria-checked={task.status === status}
					aria-label={STATUS_META[status].label}
					class="item justify-between"
					disabled={busy}
					onclick={() => setStatus(status)}
				>
					<StatusPill {status} size="xs" dot />
					{#if task.status === status}<span aria-hidden="true" class="text-accent">✓</span>{/if}
				</button>
			{/each}
			<div role="separator" class="bg-line-soft mx-1 my-1 h-px"></div>
			<button type="button" role="menuitem" class="item" onclick={() => enter('rename')}
				>Rename…</button
			>
			<a
				role="menuitem"
				class="item no-underline"
				href="/graph/{graphStore.graphId}/task/{task.id}"
				onclick={onclose}>Open task</a
			>
			<button type="button" role="menuitem" class="item danger" onclick={() => enter('delete')}
				>Delete…</button
			>
		{/if}
	{/if}
</div>

<style>
	.item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		height: 2rem;
		padding: 0 0.625rem;
		border-radius: 6px;
		background: transparent;
		color: var(--color-ink);
		font-size: 13px;
		text-align: left;
		cursor: pointer;
	}
	.item:hover,
	.item:focus-visible {
		background: var(--color-canvas);
		outline: none;
	}
	.item:focus-visible {
		box-shadow: inset 0 0 0 2px var(--color-accent);
	}
	.item:disabled {
		cursor: default;
		opacity: 0.6;
	}
	.danger {
		color: var(--color-danger);
	}
	.danger:hover,
	.danger:focus-visible {
		background: var(--color-st-blocked-bg);
	}
</style>
