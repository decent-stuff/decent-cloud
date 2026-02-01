//! Geographic region definitions and country-to-region mapping.
//!
//! This module re-exports region definitions from `dcc-common` to avoid duplication.
//!
//! Provides:
//! - `REGIONS`: All supported region identifiers with display names
//! - `country_to_region()`: Maps ISO 3166-1 alpha-2 country codes to regions
//! - `is_valid_region()`: Validates region identifiers
//! - `is_valid_country_code()`: Validates country codes

pub use dcc_common::regions::*;
