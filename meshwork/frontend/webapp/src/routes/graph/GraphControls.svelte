<script lang="ts">
	import { Panel, useSvelteFlow, useViewport } from '@xyflow/svelte';

	let { onLayout }: { onLayout: () => void } = $props();

	const { zoomIn, zoomOut, fitView } = useSvelteFlow();
	const viewport = useViewport();

	const btn = 'h-10 border-0 bg-surface text-ink';
</script>

<Panel position="bottom-left">
	<div
		role="group"
		aria-label="Zoom"
		class="border-line bg-surface flex items-center overflow-hidden rounded-[10px] border"
	>
		<button type="button" class="{btn} w-10 text-lg" aria-label="Zoom out" onclick={() => zoomOut()}
			>−</button
		>
		<span class="text-ink-3 px-2.5 text-[13px] font-medium"
			>{Math.round(viewport.current.zoom * 100)}%</span
		>
		<button type="button" class="{btn} w-10 text-lg" aria-label="Zoom in" onclick={() => zoomIn()}
			>+</button
		>
		<button
			type="button"
			class="{btn} border-line border-l px-3.5 text-[13px] font-medium"
			onclick={() => fitView({ duration: 200 })}>Fit</button
		>
		<button
			type="button"
			class="{btn} border-line border-l px-3.5 text-[13px] font-medium"
			onclick={onLayout}>Auto layout</button
		>
	</div>
</Panel>
