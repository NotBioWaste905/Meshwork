<script lang="ts">
	import '../app.css';
	import { Navbar, NavBrand, NavLi, NavUl, NavHamburger } from 'flowbite-svelte';

	let { children } = $props();
</script>

<Navbar>
	<NavBrand href="/">
		<span class="self-center text-xl font-semibold whitespace-nowrap dark:text-white"
			>🕸 Meshwork</span
		>
	</NavBrand>
	<NavHamburger />
	<NavUl>
		<NavLi href="/graph">Get Started</NavLi>
		<NavLi href="/pricing">Pricing</NavLi>
		<NavLi href="/contact">Contact</NavLi>
	</NavUl>
</Navbar>

{@render children()}
