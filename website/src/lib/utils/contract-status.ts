export interface ContractStatusBadge {
	text: string;
	class: string;
	icon: string;
}

const STATUS_BADGES: Record<string, ContractStatusBadge> = {
	// Payment-aware statuses (for 'requested' + payment_status combinations)
	'awaiting_payment': {
		text: 'Awaiting Payment',
		class: 'bg-orange-500/20 text-orange-400 border-orange-500/30',
		icon: '💳'
	},
	'payment_failed': {
		text: 'Payment Failed',
		class: 'bg-red-500/20 text-red-400 border-red-500/30',
		icon: '❌'
	},
	// Standard contract statuses
	requested: {
		text: 'Pending Provider',
		class: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
		icon: '⏳'
	},
	pending: {
		text: 'Pending',
		class: 'bg-primary-500/20 text-primary-400 border-primary-500/30',
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

/**
 * Get display status badge based on contract status and payment status.
 *
 * State machine for display:
 * - status='requested' + payment_status='pending' → 'Awaiting Payment' (Stripe not paid yet)
 * - status='requested' + payment_status='failed' → 'Payment Failed'
 * - status='requested' + payment_status='succeeded' → 'Requested' (paid, waiting for provider)
 * - Other statuses → use status directly
 */
export function getContractStatusBadge(status: string, paymentStatus?: string): ContractStatusBadge {
	const normalizedStatus = status?.toLowerCase() ?? '';
	const normalizedPaymentStatus = paymentStatus?.toLowerCase() ?? '';

	// Handle 'requested' status with payment_status awareness
	if (normalizedStatus === 'requested') {
		if (normalizedPaymentStatus === 'pending') {
			return STATUS_BADGES['awaiting_payment'];
		}
		if (normalizedPaymentStatus === 'failed') {
			return STATUS_BADGES['payment_failed'];
		}
		// payment_status='succeeded' falls through to show 'Requested'
	}

	return STATUS_BADGES[normalizedStatus] ?? { ...DEFAULT_BADGE, text: status ?? DEFAULT_BADGE.text };
}
