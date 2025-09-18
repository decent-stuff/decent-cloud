const PROVIDER_LABEL_VARIANTS = {
  register: ["ProvRegister", "NPRegister"] as const,
  checkIn: ["ProvCheckIn", "NPCheckIn"] as const,
  offering: ["ProvOffering", "NPOffering"] as const,
  profile: ["ProvProfile", "NPProfile"] as const,
} as const;

type ProviderLabelVariant = typeof PROVIDER_LABEL_VARIANTS;
type ProviderLabelKey = keyof ProviderLabelVariant;

type LabelMatcher = (label: unknown) => boolean;

function createLabelMatcher(labels: readonly string[]): LabelMatcher {
  const labelSet = new Set(labels);
  return (label: unknown): boolean =>
    typeof label === "string" && labelSet.has(label);
}

export const providerLabelVariants: {
  readonly [K in ProviderLabelKey]: readonly string[];
} = {
  register: [...PROVIDER_LABEL_VARIANTS.register],
  checkIn: [...PROVIDER_LABEL_VARIANTS.checkIn],
  offering: [...PROVIDER_LABEL_VARIANTS.offering],
  profile: [...PROVIDER_LABEL_VARIANTS.profile],
} as const;

export const isProviderRegisterLabel = createLabelMatcher(
  providerLabelVariants.register
);

export const isProviderCheckInLabel = createLabelMatcher(
  providerLabelVariants.checkIn
);

export const isProviderOfferingLabel = createLabelMatcher(
  providerLabelVariants.offering
);

export const isProviderProfileLabel = createLabelMatcher(
  providerLabelVariants.profile
);

export type ProviderLabelMatchers = {
  register: LabelMatcher;
  checkIn: LabelMatcher;
  offering: LabelMatcher;
  profile: LabelMatcher;
};

export const providerLabelMatchers: ProviderLabelMatchers = {
  register: isProviderRegisterLabel,
  checkIn: isProviderCheckInLabel,
  offering: isProviderOfferingLabel,
  profile: isProviderProfileLabel,
};
