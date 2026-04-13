export type RotationStatus = 'idle' | 'in_progress' | 'completed';

export class RotationStateTracker {
	private completed: Set<string> = new Set();
	private timers: Map<string, ReturnType<typeof setTimeout>> = new Map();
	private ttlMs: number;
	private onChange?: () => void;

	get completedSet(): Readonly<Set<string>> {
		return this.completed;
	}

	constructor(options?: { ttlMs?: number; onChange?: () => void }) {
		this.ttlMs = options?.ttlMs ?? 30_000;
		this.onChange = options?.onChange;
	}

	getStatus(contractId: string, rotationRequestedAtNs?: number): RotationStatus {
		if (this.completed.has(contractId)) return 'completed';
		if (rotationRequestedAtNs !== undefined && rotationRequestedAtNs !== null) return 'in_progress';
		return 'idle';
	}

	isRecentlyCompleted(contractId: string): boolean {
		return this.completed.has(contractId);
	}

	markCompleted(contractId: string): void {
		this.clearTimer(contractId);
		const next = new Set(this.completed);
		next.add(contractId);
		this.completed = next;
		this.onChange?.();
		const timer = setTimeout(() => {
			const after = new Set(this.completed);
			after.delete(contractId);
			this.completed = after;
			this.timers.delete(contractId);
			this.onChange?.();
		}, this.ttlMs);
		this.timers.set(contractId, timer);
	}

	clearAll(): void {
		for (const timer of this.timers.values()) {
			clearTimeout(timer);
		}
		this.timers.clear();
		this.completed = new Set();
	}

	private clearTimer(contractId: string): void {
		const existing = this.timers.get(contractId);
		if (existing !== undefined) {
			clearTimeout(existing);
			this.timers.delete(contractId);
		}
	}
}
