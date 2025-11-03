/**
 * Ledger Import Service
 * 
 * Synchronizes data from the ICP canister ledger into the Cloudflare D1 database.
 * Uses the next_block_entries endpoint to fetch ledger data and imports it.
 */

import { Env, LedgerEntry, SyncStatus } from '../types';
import { IcpAgent, NextBlockEntry } from './icp-agent';
import { DatabaseService } from '../utils';

export interface ImportStats {
  totalProcessed: number;
  usersCreated: number;
  profilesCreated: number;
  offeringsCreated: number;
  ledgerEntriesCreated: number;
  errors: string[];
}

/**
 * Service for importing ledger data from ICP canister to CF D1 database
 */
export class LedgerImportService {
  private icpAgent: IcpAgent;
  private db: DatabaseService;

  constructor(private env: Env) {
    this.icpAgent = new IcpAgent(env);
    this.db = new DatabaseService(env);
  }

  /**
   * Import ledger data from the canister
   * @param batchSize Number of entries to fetch per batch
   * @param label Optional label to filter entries
   */
  async importLedgerData(
    batchSize: number = 100,
    label?: string
  ): Promise<ImportStats> {
    const stats: ImportStats = {
      totalProcessed: 0,
      usersCreated: 0,
      profilesCreated: 0,
      offeringsCreated: 0,
      ledgerEntriesCreated: 0,
      errors: [],
    };

    try {
      let offset = 0;
      let hasMore = true;

      // Get current sync status
      const syncStatus = await this.getSyncStatus(label || 'all');
      if (syncStatus) {
        offset = syncStatus.totalRecordsSynced;
        console.log(`Resuming sync from offset ${offset}`);
      }

      while (hasMore) {
        console.log(`Fetching entries: offset=${offset}, limit=${batchSize}, label=${label || 'all'}`);

        // Fetch entries from canister
        const result = await this.icpAgent.getNextBlockEntries(label, offset, batchSize);

        console.log(`Received ${result.entries.length} entries, has_more=${result.has_more}`);

        // Process each entry
        for (const entry of result.entries) {
          try {
            await this.processEntry(entry, stats);
            stats.totalProcessed++;
          } catch (error) {
            const errorMsg = `Failed to process entry: ${error instanceof Error ? error.message : String(error)}`;
            console.error(errorMsg);
            stats.errors.push(errorMsg);
          }
        }

        // Update sync status
        await this.updateSyncStatus(
          label || 'all',
          offset + result.entries.length,
          stats.totalProcessed,
          stats.errors.length,
          stats.errors[stats.errors.length - 1]
        );

        offset += result.entries.length;
        hasMore = result.has_more;

        // Break if we've processed all available entries
        if (!hasMore || result.entries.length === 0) {
          break;
        }
      }

      console.log('Import complete:', stats);
    } catch (error) {
      const errorMsg = `Import failed: ${error instanceof Error ? error.message : String(error)}`;
      console.error(errorMsg);
      stats.errors.push(errorMsg);
    }

    return stats;
  }

  /**
   * Process a single ledger entry
   */
  private async processEntry(entry: NextBlockEntry, stats: ImportStats): Promise<void> {
    const { label, key, value } = entry;

    // Convert key to hex string
    const keyHex = IcpAgent.uint8ArrayToHex(key);

    // Store raw ledger entry
    await this.storeLedgerEntry(label, keyHex, value);
    stats.ledgerEntriesCreated++;

    // Process by label type
    switch (label) {
      case 'ProvRegister':
      case 'UserRegister':
        await this.processUserRegistration(keyHex, value, stats);
        break;

      case 'ProvProfile':
        await this.processProviderProfile(keyHex, value, stats);
        break;

      case 'ProvOffering':
        await this.processProviderOffering(keyHex, value, stats);
        break;

      case 'ContractSignRequest':
        // TODO: Implement contract processing
        console.log('Contract processing not yet implemented');
        break;

      case 'RewardDistribution':
        // TODO: Implement reward processing
        console.log('Reward processing not yet implemented');
        break;

      default:
        console.log(`Unknown label type: ${label}`);
    }
  }

  /**
   * Store raw ledger entry
   */
  private async storeLedgerEntry(label: string, key: string, value: Uint8Array): Promise<void> {
    const database = this.db.getDatabase();

    await database.prepare(`
      INSERT INTO ledger_entries (label, key, value, block_offset, operation, icp_timestamp)
      VALUES (?, ?, ?, ?, ?, ?)
    `).bind(
      label,
      key,
      value,
      0, // block_offset - we don't have this from next_block_entries
      'INSERT',
      Date.now() * 1_000_000 // Convert to nanoseconds
    ).run();
  }

