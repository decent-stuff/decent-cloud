export interface LifecycleStage {
	key: string;
	label: string;
	icon: string;
	description: string;
}

export const LIFECYCLE_STAGES: LifecycleStage[] = [
	{ key: "payment", label: "Payment", icon: "💳", description: "Complete payment to initiate your rental request" },
	{ key: "provider", label: "Provider Review", icon: "⏳", description: "Provider is reviewing your request and will accept or reject it" },
	{ key: "provisioning", label: "Provisioning", icon: "⚙️", description: "Your VM is being created and configured" },
	{ key: "ready", label: "Ready", icon: "✅", description: "Your resource is ready to use" },
];

export interface StageTiming {
	min: number;
	max: number;
	label: string;
}

export const STAGE_TIMING: Record<string, StageTiming> = {
	requested_pending:   { min: 0,  max: 5,    label: "Payment processing (0–5 min)" },
	requested_succeeded: { min: 0,  max: 1440, label: "Provider review (up to 24 h)" },
	pending:             { min: 0,  max: 1440, label: "Provider review (up to 24 h)" },
	accepted:            { min: 0,  max: 15,   label: "Provisioning queue (up to 15 min)" },
	provisioning:        { min: 5,  max: 20,   label: "VM setup (5–20 min)" },
	provisioned:         { min: 1,  max: 5,    label: "Final checks (1–5 min)" },
	active:              { min: 0,  max: 0,    label: "Running" },
};

export interface NextStepInfo {
	text: string;
	isWaiting: boolean;
}

export function getStageIndex(status: string, paymentStatus?: string): number {
	const s = status.toLowerCase();
	const ps = paymentStatus?.toLowerCase() ?? "";

	if (s === "cancelled" || s === "rejected") return -1;
	if (s === "requested" && ps === "pending") return 0;
	if (s === "requested" && ps === "failed") return 0;
	if (s === "requested" || s === "pending") return 1;
	if (s === "accepted") return 2;
	if (s === "provisioning") return 2;
	if (s === "provisioned" || s === "active") return 3;
	return 1;
}

export function getStageTiming(status: string, paymentStatus?: string): StageTiming | null {
	const s = status.toLowerCase();
	const ps = paymentStatus?.toLowerCase() ?? "";
	if (s === "requested" && ps === "pending") return STAGE_TIMING["requested_pending"];
	if (s === "requested" && ps === "succeeded") return STAGE_TIMING["requested_succeeded"];
	if (s === "pending") return STAGE_TIMING["pending"];
	if (s === "accepted") return STAGE_TIMING["accepted"];
	if (s === "provisioning") return STAGE_TIMING["provisioning"];
	if (s === "provisioned") return STAGE_TIMING["provisioned"];
	if (s === "active") return STAGE_TIMING["active"];
	return null;
}

export function getNextStepInfo(status: string, paymentStatus?: string): NextStepInfo | null {
	const s = status.toLowerCase();
	const ps = paymentStatus?.toLowerCase() ?? "";

	if (s === "requested" && ps === "pending") {
		return { text: "Complete payment to proceed", isWaiting: false };
	}
	if (s === "requested" && ps === "failed") {
		return { text: "Payment failed. Please try again or contact support.", isWaiting: false };
	}
	if (s === "requested" && ps === "succeeded") {
		return { text: "Waiting for provider to accept your request (typically within a few hours)", isWaiting: true };
	}
	if (s === "pending") {
		return { text: "Waiting for provider response", isWaiting: true };
	}
	if (s === "accepted") {
		return { text: "Provider accepted! Waiting for provisioning to start...", isWaiting: true };
	}
	if (s === "provisioning") {
		return { text: "Provider is setting up your resource (typically 5–20 minutes)", isWaiting: true };
	}
	if (s === "provisioned" || s === "active") {
		return { text: "Your resource is ready! See connection details below.", isWaiting: false };
	}
	if (s === "rejected") {
		return { text: "Provider rejected this request. You can try another provider.", isWaiting: false };
	}
	if (s === "failed") {
		return { text: "Provisioning failed. You can request a refund or contact support.", isWaiting: false };
	}
	if (s === "cancelled") {
		return null;
	}
	return null;
}

