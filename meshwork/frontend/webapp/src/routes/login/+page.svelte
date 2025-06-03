<script>
	import { Button, Card, Label, Input, Checkbox, A } from 'flowbite-svelte';

	let isLogin = true;
	let email = '';
	let password = '';
	let confirmPassword = '';
	let loading = false;
	let error = '';

	async function handleSubmit() {
		loading = true;
		error = '';

		if (!isLogin && password !== confirmPassword) {
			error = 'Passwords do not match';
			loading = false;
			return;
		}

		try {
			const response = await fetch('http://localhost:8000/login', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					email,
					password,
					action: isLogin ? 'login' : 'register'
				})
			});

			const data = await response.json();

			if (!response.ok) {
				throw new Error(data.message || 'Authentication failed');
			}

			// Handle success (redirect, store token, etc.)
			console.log('Success:', data);
		} catch (err) {
			error = err.message;
		} finally {
			loading = false;
		}
	}

	function toggleMode() {
		isLogin = !isLogin;
		error = '';
		confirmPassword = '';
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-50">
	<Card class="p-4 sm:p-6 md:p-8">
		<form class="flex flex-col space-y-6" on:submit|preventDefault={handleSubmit}>
			<h3 class="text-xl font-medium text-gray-900 dark:text-white">
				{isLogin ? 'Sign in to our platform' : 'Create your account'}
			</h3>

			{#if error}
				<div
					class="rounded-lg bg-red-50 p-4 text-sm text-red-800 dark:bg-gray-800 dark:text-red-400"
				>
					{error}
				</div>
			{/if}

			<Label class="space-y-2">
				<span>Your email</span>
				<Input
					type="email"
					name="email"
					placeholder="name@company.com"
					bind:value={email}
					required
				/>
			</Label>

			<Label class="space-y-2">
				<span>Your password</span>
				<Input
					type="password"
					name="password"
					placeholder="•••••••••"
					bind:value={password}
					required
				/>
			</Label>

			{#if !isLogin}
				<Label class="space-y-2">
					<span>Confirm password</span>
					<Input
						type="password"
						name="confirmPassword"
						placeholder="•••••••••"
						bind:value={confirmPassword}
						required
					/>
				</Label>
			{/if}

			{#if isLogin}
				<div class="flex items-start">
					<Checkbox>Remember me</Checkbox>
				</div>
			{/if}

			<Button type="submit" class="w-full" disabled={loading}>
				{loading ? 'Please wait...' : isLogin ? 'Login to your account' : 'Create account'}
			</Button>

			<div class="text-sm font-medium text-gray-500 dark:text-gray-300">
				{isLogin ? "Don't have an account?" : 'Already have an account?'}
				<A href="#" onclick={toggleMode} class="text-blue-700 hover:underline dark:text-blue-500">
					{isLogin ? 'Create account' : 'Sign in here'}
				</A>
			</div>
		</form>
	</Card>
</div>
