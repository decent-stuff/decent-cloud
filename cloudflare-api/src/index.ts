import { Env } from './types';
import { CorsHandler, JsonResponse } from './utils';
import {
  handleCanisterRequest,
  handleCanisterMethod
} from './routes/canister';
import { handleSyncRequest } from './routes/sync';

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    // Handle CORS preflight requests
    if (request.method === 'OPTIONS') {
      return CorsHandler.handle();
    }

    // API Routes
    try {
      switch (url.pathname) {
        // Canister-compatible endpoints
        case '/api/v1/canister':
          return handleCanisterRequest(request, env);

        // Health check
        case '/api/v1/health':
          return new Response(JSON.stringify({
            success: true,
            message: 'Decent Cloud API is running',
            environment: env.ENVIRONMENT,
            features: {
              canisterApi: true,
              ledgerSync: true
            }
          }), {
            headers: { 'Content-Type': 'application/json' }
          });

        default:
          // Handle canister method calls
          if (url.pathname.startsWith('/api/v1/canister/')) {
            const method = url.pathname.replace('/api/v1/canister/', '');
            return handleCanisterMethod(request, env, method);
          }

          // Handle sync endpoints
          if (url.pathname.startsWith('/api/sync')) {
            return handleSyncRequest(request, env);
          }
          
          return new Response(JSON.stringify({
            success: false,
            error: 'Not Found'
          }), {
            status: 404,
            headers: { 'Content-Type': 'application/json' }
          });
      }
    } catch (err: any) {
      console.error('API Error:', err.message);
      return new Response(JSON.stringify({
        success: false,
        error: 'Internal Server Error',
        details: err.message
      }), {
        status: 500,
        headers: { 'Content-Type': 'application/json' }
      });
    }
  },
} satisfies ExportedHandler<Env>;
