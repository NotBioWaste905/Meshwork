<script>
	import { Button } from 'flowbite-svelte';
</script>

<div class="p-8">
	<h1>Contact <b>Meshwork</b></h1>
	<p>
		Have questions about Meshwork? Want to learn more about our project management system? Get in
		touch with us and we'll be happy to help.
		<br />
		<br />
		<Button href="mailto:contact@meshwork.dev">Email Us</Button>
	</p>

	<div class="mt-8">
		<h2 class="mb-4 text-xl">Other Ways to Reach Us</h2>
		<p>
			GitHub: <a href="https://github.com/meshwork" class="text-blue-600 hover:underline"
				>@meshwork</a
			>
			<br />
			Twitter:
			<a href="https://twitter.com/meshwork" class="text-blue-600 hover:underline">@meshwork</a>
		</p>
	</div>
</div>

<style>
	h1 {
		font-size: 2rem;
		margin-bottom: 1rem;
	}
	div {
		margin-inline-start: 20%;
		margin-inline-end: 20%;
		text-align: center;
	}
</style>
