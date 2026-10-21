<script lang="ts">
	let { note = 'Demo data: no API for this yet' }: { note?: string } = $props();
</script>

<span
	class="border-faint text-muted inline-flex items-center rounded-md border border-dashed px-1.5 py-px text-[11px] font-medium"
	title={note}>mock</span
>
