/**
 * Sync Routes
 * 
 * Endpoints for synchronizing ledger data from ICP canister to CF D1
 */

import { Env } from '../types';
import { JsonResponse } from '../utils';
import { LedgerImportService } from '../services/ledger-import';

/**
 * Handle sync-related requests
 */
export async function handleSyncRequest(request: Request, env: Env): Promise<Response> {
  const url = new URL(request.url);
  const pathname = url.pathname;

  // POST /api/sync/import - Import ledger data
  if (pathname === '/api/sync/import' && request.method === 'POST') {
    return await handleImportLedger(request, env);
  }

  // POST /api/sync/import/:label - Import specific label
  if (pathname.match(/^\/api\/sync\/import\/[^/]+$/) && request.method === 'POST') {
    const label = pathname.split('/').pop()!;
    return await handleImportByLabel(request, env, label);
  }

  // GET /api/sync/status - Get sync status
  if (pathname === '/api/sync/status' && request.method === 'GET') {
    return await handleSyncStatus(request, env);
  }

  return JsonResponse.error('Not found', 404);
}

/**
 * Import all ledger data
 */
async function handleImportLedger(request: Request, env: Env): Promise<Response> {
  try {
    // Parse request body for options
    const body = await request.json().catch(() => ({})) as any;
    const batchSize = body.batchSize || 100;

    const importService = new LedgerImportService(env);
    const stats = await importService.importAll(batchSize);

    return JsonResponse.success({
      message: 'Ledger import completed',
      stats,
    });
  } catch (error) {
    console.error('Import error:', error);
    return JsonResponse.error(
      'Import failed',
      500,
      error instanceof Error ? error.message : String(error)
    );
  }
}

/**
 * Import data for specific label
 */
async function handleImportByLabel(request: Request, env: Env, label: string): Promise<Response> {
  try {
    // Parse request body for options
    const body = await request.json().catch(() => ({})) as any;
    const batchSize = body.batchSize || 100;

    const importService = new LedgerImportService(env);
    const stats = await importService.importByLabel(label, batchSize);

    return JsonResponse.success({
      message: `Import completed for label: ${label}`,
      label,
      stats,
    });
  } catch (error) {
    console.error(`Import error for label ${label}:`, error);
    return JsonResponse.error(
      'Import failed',
      500,
      error instanceof Error ? error.message : String(error)
    );
  }
}

/**
 * Get sync status
 */
async function handleSyncStatus(_request: Request, env: Env): Promise<Response> {
  try {
    const importService = new LedgerImportService(env);
    const statuses = await importService.getAllSyncStatuses();

    return JsonResponse.success({
      statuses,
      totalTables: statuses.length,
    });
  } catch (error) {
    console.error('Status error:', error);
    return JsonResponse.error(
      'Failed to get sync status',
      500,
      error instanceof Error ? error.message : String(error)
    );
  }
}
