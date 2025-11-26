import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ url }) => {
	const action = url.searchParams.get('action');

	// Redirect to login page if action=login or action=signup
	if (action === 'login' || action === 'signup') {
		throw redirect(307, '/login');
	}

	return {};
};
