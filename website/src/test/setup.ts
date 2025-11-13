// Test setup file
import { vi, beforeAll, afterAll } from 'vitest';

// Suppress console.error in tests to reduce stderr noise
// Tests that need to verify error logging should mock console.error explicitly
type MaybeProcess = { env?: Record<string, string | undefined> };
const runtimeProcess = (globalThis as typeof globalThis & { process?: MaybeProcess }).process;
const shouldSuppressErrors = runtimeProcess?.env?.VITEST_SUPPRESS_CONSOLE !== 'false';
const originalError = console.error;
beforeAll(() => {
	if (shouldSuppressErrors) {
		console.error = vi.fn();
	}
});

afterAll(() => {
	if (shouldSuppressErrors) {
		console.error = originalError;
	}
});
