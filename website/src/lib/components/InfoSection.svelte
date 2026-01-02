<script lang="ts">
	import Icon from './Icons.svelte';
</script>

<section class="py-28 px-6 bg-surface-elevated/30">
	<div class="max-w-6xl mx-auto">
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
			<!-- Content -->
			<div class="space-y-6">
				<h2 class="section-title">
					Join the Cloud Marketplace
				</h2>
				<p class="text-base text-neutral-400 leading-relaxed">
					Whether you're a provider with spare capacity or services to offer,
					or a user looking for reliable infrastructure and cloud services,
					Decent Cloud connects you with a global network of trusted providers.
				</p>

				<ul class="space-y-3">
					<li class="flex items-center gap-2.5">
						<Icon name="check" size={20} class="text-primary-500 shrink-0" />
						<span class="text-sm text-neutral-300">Direct connections - choose your provider</span>
					</li>
					<li class="flex items-center gap-2.5">
						<Icon name="check" size={20} class="text-primary-500 shrink-0" />
						<span class="text-sm text-neutral-300">Transparent pricing and reputation system</span>
					</li>
					<li class="flex items-center gap-2.5">
						<Icon name="check" size={20} class="text-primary-500 shrink-0" />
						<span class="text-sm text-neutral-300">Community-driven marketplace</span>
					</li>
				</ul>

				<a
					href="/dashboard/marketplace"
					class="inline-flex items-center gap-2 px-5 py-2.5 bg-white text-neutral-900 text-sm font-semibold hover:bg-neutral-100 transition-colors"
				>
					<span>Get Started Now</span>
					<Icon name="arrow-right" size={20} />
				</a>
			</div>

			<!-- Image -->
			<div class="hidden lg:block">
				<div class="relative">
					<img
						src="/images/servers.jpg"
						alt="Server infrastructure"
						class="w-full border border-neutral-800 shadow-layered"
					/>
				</div>
			</div>
		</div>
	</div>
</section>
