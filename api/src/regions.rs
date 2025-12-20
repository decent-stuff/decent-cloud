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
#[allow(dead_code)]
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
    let region = match country.to_uppercase().as_str() {
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
    };
    region
}

/// Check if a country code is valid (maps to a known region).
pub fn is_valid_country_code(country: &str) -> bool {
    country_to_region(country).is_some()
}

/// Get all valid country codes for a specific region.
#[allow(dead_code)]
pub fn countries_in_region(region: &str) -> Vec<&'static str> {
    // This is a bit inefficient but rarely called
    let all_countries = [
        // Europe
        "AT", "BE", "FR", "DE", "LI", "LU", "MC", "NL", "CH", "DK", "EE", "FI", "IS", "IE", "LV",
        "LT", "NO", "SE", "GB", "UK", "AD", "AL", "BA", "HR", "CY", "GR", "IT", "MT", "ME", "MK",
        "PT", "SM", "RS", "SI", "ES", "VA", "XK", "BG", "CZ", "HU", "PL", "RO", "SK",
        // CIS
        "RU", "BY", "UA", "MD", "AM", "AZ", "GE", "KZ", "KG", "TJ", "TM", "UZ", // NA
        "US", "CA", "MX", "GT", "BZ", "HN", "SV", "NI", "CR", "PA", "CU", "JM", "HT", "DO", "PR",
        "BS", "BB", "TT", "LC", "VC", "GD", "AG", "DM", "KN", "AW", "CW", "SX", "BM", "KY", "VI",
        "VG", "TC", "AI", "MS", "GP", "MQ", "MF", "BL", "GL", "PM", // LATAM
        "AR", "BO", "BR", "CL", "CO", "EC", "GY", "PY", "PE", "SR", "UY", "VE", "GF", "FK",
        // APAC
        "CN", "JP", "KR", "KP", "MN", "TW", "HK", "MO", "SG", "MY", "TH", "VN", "PH", "ID", "MM",
        "KH", "LA", "BN", "TL", "IN", "PK", "BD", "LK", "NP", "BT", "MV", "AF", "AU", "NZ", "PG",
        "FJ", "SB", "VU", "NC", "PF", "WS", "TO", "FM", "PW", "MH", "KI", "NR", "TV", "GU", "MP",
        "AS", "CK", "NU", "TK", "WF", // MENA
        "AE", "SA", "QA", "KW", "BH", "OM", "YE", "IQ", "IR", "JO", "LB", "SY", "IL", "PS", "TR",
        "EG", "LY", "TN", "DZ", "MA", "EH", // SSA
        "MR", "ML", "NE", "TD", "SD", "SS", "ER", "DJ", "SO", "ET", "KE", "UG", "RW", "BI", "TZ",
        "MZ", "MW", "ZM", "ZW", "BW", "NA", "SZ", "LS", "ZA", "MG", "MU", "SC", "KM", "RE", "YT",
        "AO", "CD", "CG", "CF", "CM", "GA", "GQ", "ST", "NG", "GH", "CI", "SN", "GM", "GN", "GW",
        "SL", "LR", "BF", "TG", "BJ", "CV",
    ];

    all_countries
        .iter()
        .filter(|c| country_to_region(c) == Some(region))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_region() {
        assert!(is_valid_region("europe"));
        assert!(is_valid_region("na"));
        assert!(is_valid_region("latam"));
        assert!(is_valid_region("apac"));
        assert!(is_valid_region("mena"));
        assert!(is_valid_region("ssa"));
        assert!(is_valid_region("cis"));

        assert!(!is_valid_region("invalid"));
        assert!(!is_valid_region(""));
        assert!(!is_valid_region("EUROPE")); // case sensitive
    }

    #[test]
    fn test_region_display_name() {
        assert_eq!(region_display_name("europe"), Some("Europe"));
        assert_eq!(region_display_name("na"), Some("North America"));
        assert_eq!(region_display_name("invalid"), None);
    }

    #[test]
    fn test_country_to_region_europe() {
        assert_eq!(country_to_region("DE"), Some("europe"));
        assert_eq!(country_to_region("de"), Some("europe")); // case insensitive
        assert_eq!(country_to_region("FR"), Some("europe"));
        assert_eq!(country_to_region("GB"), Some("europe"));
        assert_eq!(country_to_region("UK"), Some("europe"));
        assert_eq!(country_to_region("PL"), Some("europe"));
    }

    #[test]
    fn test_country_to_region_cis() {
        assert_eq!(country_to_region("RU"), Some("cis"));
        assert_eq!(country_to_region("UA"), Some("cis"));
        assert_eq!(country_to_region("KZ"), Some("cis"));
    }

    #[test]
    fn test_country_to_region_north_america() {
        assert_eq!(country_to_region("US"), Some("na"));
        assert_eq!(country_to_region("CA"), Some("na"));
        assert_eq!(country_to_region("MX"), Some("na"));
    }

    #[test]
    fn test_country_to_region_latin_america() {
        assert_eq!(country_to_region("BR"), Some("latam"));
        assert_eq!(country_to_region("AR"), Some("latam"));
        assert_eq!(country_to_region("CL"), Some("latam"));
    }

    #[test]
    fn test_country_to_region_asia_pacific() {
        assert_eq!(country_to_region("JP"), Some("apac"));
        assert_eq!(country_to_region("AU"), Some("apac"));
        assert_eq!(country_to_region("SG"), Some("apac"));
        assert_eq!(country_to_region("IN"), Some("apac"));
    }

    #[test]
    fn test_country_to_region_mena() {
        assert_eq!(country_to_region("AE"), Some("mena"));
        assert_eq!(country_to_region("SA"), Some("mena"));
        assert_eq!(country_to_region("EG"), Some("mena"));
    }

    #[test]
    fn test_country_to_region_sub_saharan_africa() {
        assert_eq!(country_to_region("ZA"), Some("ssa"));
        assert_eq!(country_to_region("NG"), Some("ssa"));
        assert_eq!(country_to_region("KE"), Some("ssa"));
    }

    #[test]
    fn test_country_to_region_unknown() {
        assert_eq!(country_to_region("XX"), None);
        assert_eq!(country_to_region(""), None);
        assert_eq!(country_to_region("ZZ"), None);
    }

    #[test]
    fn test_is_valid_country_code() {
        assert!(is_valid_country_code("US"));
        assert!(is_valid_country_code("DE"));
        assert!(is_valid_country_code("jp")); // case insensitive

        assert!(!is_valid_country_code("XX"));
        assert!(!is_valid_country_code(""));
    }

    #[test]
    fn test_countries_in_region() {
        let europe_countries = countries_in_region("europe");
        assert!(europe_countries.contains(&"DE"));
        assert!(europe_countries.contains(&"FR"));
        assert!(!europe_countries.contains(&"US"));

        let na_countries = countries_in_region("na");
        assert!(na_countries.contains(&"US"));
        assert!(na_countries.contains(&"CA"));
    }

    #[test]
    fn test_regions_constant() {
        assert_eq!(REGIONS.len(), 7);
        assert!(REGIONS.iter().any(|(code, _)| *code == "europe"));
        assert!(REGIONS.iter().any(|(code, _)| *code == "na"));
        assert!(REGIONS.iter().any(|(code, _)| *code == "latam"));
        assert!(REGIONS.iter().any(|(code, _)| *code == "apac"));
        assert!(REGIONS.iter().any(|(code, _)| *code == "mena"));
        assert!(REGIONS.iter().any(|(code, _)| *code == "ssa"));
        assert!(REGIONS.iter().any(|(code, _)| *code == "cis"));
    }
}
