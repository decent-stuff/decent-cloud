import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
	createSshKeyRotationPoller,
	POLL_INTERVAL_MS,
	POLL_TIMEOUT_MS
} from './ssh-key-rotation-poller';

describe('createSshKeyRotationPoller', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('starts in idle status', () => {
		const poller = createSshKeyRotationPoller();
		expect(poller.status).toBe('idle');
	});

	it('transitions to polling status after start', () => {
		const poller = createSshKeyRotationPoller();
		const fetchContract = vi.fn().mockResolvedValue({ ssh_key_rotation_requested_at_ns: 12345 });
		poller.start(fetchContract, vi.fn(), vi.fn());
		expect(poller.status).toBe('polling');
		poller.stop();
	});

	it('calls onComplete when ssh_key_rotation_requested_at_ns is cleared', async () => {
		const poller = createSshKeyRotationPoller(100, 60_000);
		const fetchContract = vi.fn().mockResolvedValue({ ssh_key_rotation_requested_at_ns: undefined });
		const onComplete = vi.fn();
		const onTimeout = vi.fn();

		poller.start(fetchContract, onComplete, onTimeout);
		await vi.advanceTimersByTimeAsync(100);

		expect(onComplete).toHaveBeenCalledOnce();
		expect(onTimeout).not.toHaveBeenCalled();
		expect(poller.status).toBe('complete');
	});

	it('keeps polling while ssh_key_rotation_requested_at_ns remains set', async () => {
		const poller = createSshKeyRotationPoller(100, 60_000);
		const fetchContract = vi.fn()
			.mockResolvedValueOnce({ ssh_key_rotation_requested_at_ns: 1000 })
			.mockResolvedValueOnce({ ssh_key_rotation_requested_at_ns: 1000 })
			.mockResolvedValue({ ssh_key_rotation_requested_at_ns: undefined });
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

	it('calls onTimeout when rotation takes too long', async () => {
		const poller = createSshKeyRotationPoller(100, 500);
		const fetchContract = vi.fn().mockResolvedValue({ ssh_key_rotation_requested_at_ns: 9999 });
		const onComplete = vi.fn();
		const onTimeout = vi.fn();

		poller.start(fetchContract, onComplete, onTimeout);
		await vi.advanceTimersByTimeAsync(600);

		expect(onTimeout).toHaveBeenCalledOnce();
		expect(onComplete).not.toHaveBeenCalled();
		expect(poller.status).toBe('timeout');
	});

	it('uses repo polling defaults', () => {
		expect(POLL_INTERVAL_MS).toBe(10_000);
		expect(POLL_TIMEOUT_MS).toBe(10 * 60 * 1_000);
	});
});
