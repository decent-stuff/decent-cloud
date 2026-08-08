import { test, expect } from '@playwright/test';

/**
 * Regression guard for the A2 "smoke suite hangs under parallelism" bug.
 *
 * ROOT CAUSE: the DB seed helper `sql()` ran `psql` via `execFileAsync` with
 * NO timeout. The worker-scoped `testAccount` fixture tears down with
 * `deleteAccountByUsername` → `sql()`, and fixture teardown runs OUTSIDE the
 * 30s per-test timeout (it executes at worker shutdown). When the teardown's
 * `DELETE FROM accounts` blocked on a row lock held by an in-flight API
 * transaction for the same account (the API inserts into `signature_audit`,
 * taking a FOR-KEY-SHARE lock on the parent `accounts` row), the unbounded
 * `psql` waited forever → the worker never exited → the suite hung for minutes
 * with no output, and every test queued on that worker never ran.
 *
 * Serial mode (1 worker) always passed because there was no parallel API
 * traffic to create the lock race; the hang only surfaced under 2+ workers.
 *
 * This spec pins the fix: every `sql()` call is now bounded by an explicit
 * timeout, so a slow/stuck query rejects in seconds instead of hanging the
 * worker indefinitely. It is NOT tagged `@smoke` (it intentionally stalls a
 * query and waits out a short timeout, ~1.5s — too slow / wrong shape for the
 * fast dev loop).
 */
test.describe('DB seed helper is bounded by a timeout', () => {
	test('sql() rejects within its timeout instead of hanging on a slow query', async () => {
		const { sql } = await import('./fixtures/seed-helpers');

		// A query that sleeps far longer than the timeout. Before the fix,
		// `sql()` had no timeout and would wait the full 8s (and forever on a
		// real lock); after the fix it rejects at the explicit 1.5s timeout.
		const start = Date.now();
		await expect(sql('SELECT pg_sleep(8)', { timeoutMs: 1500 })).rejects.toThrow();
		const elapsed = Date.now() - start;

		// Rejected near the 1.5s timeout — NOT the full 8s sleep.
		expect(elapsed, 'psql must be killed near its timeout, not left to sleep').toBeLessThan(6000);
		expect(elapsed, 'timeout must actually engage (not resolve instantly)').toBeGreaterThanOrEqual(1400);
	});
});
