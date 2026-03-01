<script lang="ts">
	import { page } from "$app/stores";

	const tabs = [
		{ href: "/dashboard/account", label: "Overview", icon: "⚙️" },
		{ href: "/dashboard/account/profile", label: "Profile", icon: "👤" },
		{ href: "/dashboard/account/security", label: "Security", icon: "🔐" },
		{ href: "/dashboard/account/subscription", label: "Subscription", icon: "⭐" },
		{ href: "/dashboard/account/billing", label: "Billing", icon: "💳" },
		{ href: "/dashboard/account/notifications", label: "Notifications", icon: "🔔" },
	];

	let currentPath = $derived($page.url.pathname);
</script>

<nav class="flex flex-wrap gap-1 mb-6 border-b border-neutral-800 pb-4">
	{#each tabs as tab}
		{@const isActive = currentPath === tab.href}
		<a
			href={tab.href}
			class="flex items-center gap-2 px-4 py-2 text-sm font-medium transition-all {isActive
				? 'bg-primary-500/20 text-primary-300 border-b-2 border-primary-500'
				: 'text-neutral-400 hover:text-white hover:bg-neutral-800'}"
		>
			<span>{tab.icon}</span>
			<span>{tab.label}</span>
		</a>
	{/each}
</nav>
