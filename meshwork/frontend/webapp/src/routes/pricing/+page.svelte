<script>
	import {
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell
	} from 'flowbite-svelte';
</script>

<div>
	<!-- need a table that shows Free/self-hosted, Cloud, Enterprise -->
	<Table border={false} class="mx-auto text-center">
		<caption
			class="bg-white p-5 text-left text-lg font-semibold text-gray-900 dark:bg-gray-800 dark:text-white"
		>
			Pricing plans</caption
		>
		<TableHead class="bg-orange-500">
			<TableHeadCell></TableHeadCell>
			<TableHeadCell class="text-md">Free/Self-Hosted</TableHeadCell>
			<TableHeadCell class="text-md">Cloud</TableHeadCell>
			<TableHeadCell class="text-md">Enterprise</TableHeadCell>
		</TableHead>
		<TableBody>
			<TableBodyRow>
				<TableBodyCell><b>Price</b></TableBodyCell>
				<TableBodyCell>0$/month</TableBodyCell>
				<TableBodyCell>10$/month</TableBodyCell>
				<TableBodyCell>5$/user</TableBodyCell>
			</TableBodyRow>
			<!-- here be dragons (and features) -->
			<TableBodyRow>
				<TableBodyCell><b>Feature 1</b></TableBodyCell>
				<TableBodyCell>+</TableBodyCell>
				<TableBodyCell>+</TableBodyCell>
				<TableBodyCell>+</TableBodyCell>
			</TableBodyRow>
			<TableBodyRow>
				<TableBodyCell><b>Feature 2</b></TableBodyCell>
				<TableBodyCell>-</TableBodyCell>
				<TableBodyCell>+</TableBodyCell>
				<TableBodyCell>+</TableBodyCell>
			</TableBodyRow>
			<TableBodyRow>
				<TableBodyCell><b>Feature 3</b></TableBodyCell>
				<TableBodyCell>-</TableBodyCell>
				<TableBodyCell>-</TableBodyCell>
				<TableBodyCell>+</TableBodyCell>
			</TableBodyRow>
		</TableBody>
	</Table>
</div>

<style>
	div {
		margin: auto;
		width: 100%;
		margin-top: -10rem;
		display: flex;
		justify-content: center;
		align-items: center;
		min-height: 100vh;
	}
</style>
