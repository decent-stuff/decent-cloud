/**
 * Debug logging that is tree-shaken out of production builds.
 */
export function debugLog(message: string, ...args: unknown[]): void {
	if (import.meta.env.DEV) {
		console.debug(message, ...args);
	}
}
