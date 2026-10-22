<script lang="ts">
	import { Handle, Position, type NodeProps } from '@xyflow/svelte';
	import ActorAvatar from '$lib/components/ActorAvatar.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { Status } from '$lib/api';
	import type { TaskFlowNode } from './types';

	let { data }: NodeProps<TaskFlowNode> = $props();

	let blocked = $derived(data.status === Status.BLOCKED);
	let border = $derived(
		data.selected
			? '2px solid var(--color-accent)'
			: blocked
				? '1.5px solid var(--color-danger-dot)'
				: data.chore
					? '1.5px dashed var(--color-faint)'
					: '1px solid var(--color-dot-grid)'
	);
</script>

<div
	class="task-node box-border h-[88px] w-40 rounded-xl p-2.5"
	style="background:{data.terminal ? 'var(--color-ink)' : '#fff'};border:{border};opacity:{data.dim
		? 0.3
		: 1};box-shadow:{data.selected
		? '0 0 0 4px rgba(35,73,199,.18)'
		: '0 1px 2px rgba(18,22,28,.08)'}"
>
	<Handle type="target" position={Position.Left} />
	<Handle type="source" position={Position.Right} />

	<div class="flex items-center justify-between gap-1.5">
		{#if data.chore}
			<span
				class="bg-st-todo-bg text-st-todo-fg inline-flex h-5 items-center rounded-full px-2 text-[11px] font-semibold tracking-wide"
				>CHORE</span
			>
		{:else}
			<StatusPill status={data.status} size="xs" dot />
		{/if}
		{#if data.users[0]}
			<ActorAvatar id={data.users[0]} size={22} />
		{/if}
	</div>
	<div
		class="mt-1.5 line-clamp-2 text-[13px] leading-4 font-medium"
		style="color:{data.terminal ? '#fff' : 'var(--color-ink)'}"
		title={data.name}
	>
		{data.name || 'Unnamed task'}
	</div>
</div>

<style>
	:global(.svelte-flow__handle) {
		width: 10px;
		height: 10px;
		background: var(--color-accent);
		border: 2px solid #fff;
		border-radius: 50%;
		min-width: 0;
		min-height: 0;
	}

	:global(.svelte-flow__handle.connectingfrom) {
		background: var(--color-st-done-dot);
	}

	:global(.svelte-flow__handle.connectingto) {
		background: var(--color-st-progress-dot);
	}
</style>
