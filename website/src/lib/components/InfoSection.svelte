<script lang="ts">
	import Icon from './Icons.svelte';
</script>

<section class="py-24 px-6 bg-surface/50">
	<div class="max-w-6xl mx-auto">
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
			<!-- Content -->
			<div class="space-y-8">
				<h2 class="text-3xl md:text-4xl font-bold text-white">
					Join the Cloud Marketplace
				</h2>
				<p class="text-lg text-neutral-400 leading-relaxed">
					Whether you're a provider with spare capacity or services to offer,
					or a user looking for reliable infrastructure and cloud services,
					Decent Cloud connects you with a global network of trusted providers.
				</p>

				<ul class="space-y-4">
					<li class="flex items-center gap-3">
						<Icon name="check" size={18} class="text-primary-400 shrink-0" />
						<span class="text-neutral-300">Direct connections - choose your provider</span>
					</li>
					<li class="flex items-center gap-3">
						<Icon name="check" size={18} class="text-primary-400 shrink-0" />
						<span class="text-neutral-300">Transparent pricing and reputation system</span>
					</li>
					<li class="flex items-center gap-3">
						<Icon name="check" size={18} class="text-primary-400 shrink-0" />
						<span class="text-neutral-300">Community-driven marketplace</span>
					</li>
				</ul>

				<a
					href="/dashboard/marketplace"
					class="inline-flex items-center gap-3 px-6 py-3 bg-white text-base font-semibold hover:bg-neutral-100 transition-colors"
				>
					<span>Get Started Now</span>
					<Icon name="arrow-right" size={18} />
				</a>
			</div>

			<!-- Image -->
			<div class="hidden lg:block">
				<div class="relative">
					<div class="absolute inset-0 bg-gradient-to-br from-primary-500/10 to-transparent"></div>
					<img
						src="/images/servers.jpg"
						alt="Server infrastructure"
						class="w-full border border-neutral-800"
					/>
				</div>
			</div>
		</div>
	</div>
</section>
