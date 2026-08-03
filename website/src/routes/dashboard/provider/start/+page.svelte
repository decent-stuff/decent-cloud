<script lang="ts">
	// "Become a Provider" technical-onboarding hub.
	//
	// PRODUCT-DIRECTION.md: decent-cloud is "OpenRouter, but for cloud resources"
	// — a proxy/reselling platform unifying many providers behind one common API.
	// "Become a Provider must mean real onboarding to the technical path … not
	// just a support-profile wizard." There are TWO honest technical paths:
	//
	//   A) Resell a managed cloud (Hetzner/Vultr): the provider adds a cloud API
	//      token; the central api provisions directly. No infra to run, no
	//      dc-agent. (Hetzner/Vultr are central-API CloudBackends at
	//      api/src/cloud/{hetzner,vultr}.rs; dc-agent has no such provisioner,
	//      and cloud VMs get public IPs with no gateway —
	//      api/src/cloud_provisioning_service.rs:289-312.)
	//   B) List your own infrastructure: install dc-agent on Proxmox/Docker/DO,
	//      register a pool, then create offerings. (dc-agent/src/provisioner/.)
	//
	// Both paths converge on step 2 (create an offering). The landing "Become a
	// Provider" CTA used to land on a support-profile wizard; this page is the
	// honest start.
	import Icon, { type IconName } from '$lib/components/Icons.svelte';

	const DC_AGENT_DOCS_URL =
		'https://github.com/decent-stuff/decent-cloud/blob/main/docs/provider-agent-installation.md';

	interface ProviderPath {
		icon: IconName;
		eyebrow: string;
		title: string;
		description: string;
		href: string;
		cta: string;
		external?: boolean;
		testid: string;
	}

	const paths: ProviderPath[] = [
		{
			icon: 'cloud',
			eyebrow: 'Path A',
			title: 'Resell a managed cloud',
			description:
				'Resell Hetzner or Vultr capacity with zero infrastructure to run. Add a cloud account (API token), then create offerings from the live catalog. The central API provisions VMs directly and gives them public IPs — no dc-agent, no gateway, no pool.',
			href: '/dashboard/cloud/accounts',
			cta: 'Add a cloud account',
			testid: 'provider-start-cloud-accounts-link'
		},
		{
			icon: 'server',
			eyebrow: 'Path B',
			title: 'List your own infrastructure',
			description:
				'Install dc-agent on your own Proxmox, Docker, or DigitalOcean host. It registers your pool, provisions VMs, and runs the gateway so your capacity is reachable behind one common API.',
			href: DC_AGENT_DOCS_URL,
			cta: 'Read the installation guide',
			external: true,
			testid: 'provider-start-install-docs-link'
		}
	];

	interface OnboardingStep {
		number: number;
		icon: IconName;
		title: string;
		description: string;
		href: string;
		cta: string;
	}

	const steps: OnboardingStep[] = [
		{
			number: 2,
			icon: 'cart',
			title: 'List your first offering',
			description:
				'Once your capacity is reachable (cloud account connected or dc-agent pool registered), create a real offering. Set specs, pricing (Stripe-supported currency), and stock. Your offering appears in the marketplace catalog.',
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
				Provide capacity on Decent Cloud
			</h1>
			<p class="text-neutral-400 text-base md:text-lg leading-relaxed max-w-2xl">
				Decent Cloud is a proxy/reselling platform: many providers, one common API. Choose how
				you want to provide capacity, then list real offerings anyone can rent.
			</p>
		</div>

		<!-- Step 1: choose your path -->
		<div class="mb-10">
			<div class="flex items-center gap-3 mb-4">
				<div
					class="flex items-center justify-center w-9 h-9 rounded-full bg-primary-500 text-neutral-900 font-bold text-base font-mono"
				>
					1
				</div>
				<h2 class="text-xl md:text-2xl font-semibold text-white">Choose how you'll provide capacity</h2>
			</div>

			<div class="grid md:grid-cols-2 gap-5">
				{#each paths as path (path.testid)}
					<div class="card p-6 md:p-7 flex flex-col" data-testid="provider-start-path">
						<div class="flex items-center justify-between mb-4">
							<div class="text-primary-400">
								<Icon name={path.icon} size={26} />
							</div>
							<span class="text-xs font-mono uppercase tracking-wider text-neutral-500">
								{path.eyebrow}
							</span>
						</div>
						<h3 class="text-lg font-semibold text-white mb-2">{path.title}</h3>
						<p class="text-neutral-400 text-sm leading-relaxed mb-5 flex-1">{path.description}</p>
						<a
							href={path.href}
							{...path.external ? { target: '_blank', rel: 'noopener noreferrer' } : {}}
							data-testid={path.testid}
							class="inline-flex items-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors self-start"
						>
							<span>{path.cta}</span>
							<Icon name={path.external ? 'external' : 'arrow-right'} size={16} />
						</a>
					</div>
				{/each}
			</div>
		</div>

		<!-- Remaining steps -->
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
							data-testid="provider-start-step-link"
							class="inline-flex items-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
						>
							<span>{step.cta}</span>
							<Icon name="arrow-right" size={16} />
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