  /**
   * Process user/provider registration
   */
  private async processUserRegistration(pubkey: string, _value: Uint8Array, stats: ImportStats): Promise<void> {
    // Check if user already exists
    const existingUser = await this.db.getDcUser(pubkey);
    if (existingUser) {
      console.log(`User ${pubkey} already exists, skipping`);
      return;
    }

    // Create user with default values
    await this.db.createDcUser({
      pubkey: pubkey,
      principal: undefined,
      reputation: 0,
      balanceTokens: 0,
    });

    stats.usersCreated++;
    console.log(`Created user: ${pubkey}`);
  }

  /**
   * Process provider profile
   */
  private async processProviderProfile(pubkey: string, profileData: Uint8Array, stats: ImportStats): Promise<void> {
    // Ensure user exists
    const user = await this.db.getDcUser(pubkey);
    if (!user) {
      // Create user first
      await this.db.createDcUser({
        pubkey: pubkey,
        principal: undefined,
        reputation: 0,
        balanceTokens: 0,
      });
      stats.usersCreated++;
    }

    // Create or update profile
    await this.db.createProviderProfile({
      pubkey: pubkey,
      profileData: profileData,
      signature: undefined,
      version: 1,
    });

    stats.profilesCreated++;
    console.log(`Created/updated profile: ${pubkey}`);
  }

  /**
   * Process provider offering
   */
  private async processProviderOffering(offeringKey: string, offeringData: Uint8Array, stats: ImportStats): Promise<void> {
    // Extract provider pubkey from offering data if possible
    // For now, use the offering key as ID
    const offeringId = offeringKey;

    // Try to extract provider pubkey (first 64 hex chars = 32 bytes)
    let providerPubkey = offeringKey;
    if (offeringKey.length > 64) {
      providerPubkey = offeringKey.substring(0, 64);
    }

    // Ensure user exists
    const user = await this.db.getDcUser(providerPubkey);
    if (!user) {
      // Create user first
      await this.db.createDcUser({
        pubkey: providerPubkey,
        principal: undefined,
        reputation: 0,
        balanceTokens: 0,
      });
      stats.usersCreated++;
    }

    // Create or update offering
    await this.db.createProviderOffering({
      id: offeringId,
      providerPubkey: providerPubkey,
      offeringData: offeringData,
      signature: undefined,
      version: 1,
      isActive: true,
    });

    stats.offeringsCreated++;
    console.log(`Created/updated offering: ${offeringId}`);
  }

  /**
   * Get sync status for a table
   */
  private async getSyncStatus(tableName: string): Promise<SyncStatus | null> {
    const database = this.db.getDatabase();

    const result = await database.prepare(`
      SELECT * FROM sync_status WHERE table_name = ?
    `).bind(tableName).first();

    if (!result) return null;

    return {
      tableName: result.table_name as string,
      lastSyncedBlockOffset: result.last_synced_block_offset as number,
      lastSyncedAt: result.last_synced_at as string,
      totalRecordsSynced: result.total_records_synced as number,
      syncErrors: result.sync_errors as number,
      lastError: result.last_error as string | undefined,
    };
  }

  /**
   * Update sync status
   */
  private async updateSyncStatus(
    tableName: string,
    blockOffset: number,
    totalRecords: number,
    errorCount: number,
    lastError?: string
  ): Promise<void> {
    const database = this.db.getDatabase();

    await database.prepare(`
      INSERT INTO sync_status (
        table_name, last_synced_block_offset, last_synced_at,
        total_records_synced, sync_errors, last_error
      )
      VALUES (?, ?, datetime('now'), ?, ?, ?)
      ON CONFLICT(table_name) DO UPDATE SET
        last_synced_block_offset = excluded.last_synced_block_offset,
        last_synced_at = excluded.last_synced_at,
        total_records_synced = excluded.total_records_synced,
        sync_errors = excluded.sync_errors,
        last_error = excluded.last_error
    `).bind(
      tableName,
      blockOffset,
      totalRecords,
      errorCount,
      lastError || null
    ).run();
  }

  /**
   * Get all sync statuses
   */
  async getAllSyncStatuses(): Promise<SyncStatus[]> {
    const database = this.db.getDatabase();

    const results = await database.prepare(`
      SELECT * FROM sync_status ORDER BY table_name
    `).all();

    return results.results.map((row: any) => ({
      tableName: row.table_name as string,
      lastSyncedBlockOffset: row.last_synced_block_offset as number,
      lastSyncedAt: row.last_synced_at as string,
      totalRecordsSynced: row.total_records_synced as number,
      syncErrors: row.sync_errors as number,
      lastError: row.last_error as string | undefined,
    }));
  }

  /**
   * Import specific label type
   */
  async importByLabel(label: string, batchSize: number = 100): Promise<ImportStats> {
    console.log(`Starting import for label: ${label}`);
    return await this.importLedgerData(batchSize, label);
  }

  /**
   * Import all ledger data (no label filter)
   */
  async importAll(batchSize: number = 100): Promise<ImportStats> {
    console.log('Starting full ledger import');
    return await this.importLedgerData(batchSize);
  }
}
