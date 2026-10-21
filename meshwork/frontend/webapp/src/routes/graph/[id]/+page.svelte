<script lang="ts">
	import {
		SvelteFlowProvider,
		Controls,
		Background,
		SvelteFlow,
		Panel,
		Position,
		type Connection,
		type NodeTypes
	} from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import Dagre from '@dagrejs/dagre';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	import TaskNode from '../TaskNode.svelte';
	import { LAST_GRAPH_KEY } from '../lastGraph';
	import type { TaskFlowEdge, TaskFlowNode } from '../types';
	import { api, ApiError, type Task } from '$lib/api';

	import { Button } from 'flowbite-svelte';

	const nodeTypes: NodeTypes = {
		task: TaskNode
	};

	let nodes = $state.raw<TaskFlowNode[]>([]);
	let edges = $state.raw<TaskFlowEdge[]>([]);
	let errorMessage = $state('');
	let currentGraphId = $derived(page.params.id ?? '');

	$effect(() => {
		loadTasks(currentGraphId);
	});

	function showError(action: string, error: unknown) {
		console.error(`Failed to ${action}:`, error);
		errorMessage = `Failed to ${action}: ${error instanceof Error ? error.message : error}`;
	}

	function rememberGraph(graphId: string | null) {
		try {
			if (graphId) localStorage.setItem(LAST_GRAPH_KEY, graphId);
			else localStorage.removeItem(LAST_GRAPH_KEY);
		} catch {
			// storage unavailable, nothing to remember
		}
	}

	function toNode(task: Task, index: number): TaskFlowNode {
		return {
			id: task.id,
			type: 'task',
			position: { x: 100 + (index % 3) * 350, y: 100 + Math.floor(index / 3) * 200 },
			data: task
		};
	}

	async function loadTasks(graphId: string) {
		let tasks: Task[];
		try {
			tasks = await api.getAllTasks(graphId);
		} catch (error) {
			if (error instanceof ApiError && error.status === 404) {
				// Unknown graph: forget it if it was the remembered one, then let /graph pick one.
				try {
					if (localStorage.getItem(LAST_GRAPH_KEY) === graphId) rememberGraph(null);
				} catch {
					// storage unavailable
				}
				await goto('/graph', { replaceState: true });
				return;
			}
			showError('load tasks', error);
			return;
		}
		rememberGraph(graphId);
		errorMessage = '';

		nodes = tasks.map(toNode);

		// Edge direction on the canvas: dependency (blocker) -> dependent task.
		const nodeIds = new Set(tasks.map((task) => task.id));
		edges = tasks.flatMap((task) =>
			task.depends_on
				.filter((dependency) => nodeIds.has(dependency))
				.map((dependency) => ({
					id: `edge-${dependency}-${task.id}`,
					source: dependency,
					target: task.id
				}))
		);
	}

	async function addNewTask() {
		try {
			const task = await api.addTask(currentGraphId, {
				name: 'New Task',
				description: 'Click to edit this task'
			});
			nodes = [...nodes, toNode(task, nodes.length)];
		} catch (error) {
			showError('add task', error);
		}
	}

	async function onConnect(params: Connection) {
		try {
			await api.connectNodes(currentGraphId, params.source, params.target);
		} catch (error) {
			showError('connect tasks', error);
			await loadTasks(currentGraphId);
		}
	}

	async function onDisconnect(params: { edge: TaskFlowEdge }) {
		try {
			await api.disconnectNodes(currentGraphId, params.edge.source, params.edge.target);
			edges = edges.filter((edge) => edge.id !== params.edge.id);
		} catch (error) {
			showError('disconnect tasks', error);
			await loadTasks(currentGraphId);
		}
	}

	function getLayoutedElements(
		nodes: TaskFlowNode[],
		edges: TaskFlowEdge[],
		options: { direction: 'LR' | 'TB' }
	) {
		const g = new Dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
		g.setGraph({ rankdir: options.direction });

		edges.forEach((edge) => g.setEdge(edge.source, edge.target));
		nodes.forEach((node) =>
			g.setNode(node.id, {
				...node,
				width: node.measured?.width ?? 0,
				height: node.measured?.height ?? 0
			})
		);

		Dagre.layout(g);

		return {
			nodes: nodes.map((node) => {
				const position = g.node(node.id);
				// We are shifting the dagre node position (anchor=center center) to the top left
				// so it matches the Svelte Flow node anchor point (top left).
				const x = position.x - (node.measured?.width ?? 0) / 2;
				const y = position.y - (node.measured?.height ?? 0) / 2;

				return {
					...node,
					position: { x, y },
					sourcePosition: options.direction === 'LR' ? Position.Right : Position.Bottom,
					targetPosition: options.direction === 'LR' ? Position.Left : Position.Top
				};
			}),
			edges
		};
	}

	function onLayout(direction: 'LR' | 'TB') {
		const layouted = getLayoutedElements(nodes, edges, { direction });

		nodes = [...layouted.nodes];
		edges = [...layouted.edges];
	}
</script>

<div class="graph-container">
	<div class="toolbar">
		{#if errorMessage}
			<span class="error">{errorMessage}</span>
		{/if}
		<span class="graph-id">Graph ID: {currentGraphId}</span>
	</div>
	<div class="flow-container">
		<SvelteFlowProvider>
			<SvelteFlow
				bind:nodes
				{nodeTypes}
				bind:edges
				fitView
				onconnect={onConnect}
				onedgeclick={onDisconnect}
			>
				<Panel position="top-right">
					<Button color="light" onclick={() => onLayout('LR')}>Sort</Button>
					<Button onclick={addNewTask} class="add-task-btn">+ Add Task</Button>
				</Panel>
				<Controls />
				<Background />
			</SvelteFlow>
		</SvelteFlowProvider>
	</div>
</div>

<style>
	.graph-container {
		height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.toolbar {
		padding: 12px 16px;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
		display: flex;
		justify-content: flex-end;
		align-items: center;
	}

	.add-task-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 14px;
	}

	.add-task-btn:hover {
		background: #2563eb;
	}

	.graph-id {
		font-size: 12px;
		color: #6b7280;
		font-family: monospace;
	}

	.error {
		margin-right: auto;
		font-size: 13px;
		color: #dc2626;
	}

	.flow-container {
		flex: 1;
	}
</style>
