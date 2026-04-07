import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createPasswordResetPoller, POLL_INTERVAL_MS, POLL_TIMEOUT_MS } from '$lib/utils/password-reset-poller';

describe('createPasswordResetPoller', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('starts in idle status', () => {
		const poller = createPasswordResetPoller();
		expect(poller.status).toBe('idle');
	});

	it('transitions to polling status after start', () => {
		const poller = createPasswordResetPoller();
		const fetchContract = vi.fn().mockResolvedValue({ password_reset_requested_at_ns: 12345 });
		poller.start(fetchContract, vi.fn(), vi.fn());
		expect(poller.status).toBe('polling');
		poller.stop();
	});

	it('calls onComplete and transitions to complete when password_reset_requested_at_ns is null', async () => {
		const poller = createPasswordResetPoller(100, 60_000);
		const fetchContract = vi.fn().mockResolvedValue({ password_reset_requested_at_ns: undefined });
		const onComplete = vi.fn();
		const onTimeout = vi.fn();

		poller.start(fetchContract, onComplete, onTimeout);
		await vi.advanceTimersByTimeAsync(100);

		expect(onComplete).toHaveBeenCalledOnce();
		expect(onTimeout).not.toHaveBeenCalled();
		expect(poller.status).toBe('complete');
	});

	it('keeps polling while password_reset_requested_at_ns is set', async () => {
		const poller = createPasswordResetPoller(100, 60_000);
		// First two polls return "still pending", third returns "done"
		const fetchContract = vi.fn()
			.mockResolvedValueOnce({ password_reset_requested_at_ns: 1000 })
			.mockResolvedValueOnce({ password_reset_requested_at_ns: 1000 })
			.mockResolvedValue({ password_reset_requested_at_ns: undefined });
		const onComplete = vi.fn();

		poller.start(fetchContract, onComplete, vi.fn());

		await vi.advanceTimersByTimeAsync(100);
		expect(onComplete).not.toHaveBeenCalled();

		await vi.advanceTimersByTimeAsync(100);
		expect(onComplete).not.toHaveBeenCalled();

		await vi.advanceTimersByTimeAsync(100);
		expect(onComplete).toHaveBeenCalledOnce();
		expect(poller.status).toBe('complete');
	});

	it('calls onTimeout and transitions to timeout after 10 minutes', async () => {
		const poller = createPasswordResetPoller(100, 500);
		const fetchContract = vi.fn().mockResolvedValue({ password_reset_requested_at_ns: 9999 });
		const onComplete = vi.fn();
		const onTimeout = vi.fn();

		poller.start(fetchContract, onComplete, onTimeout);
		// Advance past the timeout window
		await vi.advanceTimersByTimeAsync(600);

		expect(onTimeout).toHaveBeenCalledOnce();
		expect(onComplete).not.toHaveBeenCalled();
		expect(poller.status).toBe('timeout');
	});

	it('stops polling when stop() is called', async () => {
		const poller = createPasswordResetPoller(100, 60_000);
		const fetchContract = vi.fn().mockResolvedValue({ password_reset_requested_at_ns: 1000 });

		poller.start(fetchContract, vi.fn(), vi.fn());
		await vi.advanceTimersByTimeAsync(100);
		const callsBefore = fetchContract.mock.calls.length;

		poller.stop();
		await vi.advanceTimersByTimeAsync(500);

		// No additional calls after stop
		expect(fetchContract.mock.calls.length).toBe(callsBefore);
	});

	it('uses POLL_INTERVAL_MS and POLL_TIMEOUT_MS as defaults', () => {
		expect(POLL_INTERVAL_MS).toBe(10_000);
		expect(POLL_TIMEOUT_MS).toBe(10 * 60 * 1_000);
	});

	it('handles null contract response without throwing', async () => {
		const poller = createPasswordResetPoller(100, 60_000);
		const fetchContract = vi.fn().mockResolvedValue(null);
		const onComplete = vi.fn();

		poller.start(fetchContract, onComplete, vi.fn());
		await vi.advanceTimersByTimeAsync(100);

		// null contract (no password_reset_requested_at_ns) counts as complete
		expect(onComplete).toHaveBeenCalledOnce();
		expect(poller.status).toBe('complete');
	});

	it('continues polling after a transient fetchContract rejection', async () => {
		const poller = createPasswordResetPoller(100, 60_000);
		const fetchContract = vi.fn()
			.mockRejectedValueOnce(new Error('network error'))
			.mockResolvedValue({ password_reset_requested_at_ns: undefined });
		const onComplete = vi.fn();
		const onTimeout = vi.fn();

		poller.start(fetchContract, onComplete, onTimeout);

		await vi.advanceTimersByTimeAsync(100);
		expect(onComplete).not.toHaveBeenCalled();
		expect(poller.status).toBe('polling');

		await vi.advanceTimersByTimeAsync(100);
		expect(onComplete).toHaveBeenCalledOnce();
		expect(poller.status).toBe('complete');
	});
});
