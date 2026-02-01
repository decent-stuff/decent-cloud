//! Geographic region definitions and country-to-region mapping.
//!
//! Provides:
//! - `REGIONS`: All supported region identifiers with display names
//! - `country_to_region()`: Maps ISO 3166-1 alpha-2 country codes to regions
//! - `is_valid_region()`: Validates region identifiers
//! - `is_valid_country_code()`: Validates country codes

/// All supported region identifiers with display names.
/// Used for validation and UI display.
pub const REGIONS: &[(&str, &str)] = &[
    ("europe", "Europe"),
    ("na", "North America"),
    ("latam", "Latin America"),
    ("apac", "Asia Pacific"),
    ("mena", "Middle East & North Africa"),
    ("ssa", "Sub-Saharan Africa"),
    ("cis", "CIS (Russia & neighbors)"),
];

/// Check if a region identifier is valid.
pub fn is_valid_region(region: &str) -> bool {
    REGIONS.iter().any(|(code, _)| *code == region)
}

/// Get the display name for a region identifier.
pub fn region_display_name(region: &str) -> Option<&'static str> {
    REGIONS
        .iter()
        .find(|(code, _)| *code == region)
        .map(|(_, name)| *name)
}

/// Normalize country code to region identifier.
/// Returns a region string that can be used for location matching.
/// Covers all ISO 3166-1 alpha-2 country codes.
///
/// Returns `None` for unknown country codes.
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
        "RU" | "BY" | "UA" | "MD" | "AM" | "AZ" | "GE" | "KZ" | "KG" | "TJ" | "TM" | "UZ" => {
            Some("cis")
        }

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

        // Middle East & North Africa
        // Middle East
        "SA" | "AE" | "QA" | "KW" | "BH" | "OM" | "IR" | "IQ" | "SY" | "JO" | "LB" | "PS"
        | "IL" | "YE" | "TR" | "EG" | "DZ" | "MA" | "TN" | "LY" | "SD" | "EH" | "MR"
        | "ML" | "NE" | "DJ" | "ER" | "ET" | "SO" | "KE" | "UG" | "TZ" | "RW" | "BI"
        | "CD" | "CG" | "GA" | "CM" | "CF" | "AO" => Some("mena"),

        // Sub-Saharan Africa
        "TD" | "SS" | "NA" | "BW" | "ZW" | "ZM" | "MW" | "MZ" | "SZ" | "LS" => Some("ssa"),

        _ => None,
    }
}

/// Validate a country code against ISO 3166-1 alpha-2 format.
pub fn is_valid_country_code(code: &str) -> bool {
    code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_identifiers() {
        assert!(is_valid_region("europe"));
        assert!(is_valid_region("na"));
        assert!(is_valid_region("latam"));
        assert!(is_valid_region("apac"));
        assert!(is_valid_region("mena"));
        assert!(is_valid_region("ssa"));
        assert!(is_valid_region("cis"));
        assert!(!is_valid_region("invalid"));
        assert!(!is_valid_region("Europe")); // case-sensitive
    }

    #[test]
    fn test_region_display_names() {
        assert_eq!(region_display_name("europe"), Some("Europe"));
        assert_eq!(region_display_name("na"), Some("North America"));
        assert_eq!(region_display_name("latam"), Some("Latin America"));
        assert_eq!(region_display_name("apac"), Some("Asia Pacific"));
        assert_eq!(region_display_name("mena"), Some("Middle East & North Africa"));
        assert_eq!(region_display_name("ssa"), Some("Sub-Saharan Africa"));
        assert_eq!(
            region_display_name("cis"),
            Some("CIS (Russia & neighbors)")
        );
        assert_eq!(region_display_name("invalid"), None);
    }

    #[test]
    fn test_country_to_region_europe() {
        assert_eq!(country_to_region("US"), Some("na"));
        assert_eq!(country_to_region("CA"), Some("na"));
        assert_eq!(country_to_region("MX"), Some("na"));
        assert_eq!(country_to_region("BR"), Some("latam"));
        assert_eq!(country_to_region("AR"), Some("latam"));
        assert_eq!(country_to_region("DE"), Some("europe"));
        assert_eq!(country_to_region("FR"), Some("europe"));
        assert_eq!(country_to_region("GB"), Some("europe"));
        assert_eq!(country_to_region("UK"), Some("europe")); // Alias
        assert_eq!(country_to_region("RU"), Some("cis"));
    }

    #[test]
    fn test_country_to_region_asia_pacific() {
        assert_eq!(country_to_region("JP"), Some("apac"));
        assert_eq!(country_to_region("CN"), Some("apac"));
        assert_eq!(country_to_region("AU"), Some("apac"));
        assert_eq!(country_to_region("NZ"), Some("apac"));
        assert_eq!(country_to_region("IN"), Some("apac"));
    }

    #[test]
    fn test_country_to_region_case_insensitive() {
        assert_eq!(country_to_region("us"), Some("na"));
        assert_eq!(country_to_region("US"), Some("na"));
        assert_eq!(country_to_region("Us"), Some("na"));
        assert_eq!(country_to_region("de"), Some("europe"));
        assert_eq!(country_to_region("DE"), Some("europe"));
    }

    #[test]
    fn test_country_to_region_unknown() {
        assert_eq!(country_to_region("XX"), None);
        assert_eq!(country_to_region("ZZ"), None);
        assert_eq!(country_to_region("123"), None);
    }

    #[test]
    fn test_valid_country_code() {
        assert!(is_valid_country_code("US"));
        assert!(is_valid_country_code("GB"));
        assert!(is_valid_country_code("DE"));
        assert!(!is_valid_country_code("USA")); // Too long
        assert!(!is_valid_country_code("U")); // Too short
        assert!(!is_valid_country_code("12")); // Numbers
        assert!(!is_valid_country_code("us")); // Lowercase
    }
}
