/**
 * Password reset polling state machine.
 *
 * Polls a contract fetch function every POLL_INTERVAL_MS until
 * `password_reset_requested_at_ns` is cleared (reset complete) or
 * POLL_TIMEOUT_MS elapses.
 */
export const POLL_INTERVAL_MS = 10_000;
export const POLL_TIMEOUT_MS = 10 * 60 * 1_000; // 10 minutes

export type PollStatus = 'idle' | 'polling' | 'complete' | 'timeout';

export interface PasswordResetPoller {
	/** Start polling. Calls onComplete when reset is done, onTimeout on expiry. */
	start(
		fetchContract: () => Promise<{ password_reset_requested_at_ns?: number } | null>,
		onComplete: () => void,
		onTimeout: () => void,
	): void;
	/** Stop polling and release the interval. */
	stop(): void;
	/** Current status — useful for testing. */
	status: PollStatus;
}

export function createPasswordResetPoller(
	intervalMs = POLL_INTERVAL_MS,
	timeoutMs = POLL_TIMEOUT_MS,
): PasswordResetPoller {
	let handle: ReturnType<typeof setInterval> | null = null;
	let startTime = 0;
	let _status: PollStatus = 'idle';

	const poller: PasswordResetPoller = {
		get status() {
			return _status;
		},

		start(fetchContract, onComplete, onTimeout) {
			poller.stop();
			startTime = Date.now();
			_status = 'polling';

			handle = setInterval(async () => {
				if (Date.now() - startTime > timeoutMs) {
					poller.stop();
					_status = 'timeout';
					onTimeout();
					return;
				}

				try {
					const contract = await fetchContract();
					if (!contract?.password_reset_requested_at_ns) {
						poller.stop();
						_status = 'complete';
						onComplete();
					}
				} catch {
					// Transient fetch error — retry on next tick.
				}
			}, intervalMs);
		},

		stop() {
			if (handle !== null) {
				clearInterval(handle);
				handle = null;
			}
		},
	};

	return poller;
}
