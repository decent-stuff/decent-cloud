import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// `/dashboard/sign-in` is a natural guess for the login page but no such route
// exists (the real login is at `/login`). Treat it as a convenience alias and
// 307 (Temporary Redirect) — this is not a permanent move.
export const load: PageLoad = () => {
	throw redirect(307, '/login');
};
