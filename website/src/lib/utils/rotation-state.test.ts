import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { RotationStateTracker } from './rotation-state';

describe('RotationStateTracker', () => {
	beforeEach(() => { vi.useFakeTimers(); });
	afterEach(() => { vi.useRealTimers(); });

	describe('getStatus — 3-state indicator', () => {
		it('returns "idle" when no rotation is active or recently completed', () => {
			const tracker = new RotationStateTracker();
			expect(tracker.getStatus('contract-1')).toBe('idle');
		});

		it('returns "idle" when rotationRequestedAtNs is undefined', () => {
			const tracker = new RotationStateTracker();
			expect(tracker.getStatus('contract-1', undefined)).toBe('idle');
		});

		it('returns "in_progress" when rotationRequestedAtNs is set', () => {
			const tracker = new RotationStateTracker();
			expect(tracker.getStatus('contract-1', 1700000000000000000)).toBe('in_progress');
		});

		it('returns "completed" after markCompleted, overriding in_progress', () => {
			const tracker = new RotationStateTracker();
			tracker.markCompleted('contract-1');
			expect(tracker.getStatus('contract-1', 1700000000000000000)).toBe('completed');
		});

		it('returns "completed" after markCompleted even without rotationRequestedAtNs', () => {
			const tracker = new RotationStateTracker();
			tracker.markCompleted('contract-1');
			expect(tracker.getStatus('contract-1')).toBe('completed');
		});
	});

	describe('recentlyCompletedRotations auto-clear after TTL', () => {
		it('removes contract from completed set after TTL expires', () => {
			const tracker = new RotationStateTracker({ ttlMs: 30_000 });
			tracker.markCompleted('contract-1');
			expect(tracker.isRecentlyCompleted('contract-1')).toBe(true);

			vi.advanceTimersByTime(29_999);
			expect(tracker.isRecentlyCompleted('contract-1')).toBe(true);

			vi.advanceTimersByTime(1);
			expect(tracker.isRecentlyCompleted('contract-1')).toBe(false);
		});

		it('returns to "idle" after TTL when rotationRequestedAtNs is cleared', () => {
			const tracker = new RotationStateTracker({ ttlMs: 30_000 });
			tracker.markCompleted('contract-1');
			expect(tracker.getStatus('contract-1')).toBe('completed');

			vi.advanceTimersByTime(30_000);
			expect(tracker.getStatus('contract-1')).toBe('idle');
		});

		it('returns to "in_progress" after TTL if rotationRequestedAtNs is still set', () => {
			const tracker = new RotationStateTracker({ ttlMs: 30_000 });
			tracker.markCompleted('contract-1');

			vi.advanceTimersByTime(30_000);
			expect(tracker.getStatus('contract-1', 1700000000000000000)).toBe('in_progress');
		});
	});

	describe('multiple concurrent rotations', () => {
		it('tracks multiple contracts independently', () => {
			const tracker = new RotationStateTracker({ ttlMs: 30_000 });

			tracker.markCompleted('contract-A');
			vi.advanceTimersByTime(15_000);
			tracker.markCompleted('contract-B');

			expect(tracker.isRecentlyCompleted('contract-A')).toBe(true);
			expect(tracker.isRecentlyCompleted('contract-B')).toBe(true);

			vi.advanceTimersByTime(15_001);
			expect(tracker.isRecentlyCompleted('contract-A')).toBe(false);
			expect(tracker.isRecentlyCompleted('contract-B')).toBe(true);

			vi.advanceTimersByTime(15_000);
			expect(tracker.isRecentlyCompleted('contract-B')).toBe(false);
		});

		it('re-marking same contract resets its TTL', () => {
			const tracker = new RotationStateTracker({ ttlMs: 30_000 });

			tracker.markCompleted('contract-1');
			vi.advanceTimersByTime(20_000);

			tracker.markCompleted('contract-1');
			vi.advanceTimersByTime(20_000);
			expect(tracker.isRecentlyCompleted('contract-1')).toBe(true);

			vi.advanceTimersByTime(10_001);
			expect(tracker.isRecentlyCompleted('contract-1')).toBe(false);
		});
	});

	describe('full lifecycle: requested → in_progress → completed → idle', () => {
		it('transitions through all 3 states correctly', () => {
			const tracker = new RotationStateTracker({ ttlMs: 30_000 });

			expect(tracker.getStatus('c1')).toBe('idle');

			expect(tracker.getStatus('c1', 1700000000000000000)).toBe('in_progress');

			tracker.markCompleted('c1');
			expect(tracker.getStatus('c1')).toBe('completed');

			vi.advanceTimersByTime(30_000);
			expect(tracker.getStatus('c1')).toBe('idle');
		});
	});

	describe('onChange callback', () => {
		it('fires onChange when markCompleted is called', () => {
			const onChange = vi.fn();
			const tracker = new RotationStateTracker({ onChange });
			tracker.markCompleted('c1');
			expect(onChange).toHaveBeenCalledTimes(1);
		});

		it('fires onChange when TTL expires', () => {
			const onChange = vi.fn();
			const tracker = new RotationStateTracker({ ttlMs: 5_000, onChange });
			tracker.markCompleted('c1');
			expect(onChange).toHaveBeenCalledTimes(1);

			vi.advanceTimersByTime(5_000);
			expect(onChange).toHaveBeenCalledTimes(2);
		});
	});

	describe('clearAll', () => {
		it('removes all completed entries and cancels timers', () => {
			const tracker = new RotationStateTracker({ ttlMs: 30_000 });
			tracker.markCompleted('c1');
			tracker.markCompleted('c2');

			tracker.clearAll();
			expect(tracker.isRecentlyCompleted('c1')).toBe(false);
			expect(tracker.isRecentlyCompleted('c2')).toBe(false);

			vi.advanceTimersByTime(60_000);
			expect(tracker.isRecentlyCompleted('c1')).toBe(false);
			expect(tracker.isRecentlyCompleted('c2')).toBe(false);
		});
	});
});
