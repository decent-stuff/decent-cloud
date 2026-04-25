<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import Icon from '$lib/components/Icons.svelte';
	import PricingCard from '$lib/components/agents/PricingCard.svelte';
	import { API_BASE_URL } from '$lib/services/api';

	let email = $state('');
	let github = $state('');
	let submitting = $state(false);
	let status = $state<'idle' | 'ok' | 'error'>('idle');
	let errorMessage = $state('');

	async function submitWaitlist(event: SubmitEvent) {
		event.preventDefault();
		if (submitting) return;
		submitting = true;
		status = 'idle';
		errorMessage = '';

		try {
			const response = await fetch(`${API_BASE_URL}/api/v1/agents-waitlist`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email, github_handle: github })
			});
			if (!response.ok) {
				throw new Error(`Waitlist signup failed: ${response.status} ${response.statusText}`);
			}
			status = 'ok';
			email = '';
			github = '';
		} catch (err) {
			status = 'error';
			errorMessage = err instanceof Error ? err.message : String(err);
		} finally {
			submitting = false;
		}
	}

	const steps = [
		{
			icon: 'link',
			title: 'Connect your repo',
			body: 'Install the Decent Agents GitHub App on a repo. The agent gets read access to issues and PRs, write access only to its own branches.'
		},
		{
			icon: 'inbox',
			title: 'File an issue',
			body: 'Tag any issue with ready-for-agent. Your agent picks it up, plans the work, asks clarifying questions in the issue thread if needed.'
		},
		{
			icon: 'check',
			title: 'Get a PR',
			body: 'The agent opens a pull request against your default branch with the change, tests, and a summary. You review like any human contributor.'
		}
	] as const;

	const faqs = [
		{
			q: 'How does it actually work?',
			a: 'Each subscription gets a dedicated long-lived container running a coding agent. It watches your repo via webhooks, picks up issues you label ready-for-agent, opens PRs, and responds to review comments. You stay in full control: nothing merges without your approval.'
		},
		{
			q: 'Can I cancel any time?',
			a: 'Yes. Cancel from your dashboard at any moment. Unused days in the current month are refunded prorated to your original payment method within 5 business days.'
		},
		{
			q: 'Which model does the agent use?',
			a: 'Claude Sonnet by default, with automatic fallback to other models if Anthropic is rate-limiting or down. Token usage is metered against your monthly cap.'
		},
		{
			q: 'Where is it hosted?',
			a: 'Hetzner data centers in Falkenstein and Helsinki, EU. No US-based runtime. Your code stays inside the agent container; we do not retain it after each session and we do not train models on it.'
		},
		{
			q: 'What about my proprietary code?',
			a: 'The agent runs in an isolated per-customer container. We do not share code across tenants and we do not feed it back to model providers for training. Anthropic API calls follow their zero-retention policy for API customers.'
		},
		{
			q: 'What if I hit the cap?',
			a: 'You get a notification when you cross 80% of either the hours or token cap. Past the cap, the agent pauses or charges overage at 1.5x Anthropic published token rates - your choice, set in your dashboard.'
		}
	] as const;
</script>

<svelte:head>
	<title>Decent Agents - Hosted AI agents for your GitHub repo</title>
	<meta
		name="description"
		content="Rent an AI engineer for your GitHub repo. CHF 49/month. EU-hosted. Cancel any time."
	/>
	<meta property="og:title" content="Decent Agents - Hosted AI agents for your GitHub repo" />
	<meta
		property="og:description"
		content="Hosted AI agents that work your GitHub backlog 24/7. EU-hosted. CHF 49/month."
	/>
</svelte:head>

