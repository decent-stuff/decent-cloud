<script lang="ts">
	import { onMount } from 'svelte';

	const words = [
		'Peer-to-Peer Cloud Marketplace',
		'No Vendor Lock-in',
		'Community-Driven',
		'Liberal Open Source License'
	];

	let currentWordIndex = 0;
	let currentText = '';
	let isDeleting = false;

	onMount(() => {
		const typeSpeed = 50;
		const deleteSpeed = 30;
		const delaySpeed = 1000;

		function type() {
			const currentWord = words[currentWordIndex];

			if (isDeleting) {
				currentText = currentWord.substring(0, currentText.length - 1);
			} else {
				currentText = currentWord.substring(0, currentText.length + 1);
			}

			let timeout = isDeleting ? deleteSpeed : typeSpeed;

			if (!isDeleting && currentText === currentWord) {
				timeout = delaySpeed;
				isDeleting = true;
			} else if (isDeleting && currentText === '') {
				isDeleting = false;
				currentWordIndex = (currentWordIndex + 1) % words.length;
			}

			setTimeout(type, timeout);
		}

		type();
	});
</script>

<section class="min-h-screen flex items-center justify-center text-center px-4">
	<div class="grid grid-cols-1 md:grid-cols-2 gap-2 md:gap-4 xl:gap-8 items-center max-w-7xl">
		<div class="text-center md:text-left">
			<h1
				class="text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl font-extrabold leading-tight animate-fade-in"
			>
				Welcome to <br />
				<span class="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-purple-600">
					Decent Cloud
				</span>
			</h1>

			<p class="text-base md:text-lg xl:text-xl mt-4 text-white/80">Airbnb of cloud services</p>

			<h2 class="text-lg md:text-2xl mt-2 text-white/80 font-bold min-h-[3rem]">
				{currentText}<span class="animate-pulse">|</span>
			</h2>

			<div class="flex flex-col sm:flex-row gap-4 mt-8 justify-center md:justify-start">
				<a
					href="/dashboard"
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:from-blue-600 hover:to-purple-700 transition-all"
				>
					Get Started
				</a>
				<a
					href="#features"
					class="px-8 py-3 border-2 border-white/30 rounded-lg font-semibold hover:bg-white/10 transition-all"
				>
					Learn More
				</a>
			</div>
		</div>

		<div class="hidden md:block">
			<img src="/images/cloud-illustration.png" alt="Cloud Computing" class="w-full max-w-lg mx-auto" />
		</div>
	</div>

	<div class="absolute bottom-8 left-1/2 transform -translate-x-1/2 animate-bounce">
		<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</div>
</section>

<style>
	@keyframes fade-in {
		from {
			opacity: 0;
			transform: translateY(-20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.animate-fade-in {
		animation: fade-in 0.8s ease-out;
	}
</style>
