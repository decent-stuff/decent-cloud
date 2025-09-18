export const providerLabelVariants = {
  register: ["ProvRegister", "NPRegister"] as const,
  checkIn: ["ProvCheckIn", "NPCheckIn"] as const,
  offering: ["ProvOffering", "NPOffering"] as const,
  profile: ["ProvProfile", "NPProfile"] as const,
} as const;

type LabelMatcher = (label: unknown) => boolean;

const createMatcher = (labels: readonly string[]): LabelMatcher => {
  const set = new Set(labels);
  return (label: unknown): boolean => typeof label === "string" && set.has(label);
};

export const providerLabelMatchers = {
  register: createMatcher(providerLabelVariants.register),
  checkIn: createMatcher(providerLabelVariants.checkIn),
  offering: createMatcher(providerLabelVariants.offering),
  profile: createMatcher(providerLabelVariants.profile),
} as const;
