// Test setup file
import { vi, beforeAll, afterAll } from 'vitest';

// Suppress console.error in tests to reduce stderr noise
// Tests that need to verify error logging should mock console.error explicitly
const originalError = console.error;
beforeAll(() => {
	console.error = vi.fn();
});

afterAll(() => {
	console.error = originalError;
});
