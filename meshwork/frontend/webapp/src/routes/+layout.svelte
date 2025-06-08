<script lang="ts">
	import '../app.css';

	let { children } = $props();
	let isDrawerOpen = $state(false);

	function toggleDrawer() {
		isDrawerOpen = !isDrawerOpen;
	}

	function closeDrawer() {
		isDrawerOpen = false;
	}
</script>

<!-- Hamburger Menu Button -->
<button class="hamburger-btn" onclick={toggleDrawer} aria-label="Toggle navigation menu">
	<div class="hamburger-icon" class:open={isDrawerOpen}>
		<span></span>
		<span></span>
		<span></span>
	</div>
</button>

<!-- Overlay -->
{#if isDrawerOpen}
	<div 
		class="drawer-overlay" 
		onclick={closeDrawer}
		onkeydown={(e) => e.key === 'Escape' && closeDrawer()}
		role="button"
		tabindex="0"
		aria-label="Close navigation menu"
	></div>
{/if}

<!-- Drawer -->
<div class="drawer" class:drawer-open={isDrawerOpen}>
	<div class="drawer-content">
		<div class="drawer-header">
			<a href="/" class="brand-link" onclick={closeDrawer}>
				<span class="self-center text-xl font-semibold whitespace-nowrap dark:text-white">
					🕸 Meshwork
				</span>
			</a>
			<button class="close-btn" onclick={closeDrawer} aria-label="Close menu"> × </button>
		</div>

		<nav class="drawer-nav">
			<ul class="nav-list">
				<li><a href="/graph" class="drawer-link" onclick={closeDrawer}>Get Started</a></li>
				<li><a href="/pricing" class="drawer-link" onclick={closeDrawer}>Pricing</a></li>
				<li><a href="/contact" class="drawer-link" onclick={closeDrawer}>Contact</a></li>
				<li><a href="/account" class="drawer-link" onclick={closeDrawer}>Account</a></li>
				<li><a href="/login" class="drawer-link" onclick={closeDrawer}>Login</a></li>
			</ul>
		</nav>
	</div>
</div>

<!-- Main content -->
<div class="main-content" class:drawer-open={isDrawerOpen}>
	{@render children()}
</div>

<style>
	.hamburger-btn {
		position: fixed;
		top: 1rem;
		left: 1rem;
		z-index: 60;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 0.75rem;
		cursor: pointer;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		transition: all 0.3s ease;
	}

	.hamburger-btn:hover {
		background: #f9fafb;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
	}

	.hamburger-icon {
		width: 24px;
		height: 18px;
		position: relative;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
	}

	.hamburger-icon span {
		display: block;
		height: 2px;
		width: 100%;
		background: #374151;
		border-radius: 1px;
		transition: all 0.3s ease;
		transform-origin: center;
	}

	.hamburger-icon.open span:nth-child(1) {
		transform: rotate(45deg) translate(6px, 6px);
	}

	.hamburger-icon.open span:nth-child(2) {
		opacity: 0;
	}

	.hamburger-icon.open span:nth-child(3) {
		transform: rotate(-45deg) translate(6px, -6px);
	}

	.drawer-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 45;
		opacity: 0;
		animation: fadeIn 0.3s ease-in-out forwards;
	}

	@keyframes fadeIn {
		to {
			opacity: 1;
		}
	}

	.drawer {
		position: fixed;
		top: 0;
		left: 0;
		height: 100vh;
		width: 320px;
		background: white;
		box-shadow: 2px 0 15px rgba(0, 0, 0, 0.1);
		transform: translateX(-100%);
		transition: transform 0.3s ease-in-out;
		z-index: 50;
		border-right: 1px solid #e5e7eb;
	}

	.drawer-open {
		transform: translateX(0);
	}

	.drawer-content {
		padding: 2rem 1.5rem;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.drawer-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 2rem;
		cursor: pointer;
		color: #6b7280;
		padding: 0.25rem;
		line-height: 1;
		transition: color 0.2s;
	}

	.close-btn:hover {
		color: #374151;
	}

	.drawer-nav {
		flex: 1;
	}

	.main-content {
		transition: margin-left 0.3s ease-in-out;
		min-height: 100vh;
	}

	/* Dark mode support */
	:global(.dark) .hamburger-btn {
		background: #1f2937;
		border-color: #374151;
	}

	:global(.dark) .hamburger-btn:hover {
		background: #374151;
	}

	:global(.dark) .hamburger-icon span {
		background: #e5e7eb;
	}

	:global(.dark) .drawer {
		background: #1f2937;
		border-right-color: #374151;
	}

	:global(.dark) .drawer-header {
		border-bottom-color: #374151;
	}

	:global(.dark) .close-btn {
		color: #9ca3af;
	}

	:global(.dark) .close-btn:hover {
		color: #e5e7eb;
	}

	.brand-link {
		text-decoration: none;
		color: inherit;
	}

	.nav-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.nav-list li {
		width: 100%;
	}

	.drawer-link {
		width: 100%;
		text-align: left;
		padding: 1rem;
		border-radius: 0.5rem;
		transition: background-color 0.2s;
		display: block;
		text-decoration: none;
		color: #374151;
	}

	.drawer-link:hover {
		background-color: #f3f4f6;
	}

	:global(.dark) .drawer-link {
		color: #e5e7eb;
	}

	:global(.dark) .drawer-link:hover {
		background-color: #374151;
	}

	/* Responsive adjustments */
	@media (max-width: 768px) {
		.drawer {
			width: 280px;
		}
	}

	@media (max-width: 480px) {
		.drawer {
			width: 100vw;
		}
	}
</style>
