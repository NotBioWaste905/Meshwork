<script lang="ts">
	import { useSvelteFlow, type NodeProps } from '@xyflow/svelte';
	import type { Task } from '$lib/api';

	let { id, data }: NodeProps<Task> = $props();
	let { updateNodeData } = useSvelteFlow();

	// Local state for editing
	let isEditing = $state(false);
	let editName = $state(data.name || '');
	let editDescription = $state(data.description || '');

	function toggleEdit() {
		if (isEditing) {
			// Save changes
			updateNodeData(id, {
				...data,
				name: editName,
				description: editDescription
			});
		} else {
			// Enter edit mode
			editName = data.name || '';
			editDescription = data.description || '';
		}
		isEditing = !isEditing;
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
	<div class="task-header">
		{#if isEditing}
			<input bind:value={editName} class="task-title-input nodrag" placeholder="Task name" />
		{:else}
			<h3 class="task-title">{data.name || 'Unnamed Task'}</h3>
		{/if}
		<button class="edit-btn" onclick={toggleEdit}>
			{isEditing ? '✓' : '✎'}
		</button>
	</div>

	{#if isEditing}
		<textarea
			bind:value={editDescription}
			class="task-description-input nodrag"
			placeholder="Task description"
			rows="3"
		></textarea>
	{:else if data.description}
		<p class="task-description">{data.description}</p>
	{/if}

	<div class="task-status">
		<span class="status-badge" style="background-color: {getStatusColor(data.status)}">
			{data.status}
		</span>
	</div>

	{#if data.tags && data.tags.length > 0}
		<div class="task-tags">
			{#each data.tags as tag}
				<span class="tag">{tag}</span>
			{/each}
		</div>
	{/if}
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

	.task-description-input {
		width: 100%;
		border: 1px solid #d1d5db;
		border-radius: 4px;
		padding: 6px 8px;
		font-size: 12px;
		margin-bottom: 8px;
		resize: vertical;
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
</style>
