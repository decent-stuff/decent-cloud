const EVENT_TYPE_LABELS: Record<string, string> = {
	status_change: 'Status Changed',
	password_reset: 'Password Reset',
	extension: 'Contract Extended',
	payment_confirmed: 'Payment Confirmed',
	provisioned: 'Provisioned'
};

export function formatEventType(eventType: string): string {
	const known = EVENT_TYPE_LABELS[eventType];
	if (known) return known;
	return eventType
		.split('_')
		.map((w) => w.charAt(0).toUpperCase() + w.slice(1))
		.join(' ');
}

const EVENT_TYPE_ICONS: Record<string, string> = {
	status_change: 'refresh',
	password_reset: 'key',
	extension: 'clock',
	payment_confirmed: 'check',
	provisioned: 'server'
};

export function getEventIcon(eventType: string): string {
	return EVENT_TYPE_ICONS[eventType] ?? 'file';
}

const ACTOR_LABELS: Record<string, string> = {
	provider: 'Provider',
	tenant: 'Tenant',
	system: 'System'
};

export function formatEventActor(actor: string): string {
	const known = ACTOR_LABELS[actor];
	if (known) return known;
	return actor.charAt(0).toUpperCase() + actor.slice(1);
}
