export interface ContractStatusBadge {
	text: string;
	class: string;
	icon: string;
}

const STATUS_BADGES: Record<string, ContractStatusBadge> = {
	requested: {
		text: 'Requested',
		class: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
		icon: '🟡'
	},
	pending: {
		text: 'Pending',
		class: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
		icon: '🔵'
	},
	accepted: {
		text: 'Accepted',
		class: 'bg-green-500/20 text-green-400 border-green-500/30',
		icon: '🟢'
	},
	provisioning: {
		text: 'Provisioning',
		class: 'bg-purple-500/20 text-purple-400 border-purple-500/30',
		icon: '⚙️'
	},
	provisioned: {
		text: 'Provisioned',
		class: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30',
		icon: '✅'
	},
	active: {
		text: 'Active',
		class: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30',
		icon: '✅'
	},
	rejected: {
		text: 'Rejected',
		class: 'bg-red-500/20 text-red-400 border-red-500/30',
		icon: '🔴'
	},
	cancelled: {
		text: 'Cancelled',
		class: 'bg-gray-500/20 text-gray-400 border-gray-500/30',
		icon: '⚫'
	}
};

const DEFAULT_BADGE: ContractStatusBadge = {
	text: 'Unknown',
	class: 'bg-gray-500/20 text-gray-300 border-gray-500/30',
	icon: '⚪'
};

export function getContractStatusBadge(status: string): ContractStatusBadge {
	const normalized = status?.toLowerCase() ?? '';
	return STATUS_BADGES[normalized] ?? { ...DEFAULT_BADGE, text: status ?? DEFAULT_BADGE.text };
}
