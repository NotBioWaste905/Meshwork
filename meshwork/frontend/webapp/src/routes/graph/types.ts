import type { Edge, Node } from '@xyflow/svelte';
import type { Task } from '$lib/api';

export type TaskFlowNode = Node<Task, 'task'>;
export type TaskFlowEdge = Edge;
