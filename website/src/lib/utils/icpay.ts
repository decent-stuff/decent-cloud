import { Icpay } from '@ic-pay/icpay-sdk';
import { createWalletSelect } from '@ic-pay/icpay-widget';

let icpayInstance: Icpay | null = null;
let walletSelectInstance: ReturnType<typeof createWalletSelect> | null = null;

export function getWalletSelect(): ReturnType<typeof createWalletSelect> {
	if (!walletSelectInstance) {
		walletSelectInstance = createWalletSelect();
	}
	return walletSelectInstance;
}

export function getIcpay(): Icpay | null {
	if (icpayInstance) return icpayInstance;

	const publishableKey = import.meta.env.VITE_ICPAY_PUBLISHABLE_KEY;
	if (!publishableKey) return null;

	const walletSelect = getWalletSelect();
	const account = walletSelect.account;

	icpayInstance = new Icpay({
		publishableKey,
		actorProvider: (canisterId, idl) =>
			walletSelect.getActor({ canisterId, idl, requiresSigning: true, anon: false }),
		connectedWallet: { owner: account?.owner || account?.principal || undefined },
		enableEvents: true,
	});
	return icpayInstance;
}

export function isIcpayConfigured(): boolean {
	return !!import.meta.env.VITE_ICPAY_PUBLISHABLE_KEY;
}
