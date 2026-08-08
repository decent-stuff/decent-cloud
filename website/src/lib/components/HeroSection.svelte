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
	// Default to the first phrase so SSR / low-JS snapshots show meaningful
	// text instead of an empty span; the typing loop takes over on mount.
	// Declared $state because it is read in markup and mutated by the loop.
	let currentText = $state(phrases[0]);
	let isDeleting = false;
	// Honors prefers-reduced-motion: when set, the type/delete loop is
	// skipped and a single static phrase is shown (no blinking cursor).
	let reducedMotion = $state(false);

	onMount(() => {
		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			reducedMotion = true;
			currentText = phrases[0];
			return;
		}

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

		<!-- Typing animation: a type/delete loop normally, replaced by a static
		     phrase under prefers-reduced-motion (no blinking cursor). -->
		<div class="h-8 flex items-center border-l-2 border-primary-500/60 pl-4">
			<span class="font-mono text-primary-400 text-sm">
				{currentText}{#if !reducedMotion}<span class="text-primary-500 animate-pulse-subtle" aria-hidden="true">_</span>{/if}
			</span>
		</div>

			<!-- CTA -->
			<div class="flex flex-wrap gap-3 pt-2">
				<a
					href="/dashboard/marketplace"
					class="inline-flex items-center gap-2.5 px-5 py-2.5 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
				>
					<span>Open Marketplace</span>
					<Icon name="arrow-right" size={20} />
				</a>
			<a
				href="/dashboard/provider/start"
				class="inline-flex items-center gap-2.5 px-5 py-2.5 border border-neutral-700 text-neutral-300 text-sm font-medium hover:border-neutral-500 hover:text-white hover:bg-surface-hover transition-all"
			>
				<span>Become a Provider</span>
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
			<!-- Educational "anatomy of a trust score" graphic — clearly NOT a
			     live provider. Explains what each metric means without
			     fabricating a provider profile, metrics, or a "Verified" badge
			     on empty data (PRODUCT-DIRECTION: never show fake provider data).
			     TODO: replace with a real top-provider card from the public API
			     once the marketplace has rental activity. -->
			<div class="relative bg-surface border border-neutral-800 p-6 space-y-5 shadow-layered">
				<!-- Header -->
				<div class="flex items-center justify-between pb-4 border-b border-neutral-800/80">
					<div class="flex items-center gap-3">
						<div class="icon-box">
							<Icon name="star" size={20} />
						</div>
						<div class="min-w-0">
							<div class="font-semibold text-white text-sm">Anatomy of a Trust Score</div>
							<div class="mt-0.5">
								<span class="text-[10px] text-neutral-500 uppercase tracking-label">How providers are scored</span>
							</div>
						</div>
					</div>
					<div class="text-right">
						<div class="text-3xl font-bold text-neutral-500 font-mono tracking-tight">0–100</div>
						<div class="text-[10px] text-neutral-500 uppercase tracking-label">Score range</div>
					</div>
				</div>

				<!-- Metrics grid: descriptive labels, not fabricated numbers -->
				<div class="grid grid-cols-2 gap-3">
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="clock" size={20} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Response</span>
						</div>
						<div class="text-xs text-neutral-400 leading-snug">Median time to accept and provision a rental.</div>
					</div>
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="check" size={20} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Completion</span>
						</div>
						<div class="text-xs text-neutral-400 leading-snug">Share of contracts delivered, not cancelled.</div>
					</div>
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="users" size={20} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Repeat</span>
						</div>
						<div class="text-xs text-neutral-400 leading-snug">How often renters return to the same provider.</div>
					</div>
					<div class="bg-surface-elevated p-4 border border-neutral-800">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="file" size={20} class="text-neutral-600" />
							<span class="text-[10px] text-neutral-500 uppercase tracking-label">Volume</span>
						</div>
						<div class="text-xs text-neutral-400 leading-snug">Total completed rentals backing the score.</div>
					</div>
				</div>

				<!-- Status -->
				<div class="flex items-center gap-2 pt-4 border-t border-neutral-800/80">
					<span class="w-2 h-2 bg-neutral-600 shrink-0"></span>
					<span class="text-xs text-neutral-400">Scores appear once a provider has real rental activity.</span>
				</div>
			</div>
		</div>
	</div>

	<!-- Scroll indicator -->
	<div class="absolute bottom-8 left-1/2 -translate-x-1/2">
		<a href="#features" class="flex flex-col items-center gap-1.5 text-neutral-600 hover:text-neutral-400 transition-colors group">
			<span class="text-[10px] uppercase tracking-[0.2em]">Scroll</span>
			<Icon name="arrow-down" size={20} class="group-hover:translate-y-0.5 transition-transform" />
		</a>
	</div>
</section>
