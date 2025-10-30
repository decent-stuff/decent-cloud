import { Env } from '../types';
import { CanisterService } from '../services/canister-service';
import { JsonResponse, CorsHandler } from '../utils';

/**
 * Handle canister-compatible requests
 * Accepts the same format as the canister and returns the same responses
 */
export async function handleCanisterRequest(request: Request, env: Env): Promise<Response> {
  // Handle CORS preflight
  if (request.method === 'OPTIONS') {
    return CorsHandler.handle();
  }

  const url = new URL(request.url);
  const method = url.searchParams.get('method');

  if (!method) {
    return JsonResponse.error('Method parameter is required', 400);
  }

  const canisterService = new CanisterService(env);

  try {
    // Parse arguments from request body
    let args: any[] = [];
    if (request.method === 'POST') {
      const body = await request.json() as { args?: any[] };
      args = body.args || [];
    }

    // Call the canister service method
    const result = await (canisterService as any)[method](...args);

    return JsonResponse.success(result);
  } catch (error: any) {
    console.error(`Canister method ${method} failed:`, error);

    // Return canister-style error
    return JsonResponse.success({
      Err: error.message || 'Unknown error'
    });
  }
}

/**
 * Handle direct canister method calls with proper URL routing
 */
export async function handleCanisterMethod(request: Request, env: Env, methodName: string): Promise<Response> {
  // Handle CORS preflight
  if (request.method === 'OPTIONS') {
    return CorsHandler.handle();
  }

  const canisterService = new CanisterService(env);

  try {
    // Parse arguments from request body
    let args: any[] = [];
    if (request.method === 'POST') {
      const body = await request.json() as { args?: any[] };
      args = body.args || [];
    }

    // Call the canister service method
    const method = methodName as keyof CanisterService;
    if (typeof (canisterService as any)[method] === 'function') {
      const result = await (canisterService as any)[method](...args);
      return JsonResponse.success(result);
    } else {
      return JsonResponse.success({
        Err: `Method ${methodName} not implemented`
      });
    }
  } catch (error: any) {
    console.error(`Canister method ${methodName} failed:`, error);

    // Return canister-style error
    return JsonResponse.success({
      Err: error.message || 'Unknown error'
    });
  }
}