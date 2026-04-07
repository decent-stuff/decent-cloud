/**
 * SSH key rotation polling state machine.
 *
 * Polls a contract fetch function until `ssh_key_rotation_requested_at_ns`
 * is cleared (rotation complete) or POLL_TIMEOUT_MS elapses.
 */
export const POLL_INTERVAL_MS = 10_000;
export const POLL_TIMEOUT_MS = 10 * 60 * 1_000;

export type PollStatus = 'idle' | 'polling' | 'complete' | 'timeout';

export interface SshKeyRotationPoller {
	start(
		fetchContract: () => Promise<{ ssh_key_rotation_requested_at_ns?: number } | null>,
		onComplete: () => void,
		onTimeout: () => void,
	): void;
	stop(): void;
	status: PollStatus;
}

export function createSshKeyRotationPoller(
	intervalMs = POLL_INTERVAL_MS,
	timeoutMs = POLL_TIMEOUT_MS,
): SshKeyRotationPoller {
	let handle: ReturnType<typeof setInterval> | null = null;
	let startTime = 0;
	let _status: PollStatus = 'idle';

	const poller: SshKeyRotationPoller = {
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
					if (!contract?.ssh_key_rotation_requested_at_ns) {
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
