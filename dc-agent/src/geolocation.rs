//! IP geolocation detection for agent registration.
//!
//! Detects the agent's geographic location based on its public IP address
//! and compares it against the pool's expected location.

use anyhow::{Context, Result};

/// Detect the agent's country code using public IP geolocation.
pub async fn detect_country() -> Result<Option<String>> {
    let lookup = public_ip_address::perform_lookup(None)
        .await
        .context("Failed to perform IP geolocation lookup")?;

    Ok(lookup.country_code)
}

/// Map ISO 3166-1 alpha-2 country code to region identifier.
/// Returns None for unknown country codes.
pub fn country_to_region(country: &str) -> Option<&'static str> {
    match country.to_uppercase().as_str() {
        // Europe (geographic, not EU political union)
        // Western Europe
        "AT" | "BE" | "FR" | "DE" | "LI" | "LU" | "MC" | "NL" | "CH" |
        // Northern Europe
        "DK" | "EE" | "FI" | "IS" | "IE" | "LV" | "LT" | "NO" | "SE" | "GB" | "UK" |
        // Southern Europe
        "AD" | "AL" | "BA" | "HR" | "CY" | "GR" | "IT" | "MT" | "ME" | "MK" | "PT" | "SM"
        | "RS" | "SI" | "ES" | "VA" | "XK" |
        // Eastern Europe (non-CIS)
        "BG" | "CZ" | "HU" | "PL" | "RO" | "SK" => Some("europe"),

        // CIS - Commonwealth of Independent States and associated
        "RU" | "BY" | "UA" | "MD" | "AM" | "AZ" | "GE" | "KZ" | "KG" | "TJ" | "TM" | "UZ" => Some("cis"),

        // North America (USA, Canada, Mexico, Central America, Caribbean)
        "US" | "CA" | "MX" | "GT" | "BZ" | "HN" | "SV" | "NI" | "CR" | "PA" | "CU" | "JM"
        | "HT" | "DO" | "PR" | "BS" | "BB" | "TT" | "LC" | "VC" | "GD" | "AG" | "DM" | "KN"
        | "AW" | "CW" | "SX" | "BM" | "KY" | "VI" | "VG" | "TC" | "AI" | "MS" | "GP" | "MQ"
        | "MF" | "BL" | "GL" | "PM" => Some("na"),

        // Latin America (South America + Central America Spanish/Portuguese speaking)
        "AR" | "BO" | "BR" | "CL" | "CO" | "EC" | "GY" | "PY" | "PE" | "SR" | "UY" | "VE"
        | "GF" | "FK" => Some("latam"),

        // Asia Pacific (East Asia, Southeast Asia, South Asia, Oceania)
        "CN" | "JP" | "KR" | "KP" | "MN" | "TW" | "HK" | "MO" | "SG" | "MY" | "TH" | "VN"
        | "PH" | "ID" | "MM" | "KH" | "LA" | "BN" | "TL" | "IN" | "PK" | "BD" | "LK" | "NP"
        | "BT" | "MV" | "AF" | "AU" | "NZ" | "PG" | "FJ" | "SB" | "VU" | "NC" | "PF" | "WS"
        | "TO" | "FM" | "PW" | "MH" | "KI" | "NR" | "TV" | "GU" | "MP" | "AS" | "CK" | "NU"
        | "TK" | "WF" => Some("apac"),

        // MENA - Middle East and North Africa
        "AE" | "SA" | "QA" | "KW" | "BH" | "OM" | "YE" | "IQ" | "IR" | "JO" | "LB" | "SY"
        | "IL" | "PS" | "TR" | "EG" | "LY" | "TN" | "DZ" | "MA" | "EH" => Some("mena"),

        // Sub-Saharan Africa
        "MR" | "ML" | "NE" | "TD" | "SD" | "SS" | "ER" | "DJ" | "SO" | "ET" | "KE" | "UG"
        | "RW" | "BI" | "TZ" | "MZ" | "MW" | "ZM" | "ZW" | "BW" | "NA" | "SZ" | "LS" | "ZA"
        | "MG" | "MU" | "SC" | "KM" | "RE" | "YT" | "AO" | "CD" | "CG" | "CF" | "CM" | "GA"
        | "GQ" | "ST" | "NG" | "GH" | "CI" | "SN" | "GM" | "GN" | "GW" | "SL" | "LR" | "BF"
        | "TG" | "BJ" | "CV" => Some("ssa"),

        // Unknown country codes
        _ => None,
    }
}

/// Region display names.
pub const REGIONS: &[(&str, &str)] = &[
    ("europe", "Europe"),
    ("na", "North America"),
    ("latam", "Latin America"),
    ("apac", "Asia Pacific"),
    ("mena", "Middle East & North Africa"),
    ("ssa", "Sub-Saharan Africa"),
    ("cis", "CIS (Russia & neighbors)"),
];

/// Get display name for a region.
pub fn region_display_name(region: &str) -> Option<&'static str> {
    REGIONS
        .iter()
        .find(|(code, _)| *code == region)
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country_to_region_europe() {
        assert_eq!(country_to_region("DE"), Some("europe"));
        assert_eq!(country_to_region("de"), Some("europe"));
        assert_eq!(country_to_region("FR"), Some("europe"));
        assert_eq!(country_to_region("PL"), Some("europe"));
    }

    #[test]
    fn test_country_to_region_north_america() {
        assert_eq!(country_to_region("US"), Some("na"));
        assert_eq!(country_to_region("CA"), Some("na"));
    }

    #[test]
    fn test_country_to_region_asia_pacific() {
        assert_eq!(country_to_region("JP"), Some("apac"));
        assert_eq!(country_to_region("AU"), Some("apac"));
        assert_eq!(country_to_region("SG"), Some("apac"));
    }

    #[test]
    fn test_country_to_region_unknown() {
        assert_eq!(country_to_region("XX"), None);
        assert_eq!(country_to_region(""), None);
    }

    #[test]
    fn test_region_display_name() {
        assert_eq!(region_display_name("europe"), Some("Europe"));
        assert_eq!(region_display_name("na"), Some("North America"));
        assert_eq!(region_display_name("invalid"), None);
    }
}
