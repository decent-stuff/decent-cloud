import {
  isProviderCheckInLabel,
  isProviderOfferingLabel,
  isProviderRegisterLabel,
  isProviderProfileLabel,
  providerLabelVariants,
} from "../ledger-labels";

describe("ledger label helpers", () => {
  it("matches both current and legacy provider offering labels", () => {
    for (const label of providerLabelVariants.offering) {
      expect(isProviderOfferingLabel(label)).toBe(true);
    }
  });

  it("rejects unrelated labels for offerings", () => {
    expect(isProviderOfferingLabel("SomethingElse")).toBe(false);
    expect(isProviderOfferingLabel(undefined)).toBe(false);
  });

  it("matches provider check-in labels across versions", () => {
    for (const label of providerLabelVariants.checkIn) {
      expect(isProviderCheckInLabel(label)).toBe(true);
    }
  });

  it("matches provider registration labels across versions", () => {
    for (const label of providerLabelVariants.register) {
      expect(isProviderRegisterLabel(label)).toBe(true);
    }
  });

  it("matches provider profile labels across versions", () => {
    for (const label of providerLabelVariants.profile) {
      expect(isProviderProfileLabel(label)).toBe(true);
    }
  });
});
