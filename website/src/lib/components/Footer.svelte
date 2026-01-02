<script lang="ts">
	import Icon from './Icons.svelte';
</script>

<footer class="py-14 px-6 border-t border-neutral-800/80">
	<div class="max-w-6xl mx-auto">
		<div class="grid grid-cols-2 md:grid-cols-4 gap-10 mb-10">
			<!-- Brand -->
			<div class="col-span-2 md:col-span-1 space-y-3">
				<h3 class="text-base font-bold text-white tracking-tight">Decent Cloud</h3>
				<p class="text-neutral-500 text-xs leading-relaxed">
					Decentralized cloud computing for everyone.
					Trust scores, escrow protection, and full transparency.
				</p>
			</div>

			<!-- Product -->
			<div>
				<h4 class="section-label mb-4">Product</h4>
				<ul class="space-y-2.5">
					<li>
						<a href="/dashboard/marketplace" class="text-neutral-400 hover:text-white transition-colors text-xs">
							Marketplace
						</a>
					</li>
					<li>
						<a href="/dashboard/offerings" class="text-neutral-400 hover:text-white transition-colors text-xs">
							Offerings
						</a>
					</li>
					<li>
						<a href="/dashboard/reputation" class="text-neutral-400 hover:text-white transition-colors text-xs">
							Reputation
						</a>
					</li>
				</ul>
			</div>

			<!-- Resources -->
			<div>
				<h4 class="section-label mb-4">Resources</h4>
				<ul class="space-y-2.5">
					<li>
						<a href="/docs/decent-cloud-whitepaper.pdf" class="text-neutral-400 hover:text-white transition-colors text-xs inline-flex items-center gap-1.5">
							Whitepaper
							<Icon name="external" size={20} class="text-neutral-600" />
						</a>
					</li>
					<li>
						<a href="https://github.com/decent-stuff/decent-cloud" class="text-neutral-400 hover:text-white transition-colors text-xs inline-flex items-center gap-1.5">
							GitHub
							<Icon name="external" size={20} class="text-neutral-600" />
						</a>
					</li>
				</ul>
			</div>

			<!-- Community -->
			<div>
				<h4 class="section-label mb-4">Community</h4>
				<ul class="space-y-2.5">
					<li>
						<a href="https://discord.gg/decentcloud" class="text-neutral-400 hover:text-white transition-colors text-xs inline-flex items-center gap-1.5">
							Discord
							<Icon name="external" size={20} class="text-neutral-600" />
						</a>
					</li>
					<li>
						<a href="https://twitter.com/decentcloud" class="text-neutral-400 hover:text-white transition-colors text-xs inline-flex items-center gap-1.5">
							Twitter
							<Icon name="external" size={20} class="text-neutral-600" />
						</a>
					</li>
				</ul>
			</div>
		</div>

		<!-- Bottom bar -->
		<div class="border-t border-neutral-800/80 pt-6 text-center">
			<p class="text-neutral-600 text-[10px] uppercase tracking-label">
				&copy; 2025 Decent Cloud &middot; Apache 2.0 License
			</p>
		</div>
	</div>
</footer>
