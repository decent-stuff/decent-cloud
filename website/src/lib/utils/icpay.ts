import { Icpay } from '@ic-pay/icpay-sdk';

let icpayInstance: Icpay | null = null;

export function getIcpay(): Icpay | null {
	if (icpayInstance) return icpayInstance;

	const publishableKey = import.meta.env.VITE_ICPAY_PUBLISHABLE_KEY;
	if (!publishableKey) return null;

	icpayInstance = new Icpay({ publishableKey });
	return icpayInstance;
}

export function isIcpayConfigured(): boolean {
	return !!import.meta.env.VITE_ICPAY_PUBLISHABLE_KEY;
}
