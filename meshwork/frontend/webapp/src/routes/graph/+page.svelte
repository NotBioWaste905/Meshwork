<script>
	import {
		SvelteFlowProvider,
		Controls,
		Background,
		SvelteFlow,
		useSvelteFlow,
		Panel
	} from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import { onMount } from 'svelte';
	import Dagre from '@dagrejs/dagre';

	import TaskNode from './TaskNode.svelte';
	import { api } from '$lib/api';

	import { Button } from 'flowbite-svelte';

	const nodeTypes = {
		task: TaskNode
	};

	let nodes = $state([]);
	let edges = $state([]);
	let currentGraphId = $state('abc123');

	// Initialize or get graph
	onMount(async () => {
		try {
			// For now, create a new graph each time
			// In a real app, you'd probably want to save/load graph IDs
			const response = await api.createGraph();
			// currentGraphId = response.ID;
			console.log('Created graph:', currentGraphId);

			// Load existing tasks
			await loadTasks();
		} catch (error) {
			console.error('Failed to initialize graph:', error);
		}
	});

	async function loadTasks() {
		if (!currentGraphId) return;

		try {
			const tasks = await api.getAllTasks(currentGraphId);
			console.log('Loaded tasks:', tasks);
			console.log('Number of tasks:', tasks.length);
			tasks.forEach((task) => {
				console.log(`Task ${task.id} (${task.name}): depends on`, task.depends_on);
			});

			// Convert tasks to nodes
			nodes = tasks.map((task, index) => ({
				id: task.id,
				type: 'task',
				position: { x: 100 + (index % 3) * 350, y: 100 + Math.floor(index / 3) * 200 },
				data: { ...task, graphId: currentGraphId }
			}));

			// Create a set of all existing node IDs for validation
			const nodeIds = new Set(tasks.map((task) => task.id));

			// Create edges array - ensure all referenced nodes exist
			const newEdges = [];
			for (const task of tasks) {
				for (let dependency of task.depends_on) {
					// Only create edge if both source and target nodes exist
					if (nodeIds.has(dependency) && nodeIds.has(task.id)) {
						console.log('Creating edge from', dependency, 'to', task.id);
						newEdges.push({
							id: `edge-${dependency}-${task.id}`,
							source: dependency,
							target: task.id
						});
					} else {
						console.warn('Skipping edge: missing node', {
							dependency,
							taskId: task.id,
							hasSource: nodeIds.has(dependency),
							hasTarget: nodeIds.has(task.id)
						});
					}
				}
			}
			edges = newEdges;
			console.log('Created edges:', edges);
		} catch (error) {
			console.error('Failed to load tasks:', error);
		}
	}

	async function addNewTask() {
		if (!currentGraphId) return;

		try {
			const newTask = {
				name: 'New Task',
				description: 'Click to edit this task'
			};

			await api.addTask(currentGraphId, newTask);
			await loadTasks(); // Reload to get the task with proper ID
		} catch (error) {
			console.error('Failed to add task:', error);
		}
	}

	function onConnect(params) {
		console.log('Connecting nodes:', params);
		// send update to backend
		api.connectNodes(currentGraphId, params.source, params.target);
	}

	function onDisconnect(params) {
		console.log('Disconnecting nodes:', params);
		// send update to backend
		api.disconnectNodes(currentGraphId, params.edge.source, params.edge.target);
		// Update edges array
		edges = edges.filter((edge) => edge.id !== params.edge.id);
	}

	// Handle node data updates
	function onNodeChange(changes) {
		// Handle node updates here if needed
		console.log('Node changes:', changes);
	}

	// Handle task updates from nodes
	async function onTaskUpdate(taskId, updatedData) {
		try {
			await api.updateTask(currentGraphId, taskId, updatedData);
			// Optionally reload tasks to ensure consistency
			// await loadTasks();
		} catch (error) {
			console.error('Failed to update task:', error);
		}
	}
	function getLayoutedElements(nodes, edges, options) {
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
					sourcePosition: options.direction === 'LR' ? 'right' : 'bottom',
					targetPosition: options.direction === 'LR' ? 'left' : 'top'
				};
			}),
			edges
		};
	}

	function onLayout(direction) {
		const layouted = getLayoutedElements(nodes, edges, { direction });

		nodes = [...layouted.nodes];
		edges = [...layouted.edges];

		fitView();
	}
</script>

<div class="graph-container">
	<div class="toolbar">
		<span class="graph-id">Graph ID: {currentGraphId}</span>
	</div>
	<div class="flow-container">
		<SvelteFlowProvider>
			<SvelteFlow
				bind:nodes
				{nodeTypes}
				bind:edges
				fitView
				onNodesChange={onNodeChange}
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

	.flow-container {
		flex: 1;
	}
</style>
