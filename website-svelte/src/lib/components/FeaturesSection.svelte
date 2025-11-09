<script lang="ts">
	const features = [
		{
			icon: '🌐',
			title: 'Decentralized Physical Infrastructure (DePIN)',
			description:
				"Access tailored virtual or physical servers from reputable node providers. It's not just a cloud, it's a whole sky full of possibilities!"
		},
		{
			icon: '⭐',
			title: 'Reputation-Based System',
			description:
				"Make informed decisions with our transparent provider reputation system. We put the 'trust' in trustless technology!"
		},
		{
			icon: '🔒',
			title: 'Confidential Computing',
			description:
				"Process sensitive data securely in Confidential Computing VMs. Your secrets are safe with us (even we don't know them)!"
		},
		{
			icon: '🤝',
			title: 'No Vendor Lock-in',
			description:
				"Easy multi-cloud deployments with consistent APIs and liberal Open Source license. Decent Cloud is going nowhere, you're safe with us. You're not just a customer, you're a free spirit!"
		}
	];
</script>

<section id="features" class="py-20 px-4">
	<div class="max-w-7xl mx-auto">
		<h2 class="text-4xl md:text-5xl font-bold text-center mb-4">Why Decent Cloud?</h2>
		<p class="text-xl text-white/70 text-center mb-16">
			We're not just another cloud provider, we're the cloud provider.
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
