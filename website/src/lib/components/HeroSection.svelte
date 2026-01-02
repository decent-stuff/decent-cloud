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

<section class="min-h-screen flex items-center justify-center px-6 bg-grid bg-radial relative pt-14">
	<div class="max-w-6xl mx-auto grid grid-cols-1 lg:grid-cols-2 gap-20 items-center">
		<!-- Content -->
		<div class="space-y-8">
			<!-- Badge -->
			<div class="inline-flex items-center gap-2.5 px-3 py-1.5 bg-surface border border-neutral-800 text-neutral-400 text-xs tracking-wide">
				<span class="w-1.5 h-1.5 bg-primary-500 animate-pulse-subtle"></span>
				<span class="uppercase tracking-label">Decentralized Cloud</span>
			</div>

			<!-- Headline -->
			<h1 class="text-4xl sm:text-5xl lg:text-[3.5rem] font-bold leading-[1.08] tracking-display">
				<span class="text-white">Rent Cloud Resources</span>
				<br />
				<span class="text-gradient">With Confidence</span>
			</h1>

			<!-- Description -->
			<p class="text-lg text-neutral-400 max-w-lg leading-relaxed">
				Every provider has a trust score based on real performance data.
				See response times, completion rates, and red flags
				<span class="text-white">before</span> you pay.
			</p>

			<!-- Typing animation -->
			<div class="h-8 flex items-center border-l-2 border-primary-500/60 pl-4">
				<span class="font-mono text-primary-400 text-sm">
					{currentText}<span class="text-primary-500 animate-pulse-subtle">_</span>
				</span>
			</div>

			<!-- CTA -->
			<div class="flex flex-wrap gap-3 pt-2">
				<a
					href="/dashboard/marketplace"
					class="inline-flex items-center gap-2.5 px-5 py-2.5 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
				>
					<span>Open Marketplace</span>
					<Icon name="arrow-right" size={16} />
				</a>
				<a
					href="#features"
					class="inline-flex items-center gap-2.5 px-5 py-2.5 border border-neutral-700 text-neutral-300 text-sm font-medium hover:border-neutral-500 hover:text-white hover:bg-surface-hover transition-all"
				>
					<span>Learn More</span>
				</a>
			</div>

			<!-- Stats row -->
			<div class="flex gap-10 pt-8 border-t border-neutral-800/60">
				<div>
					<div class="text-xl font-semibold text-white font-mono tabular-nums">100%</div>
					<div class="text-[10px] uppercase tracking-label text-neutral-500 mt-0.5">Transparent</div>
				</div>
				<div>
					<div class="text-xl font-semibold text-white font-mono tabular-nums">0</div>
					<div class="text-[10px] uppercase tracking-label text-neutral-500 mt-0.5">Hidden Fees</div>
				</div>
				<div>
					<div class="text-xl font-semibold text-white font-mono tabular-nums">Real</div>
					<div class="text-[10px] uppercase tracking-label text-neutral-500 mt-0.5">Data Only</div>
				</div>
			</div>
		</div>

		<!-- Visual -->
		<div class="hidden lg:block relative">
			<!-- Main visual: Trust score card mockup -->
			<div class="relative bg-surface border border-neutral-800 p-6 space-y-5 shadow-layered">
				<!-- Header -->
				<div class="flex items-center justify-between pb-4 border-b border-neutral-800/80">
					<div class="flex items-center gap-3">
						<div class="icon-box">
							<Icon name="server" size={18} />
						</div>
						<div>
							<div class="font-semibold text-white text-sm">provider_alpha</div>
							<div class="text-[10px] text-neutral-500 uppercase tracking-label">Verified Provider</div>
						</div>
					</div>
					<div class="text-right">
						<div class="text-3xl font-bold text-primary-400 font-mono tracking-tight">87</div>
						<div class="text-[10px] text-neutral-500 uppercase tracking-label">Trust Score</div>
					</div>
				</div>

				<!-- Metrics grid -->
				<div class="grid grid-cols-2 gap-3">
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="clock" size={12} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Response</span>
						</div>
						<div class="text-lg font-semibold text-white font-mono">2.3h</div>
					</div>
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="check" size={12} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Completion</span>
						</div>
						<div class="text-lg font-semibold text-white font-mono">98.2%</div>
					</div>
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="users" size={12} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Repeat</span>
						</div>
						<div class="text-lg font-semibold text-white font-mono">73%</div>
					</div>
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="file" size={12} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Contracts</span>
						</div>
						<div class="text-lg font-semibold text-white font-mono">1,247</div>
					</div>
				</div>

				<!-- Status -->
				<div class="flex items-center justify-between pt-4 border-t border-neutral-800/80">
					<div class="flex items-center gap-2">
						<span class="status-dot status-dot-success"></span>
						<span class="text-xs text-neutral-400">No red flags detected</span>
					</div>
					<span class="text-[10px] text-neutral-600 uppercase tracking-label">Updated 2m ago</span>
				</div>
			</div>
		</div>
	</div>

	<!-- Scroll indicator -->
	<div class="absolute bottom-8 left-1/2 -translate-x-1/2">
		<a href="#features" class="flex flex-col items-center gap-1.5 text-neutral-600 hover:text-neutral-400 transition-colors group">
			<span class="text-[10px] uppercase tracking-[0.2em]">Scroll</span>
			<Icon name="arrow-down" size={14} class="group-hover:translate-y-0.5 transition-transform" />
		</a>
	</div>
</section>
