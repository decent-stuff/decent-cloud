import type { UserNotification } from '$lib/services/api';

export type SavedOfferingPriceChangeDirection = 'up' | 'down';

export interface SavedOfferingPriceChangeSummary {
	byOfferingId: Map<number, SavedOfferingPriceChangeDirection>;
	unreadNotificationIds: number[];
}

export function collectSavedOfferingPriceChanges(
	notifications: UserNotification[]
): SavedOfferingPriceChangeSummary {
	const byOfferingId = new Map<number, SavedOfferingPriceChangeDirection>();
	const unreadNotificationIds: number[] = [];

	for (const notification of notifications) {
		if (
			notification.notificationType !== 'saved_offering_price_change' ||
			notification.offeringId === undefined ||
			notification.readAt !== undefined
		) {
			continue;
		}

		unreadNotificationIds.push(notification.id);
		if (byOfferingId.has(notification.offeringId)) {
			continue;
		}

		byOfferingId.set(
			notification.offeringId,
			// The API now carries direction explicitly; titles are presentation-only.
			notification.priceDirection === 'down' ? 'down' : 'up'
		);
	}

	return { byOfferingId, unreadNotificationIds };
}
