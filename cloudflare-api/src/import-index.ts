import { Env } from './types';
import { CanisterService } from './services/canister-service';
import { JsonResponse } from './utils';

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    // Handle CORS preflight requests
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: {
          'Access-Control-Allow-Origin': '*',
          'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
          'Access-Control-Allow-Headers': 'Content-Type',
        }
      });
    }

    try {
      switch (url.pathname) {
        // Health check
        case '/':
        case '/health':
          return JsonResponse.json({
            success: true,
            message: 'Decent Cloud Import Worker is running',
            environment: env.ENVIRONMENT,
            canisterId: env.CANISTER_ID,
            timestamp: new Date().toISOString()
          });

        // Test canister connectivity
        case '/import/test':
          if (request.method !== 'POST') {
            return JsonResponse.error('Method not allowed', 405);
          }

          const canisterService = new CanisterService(env);
          try {
            const result = await canisterService.callMethod('provider_list_registered', []);
            return JsonResponse.json({
              success: true,
              message: 'Canister connectivity test successful',
              data: {
                providersCount: Array.isArray(result) ? result.length : 0,
                rawResult: result
              }
            });
          } catch (error) {
            return JsonResponse.json({
              success: false,
              message: 'Canister connectivity test failed',
              error: error instanceof Error ? error.message : 'Unknown error'
            });
          }

        // Simple import - fetch providers only
        case '/import/providers':
          if (request.method !== 'POST') {
            return JsonResponse.error('Method not allowed', 405);
          }

          const importService = new CanisterService(env);
          try {
            const providers = await importService.callMethod('provider_list_registered', []);
            const checkedIn = await importService.callMethod('provider_list_checked_in', []);

            return JsonResponse.json({
              success: true,
              message: 'Provider import completed',
              data: {
                registeredProviders: Array.isArray(providers) ? providers.length : 0,
                checkedInProviders: Array.isArray(checkedIn) ? checkedIn.length : 0,
                providers: providers
              }
            });
          } catch (error) {
            return JsonResponse.json({
              success: false,
              message: 'Provider import failed',
              error: error instanceof Error ? error.message : 'Unknown error'
            });
          }

        default:
          return JsonResponse.error('Not Found', 404);
      }

    } catch (error: any) {
      console.error('Import Worker Error:', error.message, error.stack);
      return JsonResponse.error(
        `Internal Server Error: ${error.message}`,
        500
      );
    }
  },
} satisfies ExportedHandler<Env>;