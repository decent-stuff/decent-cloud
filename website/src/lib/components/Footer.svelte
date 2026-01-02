<script lang="ts">
	import Icon from './Icons.svelte';
</script>

<footer class="py-16 px-6 border-t border-neutral-800">
	<div class="max-w-6xl mx-auto">
		<div class="grid grid-cols-1 md:grid-cols-4 gap-12 mb-12">
			<!-- Brand -->
			<div class="space-y-4">
				<h3 class="text-xl font-bold text-white">Decent Cloud</h3>
				<p class="text-neutral-500 text-sm leading-relaxed">
					Decentralized cloud computing for everyone.
					Trust scores, escrow protection, and full transparency.
				</p>
			</div>

			<!-- Product -->
			<div>
				<h4 class="text-xs uppercase tracking-widest text-neutral-500 mb-4">Product</h4>
				<ul class="space-y-3">
					<li>
						<a href="/dashboard/marketplace" class="text-neutral-400 hover:text-white transition-colors text-sm">
							Marketplace
						</a>
					</li>
					<li>
						<a href="/dashboard/offerings" class="text-neutral-400 hover:text-white transition-colors text-sm">
							Offerings
						</a>
					</li>
					<li>
						<a href="/dashboard/reputation" class="text-neutral-400 hover:text-white transition-colors text-sm">
							Reputation
						</a>
					</li>
				</ul>
			</div>

			<!-- Resources -->
			<div>
				<h4 class="text-xs uppercase tracking-widest text-neutral-500 mb-4">Resources</h4>
				<ul class="space-y-3">
					<li>
						<a href="/docs/decent-cloud-whitepaper.pdf" class="text-neutral-400 hover:text-white transition-colors text-sm inline-flex items-center gap-2">
							Whitepaper
							<Icon name="external" size={12} class="opacity-50" />
						</a>
					</li>
					<li>
						<a href="https://github.com/decent-stuff/decent-cloud" class="text-neutral-400 hover:text-white transition-colors text-sm inline-flex items-center gap-2">
							GitHub
							<Icon name="external" size={12} class="opacity-50" />
						</a>
					</li>
				</ul>
			</div>

			<!-- Community -->
			<div>
				<h4 class="text-xs uppercase tracking-widest text-neutral-500 mb-4">Community</h4>
				<ul class="space-y-3">
					<li>
						<a href="https://discord.gg/decentcloud" class="text-neutral-400 hover:text-white transition-colors text-sm inline-flex items-center gap-2">
							Discord
							<Icon name="external" size={12} class="opacity-50" />
						</a>
					</li>
					<li>
						<a href="https://twitter.com/decentcloud" class="text-neutral-400 hover:text-white transition-colors text-sm inline-flex items-center gap-2">
							Twitter
							<Icon name="external" size={12} class="opacity-50" />
						</a>
					</li>
				</ul>
			</div>
		</div>

		<!-- Bottom bar -->
		<div class="border-t border-neutral-800 pt-8 text-center">
			<p class="text-neutral-600 text-sm">
				&copy; 2025 Decent Cloud. Liberal Open Source License (Apache 2.0).
			</p>
		</div>
	</div>
</footer>
