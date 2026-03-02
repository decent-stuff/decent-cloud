export type AuthCtaKind = 'google' | 'seed' | 'back';

const AUTH_CTA_CLASSES: Record<AuthCtaKind, string> = {
	google:
		'btn-secondary btn-control-md w-full gap-3 border-neutral-700 hover:border-neutral-500 group',
	seed: 'btn-secondary btn-control-md w-full',
	back: 'btn-tertiary btn-control-md'
};

export function getAuthCtaClass(kind: AuthCtaKind): string {
	return AUTH_CTA_CLASSES[kind];
}
