<script>
	import { SvelteFlow } from '@xyflow/svelte';
	import { Tabs, TabItem } from 'flowbite-svelte';

	import '@xyflow/svelte/dist/style.css';

	let nodes = $state.raw([
		{ id: '1', position: { x: 0, y: 0 }, data: { label: '1' } },
		{ id: '2', position: { x: 0, y: 100 }, data: { label: '2' } }
	]);

	let edges = $state.raw([{ id: 'e1-2', source: '1', target: '2' }]);
</script>

<div class="h-screen w-full">
	<SvelteFlow bind:nodes bind:edges />
	<!-- <Tabs tabStyle="pill">
		<TabItem open title="Graph">
			<h1>Graph</h1>
		</TabItem>
		<TabItem title="Kanban">
			<h1>Kanban</h1>
			<p>Kanban content</p>
		</TabItem>
	</Tabs> -->
</div>
