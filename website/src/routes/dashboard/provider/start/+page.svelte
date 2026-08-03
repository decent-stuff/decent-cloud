<script lang="ts">
	// "Become a Provider" technical-onboarding hub.
	//
	// PRODUCT-DIRECTION.md: "Become a Provider must mean real onboarding to the
	// technical path (install the agent, register a pool, list an offering), not
	// just a support-profile wizard." The landing "Become a Provider" CTA used to
	// land on /dashboard/provider/support (a support-profile completeness wizard)
	// which never exposed the real provider-setup path. This page is the honest
	// start: it surfaces the three concrete steps a new provider takes, in order.
	import Icon, { type IconName } from '$lib/components/Icons.svelte';

	const DC_AGENT_DOCS_URL =
		'https://github.com/decent-stuff/decent-cloud/blob/main/docs/provider-agent-installation.md';

	interface OnboardingStep {
		number: number;
		icon: IconName;
		title: string;
		description: string;
		href: string;
		cta: string;
		external?: boolean;
	}

	const steps: OnboardingStep[] = [
		{
			number: 1,
			icon: 'server',
			title: 'Install the provider agent (dc-agent)',
			description:
				'Install dc-agent on your infrastructure (Proxmox, Hetzner, Docker, DigitalOcean). It registers your pool, provisions VMs, and runs the gateway. Follow the installation guide to get a setup token and run the one-command setup.',
			href: DC_AGENT_DOCS_URL,
			cta: 'Read the installation guide',
			external: true
		},
		{
			number: 2,
			icon: 'cart',
			title: 'List your first offering',
			description:
				'Once your agent is online and your pool is registered, create a real offering backed by your capacity. Set specs, pricing (Stripe-supported currency), and stock. Your offering appears in the marketplace catalog.',
			href: '/dashboard/offerings/create',
			cta: 'Create an offering'
		},
		{
			number: 3,
			icon: 'check',
			title: 'Complete your support profile',
			description:
				'Set up your provider identity, support contacts, notification channels, and Help Center so tenants can reach you. This step makes your provider presence trustworthy and reachable.',
			href: '/dashboard/provider/support',
			cta: 'Open the support profile'
		}
	];
</script>

<svelte:head>
	<title>Become a Provider - Decent Cloud</title>
</svelte:head>

<div class="min-h-screen bg-base">
	<div class="max-w-4xl mx-auto px-4 py-12 md:py-16">
		<!-- Header -->
		<div class="mb-10">
			<div class="flex items-center gap-2 text-primary-400 text-sm font-medium mb-3">
				<Icon name="arrow-right" size={16} />
				<span>Become a Provider</span>
			</div>
			<h1 class="text-3xl md:text-4xl font-bold text-white tracking-tight mb-4">
				List your infrastructure on Decent Cloud
			</h1>
			<p class="text-neutral-400 text-base md:text-lg leading-relaxed max-w-2xl">
				Decent Cloud is a proxy/reselling platform: providers install an agent, register a
				pool of capacity, and list real offerings that anyone can rent through one common API.
				Follow these three steps to become a provider.
			</p>
		</div>

		<!-- Steps -->
		<div class="space-y-5">
			{#each steps as step (step.number)}
				<div
					class="card p-6 md:p-8 flex flex-col md:flex-row md:items-start gap-5 md:gap-6"
					data-testid="provider-start-step"
					data-step-number={step.number}
				>
					<!-- Step number + icon -->
					<div class="flex items-center gap-4 md:flex-col md:items-center md:w-32 md:flex-shrink-0">
						<div
							class="flex items-center justify-center w-11 h-11 rounded-full bg-primary-500 text-neutral-900 font-bold text-lg font-mono"
						>
							{step.number}
						</div>
						<div class="text-primary-400 md:mt-3">
							<Icon name={step.icon} size={24} />
						</div>
					</div>

					<!-- Content -->
					<div class="flex-1 min-w-0">
						<h2 class="text-lg md:text-xl font-semibold text-white mb-2">{step.title}</h2>
						<p class="text-neutral-400 text-sm md:text-base leading-relaxed mb-4">
							{step.description}
						</p>
						<a
							href={step.href}
							{...step.external ? { target: '_blank', rel: 'noopener noreferrer' } : {}}
							data-testid={
								step.external ? 'provider-start-install-docs-link' : 'provider-start-step-link'
							}
							class="inline-flex items-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
						>
							<span>{step.cta}</span>
							<Icon name={step.external ? 'external' : 'arrow-right'} size={16} />
						</a>
					</div>
				</div>
			{/each}
		</div>

		<!-- Footer note -->
		<div
			class="mt-10 p-5 rounded-md border border-neutral-800 bg-surface-elevated text-sm text-neutral-400 leading-relaxed"
		>
			<p>
				Already have an agent running? You can manage your
				<a href="/dashboard/offerings" class="text-primary-400 hover:text-primary-300 underline">offerings</a>
				and view your
				<a href="/dashboard/provider/earnings" class="text-primary-400 hover:text-primary-300 underline">earnings</a>
				from the dashboard.
			</p>
		</div>
	</div>
</div>
