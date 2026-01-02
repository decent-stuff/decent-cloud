<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from './Icons.svelte';

	const phrases = [
		'Transparent Trust Scores',
		'Escrow-Protected Payments',
		'Real-Time Red Flag Detection',
		'Verified Provider Track Records'
	];

	let currentIndex = 0;
	let currentText = '';
	let isDeleting = false;

	onMount(() => {
		const typeSpeed = 50;
		const deleteSpeed = 30;
		const pauseTime = 1500;

		function tick() {
			const phrase = phrases[currentIndex];

			if (isDeleting) {
				currentText = phrase.substring(0, currentText.length - 1);
			} else {
				currentText = phrase.substring(0, currentText.length + 1);
			}

			let delay = isDeleting ? deleteSpeed : typeSpeed;

			if (!isDeleting && currentText === phrase) {
				delay = pauseTime;
				isDeleting = true;
			} else if (isDeleting && currentText === '') {
				isDeleting = false;
				currentIndex = (currentIndex + 1) % phrases.length;
			}

			setTimeout(tick, delay);
		}

		tick();
	});
</script>

<section class="min-h-screen flex items-center justify-center px-6 bg-grid relative">
	<div class="max-w-6xl mx-auto grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
		<!-- Content -->
		<div class="space-y-8">
			<!-- Badge -->
			<div class="inline-flex items-center gap-2 px-3 py-1 bg-surface-elevated border border-neutral-700 text-neutral-300 text-sm">
				<span class="w-1.5 h-1.5 bg-primary-400 animate-pulse-subtle"></span>
				<span>Decentralized Cloud Marketplace</span>
			</div>

			<!-- Headline -->
			<h1 class="text-4xl sm:text-5xl lg:text-6xl font-bold leading-[1.1] tracking-tight">
				<span class="text-white">Rent Cloud Resources</span>
				<br />
				<span class="text-gradient">With Confidence</span>
			</h1>

			<!-- Description -->
			<p class="text-lg text-neutral-400 max-w-xl leading-relaxed">
				Every provider has a trust score based on real performance data.
				See response times, completion rates, and red flags
				<span class="text-white font-medium">before</span> you pay.
			</p>

			<!-- Typing animation -->
			<div class="h-8 flex items-center border-l-2 border-primary-500 pl-4">
				<span class="font-mono text-primary-400 text-sm tracking-wide">
					{currentText}<span class="animate-pulse-subtle">_</span>
				</span>
			</div>

			<!-- CTA -->
			<div class="flex flex-wrap gap-4 pt-4">
				<a
					href="/dashboard/marketplace"
					class="inline-flex items-center gap-3 px-6 py-3 bg-primary-500 text-base font-semibold hover:bg-primary-400 transition-colors"
				>
					<span>Open Marketplace</span>
					<Icon name="arrow-right" size={18} />
				</a>
				<a
					href="#features"
					class="inline-flex items-center gap-3 px-6 py-3 border border-neutral-600 text-neutral-200 font-medium hover:border-neutral-400 hover:text-white transition-colors"
				>
					<span>Learn More</span>
				</a>
			</div>

			<!-- Stats row -->
			<div class="flex gap-8 pt-6 border-t border-neutral-800">
				<div>
					<div class="text-2xl font-semibold text-white font-mono tabular-nums">100%</div>
					<div class="text-xs uppercase tracking-wider text-neutral-500">Transparent</div>
				</div>
				<div>
					<div class="text-2xl font-semibold text-white font-mono tabular-nums">0</div>
					<div class="text-xs uppercase tracking-wider text-neutral-500">Hidden Fees</div>
				</div>
				<div>
					<div class="text-2xl font-semibold text-white font-mono tabular-nums">Real</div>
					<div class="text-xs uppercase tracking-wider text-neutral-500">Data Only</div>
				</div>
			</div>
		</div>

		<!-- Visual -->
		<div class="hidden lg:block relative">
			<div class="relative">
				<!-- Grid decoration -->
				<div class="absolute inset-0 bg-gradient-to-br from-primary-500/5 to-transparent"></div>

				<!-- Main visual: Trust score card mockup -->
				<div class="relative bg-surface border border-neutral-800 p-6 space-y-6">
					<!-- Header -->
					<div class="flex items-center justify-between pb-4 border-b border-neutral-800">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 bg-surface-elevated border border-neutral-700 flex items-center justify-center">
								<Icon name="server" size={20} class="text-primary-400" />
							</div>
							<div>
								<div class="font-semibold text-white">provider_alpha</div>
								<div class="text-xs text-neutral-500">Verified Provider</div>
							</div>
						</div>
						<div class="text-right">
							<div class="text-2xl font-bold text-primary-400 font-mono">87</div>
							<div class="text-xs text-neutral-500">Trust Score</div>
						</div>
					</div>

					<!-- Metrics grid -->
					<div class="grid grid-cols-2 gap-4">
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<div class="flex items-center gap-2 mb-2">
								<Icon name="clock" size={14} class="text-neutral-500" />
								<span class="text-xs text-neutral-500 uppercase tracking-wider">Response</span>
							</div>
							<div class="text-lg font-semibold text-white font-mono">2.3h</div>
						</div>
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<div class="flex items-center gap-2 mb-2">
								<Icon name="check" size={14} class="text-neutral-500" />
								<span class="text-xs text-neutral-500 uppercase tracking-wider">Completion</span>
							</div>
							<div class="text-lg font-semibold text-white font-mono">98.2%</div>
						</div>
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<div class="flex items-center gap-2 mb-2">
								<Icon name="users" size={14} class="text-neutral-500" />
								<span class="text-xs text-neutral-500 uppercase tracking-wider">Repeat</span>
							</div>
							<div class="text-lg font-semibold text-white font-mono">73%</div>
						</div>
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<div class="flex items-center gap-2 mb-2">
								<Icon name="file" size={14} class="text-neutral-500" />
								<span class="text-xs text-neutral-500 uppercase tracking-wider">Contracts</span>
							</div>
							<div class="text-lg font-semibold text-white font-mono">1,247</div>
						</div>
					</div>

					<!-- Status -->
					<div class="flex items-center justify-between pt-4 border-t border-neutral-800">
						<div class="flex items-center gap-2">
							<span class="w-2 h-2 bg-success rounded-full"></span>
							<span class="text-sm text-neutral-400">No red flags detected</span>
						</div>
						<span class="text-xs text-neutral-600">Updated 2m ago</span>
					</div>
				</div>
			</div>
		</div>
	</div>

	<!-- Scroll indicator -->
	<div class="absolute bottom-8 left-1/2 -translate-x-1/2">
		<a href="#features" class="flex flex-col items-center gap-2 text-neutral-600 hover:text-neutral-400 transition-colors">
			<span class="text-xs uppercase tracking-widest">Scroll</span>
			<Icon name="arrow-down" size={16} />
		</a>
	</div>
</section>
