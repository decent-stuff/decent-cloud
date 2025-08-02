use crate::enums::*;
use crate::errors::OfferingError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server offering similar to the CSV format of serverhunter.com
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerOffering {
    pub offer_name: String,
    pub description: String,
    pub unique_internal_identifier: String,
    pub product_page_url: String,
    pub currency: Currency,
    pub monthly_price: f64,
    pub setup_fee: f64,
    pub visibility: Visibility,
    pub product_type: ProductType,
    pub virtualization_type: Option<VirtualizationType>,
    pub billing_interval: BillingInterval,
    pub stock: StockStatus,
    pub processor_brand: Option<String>,
    pub processor_amount: Option<u32>,
    pub processor_cores: Option<u32>,
    pub processor_speed: Option<String>,
    pub processor_name: Option<String>,
    pub memory_error_correction: Option<ErrorCorrection>,
    pub memory_type: Option<String>,
    pub memory_amount: Option<String>,
    pub hdd_amount: u32,
    pub total_hdd_capacity: Option<String>,
    pub ssd_amount: u32,
    pub total_ssd_capacity: Option<String>,
    pub unmetered: Vec<String>,
    pub uplink_speed: Option<String>,
    pub traffic: Option<u32>,
    pub datacenter_country: String,
    pub datacenter_city: String,
    pub datacenter_coordinates: Option<(f64, f64)>,
    pub features: Vec<String>,
    pub operating_systems: Vec<String>,
    pub control_panel: Option<String>,
    pub gpu_name: Option<String>,
    pub payment_methods: Vec<String>,
}

impl ServerOffering {
    /// Get all instance IDs for this offering
    /// Returns a vector containing the unique internal identifier
    pub fn get_all_instance_ids(&self) -> Vec<String> {
        vec![self.unique_internal_identifier.clone()]
    }

    /// Search for matches in this offering based on the given filter
    /// Returns matching field names/values
    pub fn matches_search(&self, search_filter: &str) -> Vec<String> {
        let mut matches = Vec::new();
        let search_lower = search_filter.to_lowercase();

        // Search in various string fields
        if self.offer_name.to_lowercase().contains(&search_lower) {
            matches.push(format!("offer_name: {}", self.offer_name));
        }
        if self.description.to_lowercase().contains(&search_lower) {
            matches.push(format!("description: {}", self.description));
        }
        if self
            .unique_internal_identifier
            .to_lowercase()
            .contains(&search_lower)
        {
            matches.push(format!(
                "unique_internal_identifier: {}",
                self.unique_internal_identifier
            ));
        }
        if self
            .datacenter_country
            .to_lowercase()
            .contains(&search_lower)
        {
            matches.push(format!("datacenter_country: {}", self.datacenter_country));
        }
        if self.datacenter_city.to_lowercase().contains(&search_lower) {
            matches.push(format!("datacenter_city: {}", self.datacenter_city));
        }

        // Search in optional string fields
        if let Some(ref processor_brand) = self.processor_brand {
            if processor_brand.to_lowercase().contains(&search_lower) {
                matches.push(format!("processor_brand: {}", processor_brand));
            }
        }
        if let Some(ref processor_name) = self.processor_name {
            if processor_name.to_lowercase().contains(&search_lower) {
                matches.push(format!("processor_name: {}", processor_name));
            }
        }
        if let Some(ref memory_type) = self.memory_type {
            if memory_type.to_lowercase().contains(&search_lower) {
                matches.push(format!("memory_type: {}", memory_type));
            }
        }
        if let Some(ref control_panel) = self.control_panel {
            if control_panel.to_lowercase().contains(&search_lower) {
                matches.push(format!("control_panel: {}", control_panel));
            }
        }
        if let Some(ref gpu_name) = self.gpu_name {
            if gpu_name.to_lowercase().contains(&search_lower) {
                matches.push(format!("gpu_name: {}", gpu_name));
            }
        }

        // Search in arrays
        for feature in &self.features {
            if feature.to_lowercase().contains(&search_lower) {
                matches.push(format!("features: {}", feature));
            }
        }
        for os in &self.operating_systems {
            if os.to_lowercase().contains(&search_lower) {
                matches.push(format!("operating_systems: {}", os));
            }
        }
        for payment in &self.payment_methods {
            if payment.to_lowercase().contains(&search_lower) {
                matches.push(format!("payment_methods: {}", payment));
            }
        }

        matches
    }

