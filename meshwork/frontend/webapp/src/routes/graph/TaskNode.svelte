<script lang="ts">
	import { useSvelteFlow, type NodeProps, Handle, Position } from '@xyflow/svelte';
	import { Status, api } from '$lib/api';
	import type { TaskFlowNode } from './types';
	import { Modal, Button } from 'flowbite-svelte';

	let { id, data }: NodeProps<TaskFlowNode> = $props();
	let { updateNodeData } = useSvelteFlow();

	// Form state, filled from the task each time the modal opens
	let editModal = $state(false);
	let editName = $state('');
	let editDescription = $state('');
	let editStatus = $state(Status.TODO);
	let editTags = $state('');
	let editUsers = $state('');
	let isSaving = $state(false);
	let errorMessage = $state('');

	function splitList(value: string): string[] {
		const items = value
			.split(',')
			.map((item) => item.trim())
			.filter((item) => item);
		return [...new Set(items)];
	}

	function openEditor() {
		editName = data.name;
		editDescription = data.description;
		editStatus = data.status;
		editTags = data.tags.join(', ');
		editUsers = data.users.join(', ');
		errorMessage = '';
		editModal = true;
	}

	async function saveTask(event: SubmitEvent) {
		event.preventDefault();
		if (isSaving) return;

		isSaving = true;
		errorMessage = '';

		try {
			const saved = await api.updateTask(data.graph_id, id, {
				name: editName,
				description: editDescription,
				status: editStatus,
				tags: splitList(editTags),
				users: splitList(editUsers)
			});
			updateNodeData(id, saved);
			editModal = false;
		} catch (error) {
			console.error('Failed to update task:', error);
			errorMessage = `Failed to save task: ${error instanceof Error ? error.message : error}`;
		} finally {
			isSaving = false;
		}
	}

	function getStatusColor(status: string) {
		switch (status) {
			case 'TODO':
				return '#e2e8f0';
			case 'IN_PROGRESS':
				return '#fbbf24';
			case 'DONE':
				return '#10b981';
			case 'REVIEW':
				return '#8b5cf6';
			case 'BLOCKED':
				return '#ef4444';
			default:
				return '#e2e8f0';
		}
	}
</script>

<div class="task-node" style="border-left-color: {getStatusColor(data.status)}">
	<!-- Input handle for incoming dependencies -->
	<Handle type="target" position={Position.Left} />

	<!-- Output handle for outgoing dependencies -->
	<Handle type="source" position={Position.Right} />

	<div class="task-header">
		<h3 class="task-title">{data.name || 'Unnamed Task'}</h3>
		<button class="edit-btn" onclick={openEditor}> ✎ </button>
		<Modal title="Edit the task" bind:open={editModal}>
			<form class="flex flex-col space-y-2" onsubmit={saveTask}>
				{#if errorMessage}
					<div class="alert alert-error rounded bg-red-100 p-2 text-sm text-red-700">
						{errorMessage}
					</div>
				{/if}
				<input
					type="text"
					placeholder="Task name"
					class="input input-bordered w-full"
					bind:value={editName}
					required
				/>
				<textarea
					bind:value={editDescription}
					class="textarea textarea-bordered w-full"
					placeholder="Task description"
					rows="3"
				></textarea>
				<select class="w-full" bind:value={editStatus}>
					{#each Object.values(Status) as status (status)}
						<option value={status}>{status}</option>
					{/each}
				</select>
				<input
					type="text"
					placeholder="Tags (comma separated)"
					class="input input-bordered w-full"
					bind:value={editTags}
				/>
				<input
					type="text"
					placeholder="Users (comma separated)"
					class="input input-bordered w-full"
					bind:value={editUsers}
				/>
				<Button type="submit" class="w-full" disabled={isSaving}>
					{isSaving ? 'Saving...' : 'Save'}
				</Button>
			</form>
		</Modal>
	</div>

	{#if data.description}
		<p class="task-description">{data.description}</p>
	{/if}

	<div class="task-status">
		<span class="status-badge" style="background-color: {getStatusColor(data.status)}">
			{data.status}
		</span>
	</div>

	<div class="task-details">
		{#if data.tags && data.tags.length > 0}
			<div class="task-tags">
				{#each data.tags as tag (tag)}
					<span class="tag">{tag}</span>
				{/each}
			</div>
		{/if}
		<div class="task-user">
			{#each data.users as user (user)}
				<span class="user">{user}</span>
			{/each}
		</div>
	</div>
</div>

<style>
	.task-node {
		min-width: 200px;
		min-height: 80px;
		background: white;
		border: 1px solid #d1d5db;
		border-left-width: 4px;
		border-radius: 6px;
		padding: 12px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.task-user {
		display: flex;
		align-items: right;
		margin-top: 8px;
		width: 100%;
	}

	.task-user .user {
		/* margin-right: 8px; */
		font-size: 12px;
		color: #6b7280;
	}

	.task-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 8px;
	}

	.task-title {
		margin: 0;
		font-size: 14px;
		font-weight: 600;
		color: #374151;
		flex: 1;
	}

	.task-title-input {
		flex: 1;
		border: 1px solid #d1d5db;
		border-radius: 4px;
		padding: 4px 8px;
		font-size: 14px;
		font-weight: 600;
	}

	.edit-btn {
		background: none;
		border: none;
		font-size: 12px;
		cursor: pointer;
		padding: 4px;
		border-radius: 3px;
	}

	.edit-btn:hover {
		background: #f3f4f6;
	}

	.task-description {
		font-size: 12px;
		color: #6b7280;
		margin: 0 0 8px 0;
		line-height: 1.4;
	}

	.task-status {
		margin-bottom: 8px;
	}

	.status-badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: 12px;
		font-size: 10px;
		font-weight: 500;
		color: #374151;
		text-transform: uppercase;
	}

	.task-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	.tag {
		background: #e5e7eb;
		color: #374151;
		padding: 2px 6px;
		border-radius: 8px;
		font-size: 10px;
	}

	/* Handle styling */
	:global(.svelte-flow__handle) {
		width: 15px;
		height: 15px;
		background: #3b82f6;
		border: 2px solid white;
		border-radius: 50%;
	}

	:global(.svelte-flow__handle:hover) {
		background: #2563eb;
		/* transform: scale(1.2); */
	}

	:global(.svelte-flow__handle.connectingfrom) {
		background: #10b981;
	}

	:global(.svelte-flow__handle.connectingto) {
		background: #f59e0b;
	}
</style>
