// Minimal assertion primitive: hard-fails the current flow with a debuggable
// message. Thrown AssertionErrors are caught by the runner and recorded as FAIL.

export class AssertionError extends Error {
	constructor(detail) {
		super(detail);
		this.name = 'AssertionError';
		this.detail = detail;
	}
}

/** Throw an AssertionError if `cond` is falsy. */
export function assert(cond, detail) {
	if (!cond) throw new AssertionError(detail);
}

/** Throw an AssertionError if a !== b (with a label). */
export function assertEquals(a, b, label) {
	if (a !== b) {
		throw new AssertionError(`${label}: expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`);
	}
}