    /// Get instance pricing information
    /// Returns a map of pricing models to time units to prices
    pub fn instance_pricing(
        &self,
        _instance_id: &str,
    ) -> HashMap<String, HashMap<String, String>> {
        let mut pricing = HashMap::new();
        let mut units = HashMap::new();

        // Convert monthly price to different time units
        units.insert("month".to_string(), self.monthly_price.to_string());
        units.insert("year".to_string(), (self.monthly_price * 12.0).to_string());
        units.insert("day".to_string(), (self.monthly_price / 30.0).to_string());
        units.insert(
            "hour".to_string(),
            (self.monthly_price / (30.0 * 24.0)).to_string(),
        );

        pricing.insert("on_demand".to_string(), units);
        pricing
    }

    /// Serialize the ServerOffering to CSV bytes
    pub fn serialize(&self) -> Result<Vec<u8>, OfferingError> {
        let mut wtr = csv::Writer::from_writer(Vec::new());

        // Write CSV headers first
        let headers = vec![
            "Offer Name",
            "Description",
            "Unique Internal identifier",
            "Product page URL",
            "Currency",
            "Monthly price",
            "Setup fee",
            "Visibility",
            "Product Type",
            "Virtualization type",
            "Billing interval",
            "Stock",
            "Processor Brand",
            "Processor Amount",
            "Processor Cores",
            "Processor Speed",
            "Processor Name",
            "Memory Error Correction",
            "Memory Type",
            "Memory Amount",
            "Hard Disk Drive Amount",
            "Total Hard Disk Drive Capacity",
            "Solid State Disk Amount",
            "Total Solid State Disk Capacity",
            "Unmetered",
            "Uplink speed",
            "Traffic",
            "Datacenter Country",
            "Datacenter City",
            "Datacenter Coordinates",
            "Features",
            "Operating Systems",
            "Control Panel",
            "GPU Name",
            "Payment Methods",
        ];
        wtr.write_record(&headers)?;

        // Build the record as a vector of strings to handle Vec fields properly
        let record = vec![
            self.offer_name.clone(),
            self.description.clone(),
            self.unique_internal_identifier.clone(),
            self.product_page_url.clone(),
            format!("{}", self.currency),
            self.monthly_price.to_string(),
            self.setup_fee.to_string(),
            format!("{}", self.visibility),
            format!("{}", self.product_type),
            self.virtualization_type
                .as_ref()
                .map(|v| format!("{}", v))
                .unwrap_or_default(),
            format!("{}", self.billing_interval),
            format!("{}", self.stock)
                .replace("InStock", "In stock")
                .replace("OutOfStock", "Out of stock"),
            self.processor_brand.as_deref().unwrap_or("").to_string(),
            self.processor_amount.unwrap_or(0).to_string(),
            self.processor_cores.unwrap_or(0).to_string(),
            self.processor_speed.as_deref().unwrap_or("").to_string(),
            self.processor_name.as_deref().unwrap_or("").to_string(),
            self.memory_error_correction
                .as_ref()
                .map(|v| format!("{}", v))
                .unwrap_or_default(),
            self.memory_type.as_deref().unwrap_or("").to_string(),
            self.memory_amount.as_deref().unwrap_or("").to_string(),
            self.hdd_amount.to_string(),
            self.total_hdd_capacity.as_deref().unwrap_or("").to_string(),
            self.ssd_amount.to_string(),
            self.total_ssd_capacity.as_deref().unwrap_or("").to_string(),
            self.unmetered.join(", "),
            self.uplink_speed.as_deref().unwrap_or("").to_string(),
            self.traffic.unwrap_or(0).to_string(),
            self.datacenter_country.clone(),
            self.datacenter_city.clone(),
            self.datacenter_coordinates
                .map(|(lat, lon)| format!("{},{}", lat, lon))
                .unwrap_or_default(),
            self.features.join(", "),
            self.operating_systems.join(", "),
            self.control_panel.as_deref().unwrap_or("").to_string(),
            self.gpu_name.as_deref().unwrap_or("").to_string(),
            self.payment_methods.join(", "),
        ];

        wtr.write_record(&record)?;
        wtr.into_inner()
            .map_err(|e| OfferingError::IoError(e.into_error()))
    }
}

impl std::fmt::Display for ServerOffering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(json_str) => write!(f, "{}", json_str),
            Err(e) => {
                write!(f, "Failed to format ServerOffering: {}", e)?;
                Err(std::fmt::Error)
            }
        }
    }
}