export function stageElapsedMinutes(created_at_ns: number, status_updated_at_ns?: number): number {
	const ref_ns = status_updated_at_ns ?? created_at_ns;
	return (Date.now() - ref_ns / 1_000_000) / 60_000;
}

export function formatElapsed(minutes: number): string {
	if (minutes < 1) return "just now";
	if (minutes < 60) return `${Math.floor(minutes)}m`;
	const h = Math.floor(minutes / 60);
	const m = Math.floor(minutes % 60);
	return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

export function isStageOverdue(
	status: string,
	paymentStatus: string | undefined,
	created_at_ns: number,
	status_updated_at_ns?: number
): boolean {
	const timing = getStageTiming(status, paymentStatus);
	if (!timing || timing.max <= 0) return false;
	const elapsed = stageElapsedMinutes(created_at_ns, status_updated_at_ns);
	return elapsed > timing.max;
}

export interface ProviderTimingEstimate {
	avgResponseTimeHours: number | null;
	timeToDeliveryHours: number | null;
}

export function getStageTimingWithProvider(
	status: string,
	paymentStatus: string | undefined,
	providerTiming?: ProviderTimingEstimate | null,
): StageTiming | null {
	const s = status.toLowerCase();
	const ps = paymentStatus?.toLowerCase() ?? "";
	
	if (s === "requested" && ps === "pending") return STAGE_TIMING["requested_pending"];
	if (s === "requested" && ps === "failed") return STAGE_TIMING["requested_failed"] ?? { min: 0, max: 5, label: "Payment failed" };
	
	if (s === "requested" && ps === "succeeded") {
		if (providerTiming?.avgResponseTimeHours !== null && providerTiming?.avgResponseTimeHours !== undefined) {
			const hrs = providerTiming.avgResponseTimeHours;
			const maxMin = Math.ceil(hrs * 60);
			return { min: 0, max: maxMin, label: `Provider review (avg ~${hrs < 1 ? `${Math.round(hrs * 60)}min` : `${hrs.toFixed(1)}h`})` };
		}
		return STAGE_TIMING["requested_succeeded"];
	}
	
	if (s === "pending") {
		if (providerTiming?.avgResponseTimeHours !== null && providerTiming?.avgResponseTimeHours !== undefined) {
			const hrs = providerTiming.avgResponseTimeHours;
			const maxMin = Math.ceil(hrs * 60);
			return { min: 0, max: maxMin, label: `Provider review (avg ~${hrs < 1 ? `${Math.round(hrs * 60)}min` : `${hrs.toFixed(1)}h`})` };
		}
		return STAGE_TIMING["pending"];
	}
	
	if (s === "accepted") {
		if (providerTiming?.timeToDeliveryHours !== null && providerTiming?.timeToDeliveryHours !== undefined) {
			const hrs = providerTiming.timeToDeliveryHours;
			const maxMin = Math.ceil(hrs * 60);
			return { min: 0, max: maxMin, label: `Provisioning queue (avg ~${hrs < 1 ? `${Math.round(hrs * 60)}min` : `${hrs.toFixed(1)}h`})` };
		}
		return STAGE_TIMING["accepted"];
	}
	
	if (s === "provisioning") {
		if (providerTiming?.timeToDeliveryHours !== null && providerTiming?.timeToDeliveryHours !== undefined) {
			const hrs = providerTiming.timeToDeliveryHours;
			const minMin = Math.max(5, Math.floor(hrs * 30));
			const maxMin = Math.ceil(hrs * 90);
			return { min: minMin, max: maxMin, label: `VM setup (typically ${minMin < 60 ? `${minMin}-${maxMin}min` : `${(minMin/60).toFixed(0)}-${(maxMin/60).toFixed(0)}h`})` };
		}
		return STAGE_TIMING["provisioning"];
	}
	
	if (s === "provisioned") return STAGE_TIMING["provisioned"];
	if (s === "active") return STAGE_TIMING["active"];
	
	return null;
}

export interface ProgressSummary {
	title: string;
	message: string;
	estimatedTime: string | null;
	showSpinner: boolean;
}

export function getProgressSummary(status: string, paymentStatus?: string): ProgressSummary | null {
	const s = status.toLowerCase();
	const ps = paymentStatus?.toLowerCase() ?? "";

	if (s === "requested" && ps === "pending") {
		return {
			title: "Processing Payment",
			message: "Your payment is being verified. This usually takes less than a minute.",
			estimatedTime: "< 1 min",
			showSpinner: true,
		};
	}
	if (s === "requested" && ps === "failed") {
		return {
			title: "Payment Failed",
			message: "Your payment could not be completed. Please try again or contact support.",
			estimatedTime: null,
			showSpinner: false,
		};
	}
	if (s === "requested" && ps === "succeeded") {
		return {
			title: "Waiting for Provider",
			message: "Your request has been sent to the provider. They will review and accept it.",
			estimatedTime: "1–24 hours",
			showSpinner: true,
		};
	}
	if (s === "pending") {
		return {
			title: "Waiting for Provider",
			message: "Your request is being reviewed by the provider.",
			estimatedTime: "1–24 hours",
			showSpinner: true,
		};
	}
	if (s === "accepted") {
		return {
			title: "Queued for Provisioning",
			message: "The provider accepted your request. Your VM is being prepared.",
			estimatedTime: "5–15 min",
			showSpinner: true,
		};
	}
	if (s === "provisioning") {
		return {
			title: "Setting Up Your VM",
			message: "Your VM is being created and configured. This includes installing the OS and setting up SSH access.",
			estimatedTime: "5–20 min",
			showSpinner: true,
		};
	}
	if (s === "provisioned") {
		return {
			title: "Almost Ready",
			message: "Your VM is provisioned and undergoing final checks.",
			estimatedTime: "1–5 min",
			showSpinner: true,
		};
	}
	if (s === "active") {
		return {
			title: "Ready to Use",
			message: "Your VM is fully provisioned and ready to use. See the connection details below.",
			estimatedTime: null,
			showSpinner: false,
		};
	}
	if (s === "rejected") {
		return {
			title: "Request Rejected",
			message: "The provider declined this request. Try a different provider.",
			estimatedTime: null,
			showSpinner: false,
		};
	}
	if (s === "failed") {
		return {
			title: "Provisioning Failed",
			message: "Something went wrong. You may request a refund or contact support.",
			estimatedTime: null,
			showSpinner: false,
		};
	}
	if (s === "cancelled") {
		return {
			title: "Cancelled",
			message: "This rental has been cancelled. If you paid, a refund will be processed automatically.",
			estimatedTime: null,
			showSpinner: false,
		};
	}

	return null;
}

export function getStageExplanation(status: string, paymentStatus?: string): string | null {
	const s = status.toLowerCase();
	const ps = paymentStatus?.toLowerCase() ?? "";

	if (s === "requested" && ps === "pending") {
		return "Your payment is being processed. This usually takes less than a minute.";
	}
	if (s === "requested" && ps === "failed") {
		return "Your payment could not be completed. Please try again or contact support.";
	}
	if (s === "requested" && ps === "succeeded") {
		return "Payment received! The provider has been notified and will review your request. You'll receive an email when they respond.";
	}
	if (s === "pending") {
		return "Your request is being reviewed by the provider. Most providers respond within a few hours.";
	}
	if (s === "accepted") {
		return "Great news! The provider accepted your request. Your VM is now queued for provisioning.";
	}
	if (s === "provisioning") {
		return "Your VM is being created. This includes allocating resources, installing the OS, and configuring SSH access.";
	}
	if (s === "provisioned" || s === "active") {
		return "Your resource is fully provisioned and ready to use. See the connection details below to get started.";
	}
	if (s === "rejected") {
		return "The provider declined this request. This can happen if the resource is no longer available or doesn't match your requirements.";
	}
	if (s === "failed") {
		return "Something went wrong during provisioning. The provider has been notified. You may request a refund or try again.";
	}
	if (s === "cancelled") {
		return "This rental has been cancelled. If you already paid, a refund will be processed automatically.";
	}

	return null;
}
