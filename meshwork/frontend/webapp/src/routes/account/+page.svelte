<script>
	import {
		Progressbar,
		Button,
		Card,
		Listgroup,
		ListgroupItem,
		Heading,
		Avatar,
		Badge
	} from 'flowbite-svelte';

	let user = {
		name: 'John Smith',
		avatar: 'https://flowbite.com/docs/images/people/profile-picture-5.jpg',
		isPro: true
	};

	let teams = [
		{ id: 1, name: 'Engineering Team', taskCount: 12 },
		{ id: 2, name: 'Design Team', taskCount: 8 },
		{ id: 3, name: 'Product Team', taskCount: 15 }
	];

	let taskGraphs = [
		{ id: 1, name: 'API Development', progress: 75 },
		{ id: 2, name: 'Frontend Features', progress: 40 },
		{ id: 3, name: 'Documentation', progress: 90 }
	];
</script>

<div class="p-8">
	<Card class="mb-8 min-w-2xl md:p-2" style="margin-left: auto; margin-right: auto;">
		<div class="flex items-center space-x-4">
			<Avatar src={user.avatar} alt="User profile" size="xl" />
			<h2 class="text-2xl font-bold text-gray-900">{user.name}</h2>
			{#if user.isPro}
				<Badge color="orange" class="px-3 py-1.5 text-sm font-semibold">PRO</Badge>
			{/if}
		</div>
	</Card>

	<div class="grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card class="md:p-4">
			<Heading tag="h2" class="mb-4">My Teams</Heading>
			<Listgroup active>
				{#each teams as team}
					<ListgroupItem class="cursor-pointer hover:bg-orange-50">
						<div class="flex w-full items-center justify-between">
							<span>{team.name}</span>
							<span class="text-sm text-orange-600">{team.taskCount} tasks</span>
						</div>
					</ListgroupItem>
				{/each}
			</Listgroup>
			<div class="mt-4">
				<Button color="orange">Create New Team</Button>
			</div>
		</Card>

		<Card class="md:p-4">
			<Heading tag="h2" class="mb-4">Task Graphs</Heading>
			<Listgroup active>
				{#each taskGraphs as graph}
					<ListgroupItem class="cursor-pointer hover:bg-orange-50">
						<Progressbar progress={graph.progress} labelOutside={graph.name} />
					</ListgroupItem>
				{/each}
			</Listgroup>
			<div class="mt-4">
				<Button color="orange">Create New Graph</Button>
			</div>
		</Card>
	</div>
</div>

<style>
	div {
		margin-inline-start: 20%;
		margin-inline-end: 20%;
		text-align: center;
	}
</style>
