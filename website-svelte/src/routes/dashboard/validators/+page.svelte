<script lang="ts">
	// TODO: Integrate with actual validator API when available
	interface Validator {
		id: string;
		name: string;
		status: 'active' | 'inactive';
		blocksValidated: number;
		uptime: number;
		rewards: number;
	}

	const mockValidators: Validator[] = [
		{
			id: '1',
			name: 'Validator Alpha',
			status: 'active',
			blocksValidated: 12543,
			uptime: 99.9,
			rewards: 1250.5
		},
		{
			id: '2',
			name: 'Validator Beta',
			status: 'active',
			blocksValidated: 11234,
			uptime: 98.7,
			rewards: 1100.25
		},
		{
			id: '3',
			name: 'Validator Gamma',
			status: 'inactive',
			blocksValidated: 8765,
			uptime: 95.3,
			rewards: 876.75
		}
	];
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Validators</h1>
		<p class="text-white/60">Network validators and their performance</p>
	</div>

	<!-- Stats Summary -->
	<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Active Validators</h3>
				<span class="text-2xl">✓</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{mockValidators.filter((v) => v.status === 'active').length}
			</p>
		</div>

		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Total Blocks</h3>
				<span class="text-2xl">🔗</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{mockValidators.reduce((sum, v) => sum + v.blocksValidated, 0).toLocaleString()}
			</p>
		</div>

		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Total Rewards</h3>
				<span class="text-2xl">💰</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{mockValidators.reduce((sum, v) => sum + v.rewards, 0).toFixed(2)} DCT
			</p>
		</div>
	</div>

	<!-- Validators Table -->
	<div class="bg-white/10 backdrop-blur-lg rounded-xl border border-white/20 overflow-hidden">
		<div class="overflow-x-auto">
			<table class="w-full">
				<thead class="bg-white/5 border-b border-white/10">
					<tr>
						<th class="px-6 py-4 text-left text-sm font-semibold text-white">Validator</th>
						<th class="px-6 py-4 text-left text-sm font-semibold text-white">Status</th>
						<th class="px-6 py-4 text-right text-sm font-semibold text-white">Blocks</th>
						<th class="px-6 py-4 text-right text-sm font-semibold text-white">Uptime</th>
						<th class="px-6 py-4 text-right text-sm font-semibold text-white">Rewards</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-white/10">
					{#each mockValidators as validator}
						<tr class="hover:bg-white/5 transition-colors">
							<td class="px-6 py-4">
								<div class="flex items-center gap-3">
									<div
										class="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-bold"
									>
										{validator.name.charAt(0)}
									</div>
									<div>
										<p class="text-white font-medium">{validator.name}</p>
										<p class="text-white/50 text-sm">{validator.id}</p>
									</div>
								</div>
							</td>
							<td class="px-6 py-4">
								<span
									class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm font-medium {validator.status ===
									'active'
										? 'bg-green-500/20 text-green-400 border border-green-500/30'
										: 'bg-gray-500/20 text-gray-400 border border-gray-500/30'}"
								>
									<span class="w-2 h-2 rounded-full bg-current"></span>
									{validator.status}
								</span>
							</td>
							<td class="px-6 py-4 text-right text-white">
								{validator.blocksValidated.toLocaleString()}
							</td>
							<td class="px-6 py-4 text-right text-white">{validator.uptime}%</td>
							<td class="px-6 py-4 text-right text-white">{validator.rewards.toFixed(2)} DCT</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>

	<!-- Become a Validator CTA -->
	<div
		class="bg-gradient-to-r from-blue-500/20 to-purple-600/20 rounded-xl p-8 border border-blue-500/30"
	>
		<h2 class="text-2xl font-bold text-white mb-2">Become a Validator</h2>
		<p class="text-white/70 mb-4">
			Help secure the network and earn rewards by becoming a validator
		</p>
		<button
			class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all"
		>
			Learn More →
		</button>
	</div>
</div>
