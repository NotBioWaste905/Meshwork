import type { Edge, Node } from '@xyflow/svelte';
import type { Task } from '$lib/api';

/** Node data: the task plus view state the canvas derives from the whole graph. */
export type TaskNodeData = Task & {
	/** Nothing depends on this task: it is a goal of the graph. */
	terminal: boolean;
	/** On the frontier: TODO with every blocker DONE, can be started now. */
	ready: boolean;
	/** Tagged `chore`: repeating work, drawn dashed. */
	chore: boolean;
	selected: boolean;
	/** Filtered out by the chips above the canvas. */
	dim: boolean;
};

export type TaskFlowNode = Node<TaskNodeData, 'task'>;
export type TaskFlowEdge = Edge;

export const NODE_WIDTH = 160;
export const NODE_HEIGHT = 88;

/** What the canvas context menu was opened on, at client (screen) coordinates. */
export type MenuTarget =
	| { kind: 'pane'; x: number; y: number }
	| { kind: 'node'; x: number; y: number; id: string };
