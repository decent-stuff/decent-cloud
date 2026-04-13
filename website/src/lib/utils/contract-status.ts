export interface ContractStatusBadge {
	text: string;
	class: string;
	icon: string;
}

const STATUS_BADGES: Record<string, ContractStatusBadge> = {
	// Payment-aware statuses (for 'requested' + payment_status combinations)
	'awaiting_payment': {
		text: 'Awaiting Payment',
		class: 'bg-warning/20 text-warning border-warning/30',
		icon: '💳'
	},
	'payment_failed': {
		text: 'Payment Failed',
		class: 'bg-danger/20 text-danger border-danger/30',
		icon: '❌'
	},
	// Standard contract statuses
	requested: {
		text: 'Pending Provider',
		class: 'bg-warning/20 text-warning border-warning/30',
		icon: '⏳'
	},
	pending: {
		text: 'Pending',
		class: 'bg-primary-500/20 text-primary-400 border-primary-500/30',
		icon: '🔵'
	},
	accepted: {
		text: 'Accepted',
		class: 'bg-success/20 text-success border-success/30',
		icon: '🟢'
	},
	provisioning: {
		text: 'Provisioning (5–15 min)',
		class: 'bg-info/20 text-info border-info/30',
		icon: '⚙️'
	},
	provisioned: {
		text: 'Provisioned',
		class: 'bg-success/20 text-success border-success/30',
		icon: '✅'
	},
	active: {
		text: 'Active',
		class: 'bg-success/20 text-success border-success/30',
		icon: '✅'
	},
	rejected: {
		text: 'Rejected',
		class: 'bg-danger/20 text-danger border-danger/30',
		icon: '🔴'
	},
	failed: {
		text: 'Failed',
		class: 'bg-danger/20 text-danger border-danger/30',
		icon: '❗'
	},
	cancelled: {
		text: 'Cancelled',
		class: 'bg-neutral-700/20 text-neutral-400 border-neutral-700/30',
		icon: '⚫'
	}
};

const DEFAULT_BADGE: ContractStatusBadge = {
	text: 'Unknown',
	class: 'bg-neutral-700/20 text-neutral-300 border-neutral-700/30',
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
