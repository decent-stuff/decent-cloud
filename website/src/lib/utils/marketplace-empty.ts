/**
 * Minimal structural shape the emptiness check needs. Any object with these
 * two number fields satisfies it (e.g. the full `DashboardData` from
 * `$lib/services/dashboard-data`), so this helper stays decoupled from the
 * service layer and trivially unit-testable.
 */
export interface MarketplaceStats {
	totalProviders: number;
	totalContracts: number;
}

/**
 * A marketplace is "genuinely empty" only when there is no provider AND no
 * contract activity. Either signal alone is evidence of real activity:
 *  - `totalProviders > 0` means someone has listed an offering.
 *  - `totalContracts > 0` means a rental has happened (relevant even during a
 *    transient provider-count gap).
 *
 * Used by the landing page to swap an all-zero stats grid for an honest
 * early-access reframe instead of presenting zeros that read as a dead or
 * failed marketplace.
 */
export function marketplaceIsEmpty(data: MarketplaceStats): boolean {
	return data.totalProviders === 0 && data.totalContracts === 0;
}