<div class="min-h-screen bg-base text-white">
	<Header />

	<!-- Hero -->
	<section class="min-h-[80vh] flex items-center justify-center px-6 bg-grid bg-radial pt-20 pb-16">
		<div class="max-w-4xl mx-auto text-center space-y-8">
			<!-- EU trust badge -->
			<div
				class="inline-flex items-center gap-2.5 px-3 py-1.5 bg-surface border border-neutral-800 text-neutral-400 text-xs tracking-wide"
			>
				<span class="w-1.5 h-1.5 bg-primary-500 animate-pulse-subtle"></span>
				<span class="uppercase tracking-label">EU-hosted on Hetzner</span>
			</div>

			<h1 class="text-4xl sm:text-5xl lg:text-6xl font-bold leading-[1.05] tracking-display">
				<span class="text-white">Rent an AI engineer for your</span>
				<span class="text-gradient">GitHub repo</span>
			</h1>

			<p class="text-lg text-neutral-400 max-w-2xl mx-auto leading-relaxed">
				Hosted AI agents that work your GitHub backlog 24/7. File an issue, get a pull request.
				CHF 49 per month. Cancel any time.
			</p>

			<!-- Waitlist form -->
			<form
				id="waitlist"
				class="max-w-xl mx-auto bg-surface border border-neutral-800 p-6 space-y-4 text-left"
				onsubmit={submitWaitlist}
			>
				<div class="space-y-2">
					<label for="agents-email" class="section-label block">Work email</label>
					<input
						id="agents-email"
						type="email"
						required
						bind:value={email}
						placeholder="you@company.com"
						class="input w-full"
						autocomplete="email"
					/>
				</div>
				<div class="space-y-2">
					<label for="agents-github" class="section-label block">GitHub handle</label>
					<input
						id="agents-github"
						type="text"
						required
						bind:value={github}
						placeholder="octocat"
						class="input w-full"
						autocomplete="username"
					/>
				</div>
				<button
					type="submit"
					disabled={submitting}
					class="btn-primary w-full inline-flex items-center justify-center gap-2 text-sm disabled:opacity-60 disabled:cursor-not-allowed"
				>
					<span>{submitting ? 'Submitting...' : 'Start beta'}</span>
					{#if !submitting}
						<Icon name="arrow-right" size={20} />
					{/if}
				</button>

				{#if status === 'ok'}
					<p class="text-success text-sm flex items-center gap-2">
						<Icon name="check" size={20} />
						Thanks - you are on the beta list. We will reach out within a week.
					</p>
				{:else if status === 'error'}
					<p class="text-danger text-sm">
						Could not save signup: {errorMessage}. Email us at agents@decent-cloud.org.
					</p>
				{/if}
			</form>
		</div>
	</section>

	<!-- How it works -->
	<section class="py-20 px-6 border-t border-neutral-800/80">
		<div class="max-w-6xl mx-auto space-y-12">
			<div class="text-center space-y-3">
				<div class="section-label">How it works</div>
				<h2 class="section-title">Three steps. No glue code.</h2>
			</div>
			<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
				{#each steps as step, idx}
					<div class="bg-surface border border-neutral-800 p-6 space-y-4">
						<div class="flex items-center gap-3">
							<div class="icon-box">
								<Icon name={step.icon} size={20} />
							</div>
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">
								Step {idx + 1}
							</span>
						</div>
						<h3 class="text-lg font-semibold text-white">{step.title}</h3>
						<p class="text-sm text-neutral-400 leading-relaxed">{step.body}</p>
					</div>
				{/each}
			</div>
		</div>
	</section>

	<!-- Pricing -->
	<section class="py-20 px-6 border-t border-neutral-800/80 bg-surface/30">
		<div class="max-w-6xl mx-auto space-y-12">
			<div class="text-center space-y-3">
				<div class="section-label">Pricing</div>
				<h2 class="section-title">One tier. No surprises.</h2>
				<p class="section-subtitle mx-auto">
					Built for dev teams of 1-20 engineers and agencies. Need more? Talk to us.
				</p>
			</div>
			<div class="flex justify-center">
				<PricingCard />
			</div>
			<div class="text-center">
				<a
					href="/agents/pricing"
					class="text-sm text-neutral-400 hover:text-white inline-flex items-center gap-1.5 transition-colors"
				>
					<span>See pricing details and usage assumptions</span>
					<Icon name="arrow-right" size={20} />
				</a>
			</div>
		</div>
	</section>

	<!-- FAQ -->
	<section class="py-20 px-6 border-t border-neutral-800/80">
		<div class="max-w-3xl mx-auto space-y-10">
			<div class="text-center space-y-3">
				<div class="section-label">FAQ</div>
				<h2 class="section-title">Common questions</h2>
			</div>
			<div class="space-y-4">
				{#each faqs as faq}
					<details class="bg-surface border border-neutral-800 p-5 group">
						<summary
							class="cursor-pointer flex items-center justify-between text-white font-semibold list-none"
						>
							<span>{faq.q}</span>
							<span
								class="text-neutral-500 group-open:rotate-180 transition-transform shrink-0 ml-4"
							>
								<Icon name="chevron-down" size={20} />
							</span>
						</summary>
						<p class="text-sm text-neutral-400 leading-relaxed mt-4">{faq.a}</p>
					</details>
				{/each}
			</div>
		</div>
	</section>

	<!-- Footer link to existing site -->
	<section class="py-10 px-6 border-t border-neutral-800/80 text-center">
		<a
			href="/"
			class="text-sm text-neutral-400 hover:text-white inline-flex items-center gap-1.5 transition-colors"
		>
			<span>Looking for the Decent Cloud marketplace?</span>
			<Icon name="arrow-right" size={20} />
		</a>
	</section>

	<Footer />
</div>
