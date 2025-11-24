<script lang="ts">
	const features = [
		{
			icon: '🌐',
			title: 'Decentralized Physical Infrastructure (DePIN)',
			description:
				'Access virtual servers, physical hardware, and cloud services from verified providers worldwide. Deploy infrastructure and services where and when you need them, with transparent pricing and reputation metrics.'
		},
		{
			icon: '⭐',
			title: 'Reputation-Based System',
			description:
				'Make informed decisions with our transparent provider reputation system. Every provider is tracked for uptime, performance, and service quality with verifiable on-chain metrics.'
		},
		{
			icon: '🔒',
			title: 'Confidential Computing',
			description:
				'Process sensitive data securely in Confidential Computing VMs with hardware-level encryption. Your workloads remain private and verifiable, even from the infrastructure provider.'
		},
		{
			icon: '🤝',
			title: 'No Vendor Lock-in',
			description:
				'Deploy across multiple providers through a unified API. Built on liberal Open Source principles, ensuring portability and preventing dependency on any single vendor.'
		}
	];
</script>

<section id="features" class="py-20 px-4">
	<div class="max-w-7xl mx-auto">
		<h2 class="text-4xl md:text-5xl font-bold text-center mb-4">Why Decent Cloud?</h2>
		<p class="text-xl text-white/70 text-center mb-16">
			A decentralized marketplace connecting providers with users who need compute, storage, GPUs, and cloud services.
		</p>

		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
			{#each features as feature}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 hover:bg-white/20 transition-all hover:scale-105"
				>
					<div class="text-6xl mb-4">{feature.icon}</div>
					<h3 class="text-xl font-bold mb-3">{feature.title}</h3>
					<p class="text-white/70">{feature.description}</p>
				</div>
			{/each}
		</div>
	</div>
</section>
