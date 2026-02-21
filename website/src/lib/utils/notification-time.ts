/**
 * Formats a nanosecond timestamp to a relative time string.
 *
 * Rules:
 *   < 60s    → "Xs ago"  (seconds)
 *   < 3600s  → "Xm ago"  (minutes)
 *   < 86400s → "Xh ago"  (hours)
 *   else     → "Xd ago"  (days)
 */
export function formatNotificationTime(createdAtNs: number, nowMs: number = Date.now()): string {
	const ageMs = nowMs - Math.floor(createdAtNs / 1_000_000);
	const ageSecs = Math.floor(ageMs / 1000);

	if (ageSecs < 60) return `${ageSecs}s ago`;
	const ageMins = Math.floor(ageSecs / 60);
	if (ageMins < 60) return `${ageMins}m ago`;
	const ageHours = Math.floor(ageMins / 60);
	if (ageHours < 24) return `${ageHours}h ago`;
	const ageDays = Math.floor(ageHours / 24);
	return `${ageDays}d ago`;
}
