import { describe, it, expect, vi, beforeEach } from 'vitest';

describe('debugLog', () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	it('calls console.debug in dev mode', async () => {
		vi.stubEnv('DEV', true);
		vi.resetModules();
		const { debugLog } = await import('./debug');
		const spy = vi.spyOn(console, 'debug').mockImplementation(() => {});
		debugLog('test message', { key: 'value' });
		expect(spy).toHaveBeenCalledWith('test message', { key: 'value' });
	});

	it('does not call console.debug in production mode', async () => {
		vi.stubEnv('DEV', false);
		vi.resetModules();
		const { debugLog } = await import('./debug');
		const spy = vi.spyOn(console, 'debug').mockImplementation(() => {});
		debugLog('should not appear');
		expect(spy).not.toHaveBeenCalled();
	});
});
