<script>
	import { Button } from 'flowbite-svelte';
</script>

<div class="p-8">
	<h1>Welcome to <b>Meshwork</b>!</h1>
	<p>
		Meshwork is a project management system that represents tasks as a directed graph to ensure
		visibility of task blocking, dependencies, and bottlenecks. It supports heavy automation
		including GitHub Workflows integration, GitLab CI/CD, artifact-demanding tasks, and automated PR
		reviews.
		<br />
		<br />
		<Button href="/login">Login</Button>
	</p>
</div>

<style>
	h1 {
		font-size: 2rem;
		/* font-weight: bold; */
		margin-bottom: 1rem;
	}
	div {
		margin-inline-start: 20%;
		margin-inline-end: 20%;
		text-align: center;
	}
</style>
