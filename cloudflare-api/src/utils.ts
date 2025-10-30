import { Env, DcUser, ProviderProfile, ProviderOffering } from './types';

export class CorsHandler {
  static handle(): Response {
    return new Response(null, {
      status: 200,
      headers: {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization',
      },
    });
  }

  static addHeaders(response: Response): Response {
    response.headers.set('Access-Control-Allow-Origin', '*');
    response.headers.set('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
    response.headers.set('Access-Control-Allow-Headers', 'Content-Type, Authorization');
    return response;
  }
}

export class JsonResponse {
  static success<T>(data: T, status = 200): Response {
    const response = new Response(JSON.stringify({
      success: true,
      data
    }), {
      status,
      headers: { 'Content-Type': 'application/json' }
    });
    return CorsHandler.addHeaders(response);
  }

  static error(message: string, status = 500, details?: string): Response {
    const response = new Response(JSON.stringify({
      success: false,
      error: message,
      ...(details && { details })
    }), {
      status,
      headers: { 'Content-Type': 'application/json' }
    });
    return CorsHandler.addHeaders(response);
  }
}

export class DatabaseService {
  constructor(private env: Env) {}

  getDatabase(): D1Database {
    // If TEST_DB_NAME is specified, use dynamic database selection
    if (this.env.TEST_DB_NAME) {
      // Use TEST_DB binding for test environment
      return (this.env as any).TEST_DB || this.env.DB;
    }
    return this.env.DB;
  }

  // Decent Cloud Core Methods

  async getDcUser(pubkey: string): Promise<DcUser | null> {
    const db = this.getDatabase();
    const user = await db.prepare(`
      SELECT * FROM dc_users WHERE pubkey = ?
    `).bind(pubkey).first();

    if (!user) return null;

    return {
      id: user.id as string,
      pubkey: user.pubkey as string,
      principal: user.principal as string | undefined,
      reputation: user.reputation as number,
      balanceTokens: user.balance_tokens as number,
      createdAt: user.created_at as string,
      updatedAt: user.updated_at as string
    };
  }

  async createDcUser(user: Omit<DcUser, 'id' | 'createdAt' | 'updatedAt'>): Promise<DcUser> {
    const db = this.getDatabase();
    const userId = `user_${user.pubkey}`;
    const now = new Date().toISOString();

    await db.prepare(`
      INSERT INTO dc_users (id, pubkey, principal, reputation, balance_tokens, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `).bind(
      userId,
      user.pubkey,
      user.principal || null,
      user.reputation,
      user.balanceTokens,
      now,
      now
    ).run();

    return {
      id: userId,
      ...user,
      createdAt: now,
      updatedAt: now
    };
  }

  async getProviderProfile(pubkey: string): Promise<ProviderProfile | null> {
    const db = this.getDatabase();
    const profile = await db.prepare(`
      SELECT * FROM provider_profiles WHERE pubkey = ?
    `).bind(pubkey).first();

    if (!profile) return null;

    return {
      pubkey: profile.pubkey as string,
      profileData: profile.profile_data as ArrayBuffer | Uint8Array,
      signature: profile.signature as ArrayBuffer | Uint8Array | undefined,
      version: profile.version as number,
      createdAt: profile.created_at as string,
      updatedAt: profile.updated_at as string
    };
  }

  async createProviderProfile(profile: Omit<ProviderProfile, 'createdAt' | 'updatedAt'>): Promise<ProviderProfile> {
    const db = this.getDatabase();
    const now = new Date().toISOString();

    await db.prepare(`
      INSERT INTO provider_profiles (pubkey, profile_data, signature, version, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(pubkey) DO UPDATE SET
        profile_data = excluded.profile_data,
        signature = excluded.signature,
        version = excluded.version,
        updated_at = excluded.updated_at
    `).bind(
      profile.pubkey,
      profile.profileData,
      profile.signature || null,
      profile.version,
      now,
      now
    ).run();

    return {
      ...profile,
      createdAt: now,
      updatedAt: now
    };
  }

  async getProviderOfferings(providerPubkey?: string, isActive?: boolean): Promise<ProviderOffering[]> {
    const db = this.getDatabase();

    let whereConditions = [];
    let bindings: any[] = [];

    if (providerPubkey) {
      whereConditions.push('provider_pubkey = ?');
      bindings.push(providerPubkey);
    }

    if (isActive !== undefined) {
      whereConditions.push('is_active = ?');
      bindings.push(isActive ? 1 : 0);
    }

    const whereClause = whereConditions.length > 0 ? `WHERE ${whereConditions.join(' AND ')}` : '';

    const results = await db.prepare(`
      SELECT * FROM provider_offerings
      ${whereClause}
      ORDER BY updated_at DESC
    `).bind(...bindings).all();

    return results.results.map((offering: any) => ({
      id: offering.id as string,
      providerPubkey: offering.provider_pubkey as string,
      offeringData: offering.offering_data as ArrayBuffer | Uint8Array,
      signature: offering.signature as ArrayBuffer | Uint8Array | undefined,
      version: offering.version as number,
      isActive: !!offering.is_active,
      createdAt: offering.created_at as string,
      updatedAt: offering.updated_at as string
    }));
  }

  async createProviderOffering(offering: Omit<ProviderOffering, 'createdAt' | 'updatedAt'>): Promise<ProviderOffering> {
    const db = this.getDatabase();
    const now = new Date().toISOString();

    await db.prepare(`
      INSERT INTO provider_offerings (
        id, provider_pubkey, offering_data, signature, version,
        is_active, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(id) DO UPDATE SET
        offering_data = excluded.offering_data,
        signature = excluded.signature,
        version = excluded.version,
        is_active = excluded.is_active,
        updated_at = excluded.updated_at
    `).bind(
      offering.id,
      offering.providerPubkey,
      offering.offeringData,
      offering.signature || null,
      offering.version,
      offering.isActive ? 1 : 0,
      now,
      now
    ).run();

    return {
      ...offering,
      createdAt: now,
      updatedAt: now
    };
  }
}