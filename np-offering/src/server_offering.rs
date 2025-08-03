use crate::csv_constants::CSV_HEADERS;
use crate::enums::*;
use crate::errors::OfferingError;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub fn get_unique_instance_id(&self) -> String {
        self.unique_internal_identifier.clone()
    }

    /// Convert struct to IndexMap for automatic field processing
    /// This method automatically includes ALL fields without manual enumeration
    fn to_field_map(&self) -> Result<IndexMap<String, Value>, OfferingError> {
        let json_value = serde_json::to_value(self)?;
        let obj = json_value.as_object().ok_or_else(|| {
            OfferingError::SerializationError("Failed to convert to object".to_string())
        })?;
        Ok(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    /// Search for matches in this offering based on the given filter
    /// Automatically searches through ALL struct fields
    pub fn matches_search(&self, search_filter: &str) -> Vec<String> {
        let fields = match self.to_field_map() {
            Ok(fields) => fields,
            Err(_) => return vec![],
        };

        let search_lower = search_filter.to_lowercase();
        fields
            .into_iter()
            .flat_map(|(field_name, value)| Self::search_value(&field_name, &value, &search_lower))
            .collect()
    }

    /// Recursively search through a JSON value for the search term
    fn search_value(field_name: &str, value: &Value, search_term: &str) -> Vec<String> {
        match value {
            Value::String(s) => {
                if s.to_lowercase().contains(search_term) {
                    vec![format!("{}: {}", field_name, s)]
                } else {
                    vec![]
                }
            }
            Value::Number(n) => {
                let s = n.to_string();
                if s.to_lowercase().contains(search_term) {
                    vec![format!("{}: {}", field_name, s)]
                } else {
                    vec![]
                }
            }
            Value::Array(arr) => arr
                .iter()
                .flat_map(|item| Self::search_value(field_name, item, search_term))
                .collect(),
            Value::Object(obj) => obj
                .iter()
                .flat_map(|(key, val)| {
                    let nested_field = format!("{}_{}", field_name, key);
                    Self::search_value(&nested_field, val, search_term)
                })
                .collect(),
            Value::Bool(b) => {
                let s = b.to_string();
                if s.to_lowercase().contains(search_term) {
                    vec![format!("{}: {}", field_name, s)]
                } else {
                    vec![]
                }
            }
            Value::Null => vec![],
        }
    }

    /// Get instance pricing information
    pub fn instance_pricing(&self, _instance_id: &str) -> HashMap<String, HashMap<String, String>> {
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
    /// Automatically handles ALL struct fields using CSV_HEADERS order
    pub fn serialize(&self) -> Result<Vec<u8>, OfferingError> {
        let mut wtr = csv::Writer::from_writer(Vec::new());
        wtr.write_record(CSV_HEADERS)?;

        let fields = self.to_field_map()?;

        // Extract field values in CSV_HEADERS order - this ensures compilation errors
        // if headers don't match field names (converted to snake_case)
        let record: Result<Vec<String>, OfferingError> = CSV_HEADERS
            .iter()
            .map(|header| Self::extract_csv_value(&fields, header))
            .collect();

        wtr.write_record(&record?)?;
        wtr.into_inner()
            .map_err(|e| OfferingError::IoError(e.into_error()))
    }

    /// Extract a CSV value from the field map, handling type conversion and formatting
    fn extract_csv_value(
        fields: &IndexMap<String, Value>,
        header: &str,
    ) -> Result<String, OfferingError> {
        // Convert CSV header to field name (handle case differences)
        let field_name = Self::header_to_field_name(header);

        let value = fields.get(&field_name).ok_or_else(|| {
            OfferingError::SerializationError(format!(
                "Field '{}' (from header '{}') not found in struct",
                field_name, header
            ))
        })?;

        Ok(Self::value_to_csv_string(value))
    }

    /// Convert CSV header to struct field name
    fn header_to_field_name(header: &str) -> String {
        match header {
            "Offer Name" => "offer_name".to_string(),
            "Description" => "description".to_string(),
            "Unique Internal identifier" => "unique_internal_identifier".to_string(),
            "Product page URL" => "product_page_url".to_string(),
            "Currency" => "currency".to_string(),
            "Monthly price" => "monthly_price".to_string(),
            "Setup fee" => "setup_fee".to_string(),
            "Visibility" => "visibility".to_string(),
            "Product Type" => "product_type".to_string(),
            "Virtualization type" => "virtualization_type".to_string(),
            "Billing interval" => "billing_interval".to_string(),
            "Stock" => "stock".to_string(),
            "Processor Brand" => "processor_brand".to_string(),
            "Processor Amount" => "processor_amount".to_string(),
            "Processor Cores" => "processor_cores".to_string(),
            "Processor Speed" => "processor_speed".to_string(),
            "Processor Name" => "processor_name".to_string(),
            "Memory Error Correction" => "memory_error_correction".to_string(),
            "Memory Type" => "memory_type".to_string(),
            "Memory Amount" => "memory_amount".to_string(),
            "Hard Disk Drive Amount" => "hdd_amount".to_string(),
            "Total Hard Disk Drive Capacity" => "total_hdd_capacity".to_string(),
            "Solid State Disk Amount" => "ssd_amount".to_string(),
            "Total Solid State Disk Capacity" => "total_ssd_capacity".to_string(),
            "Unmetered" => "unmetered".to_string(),
            "Uplink speed" => "uplink_speed".to_string(),
            "Traffic" => "traffic".to_string(),
            "Datacenter Country" => "datacenter_country".to_string(),
            "Datacenter City" => "datacenter_city".to_string(),
            "Datacenter Coordinates" => "datacenter_coordinates".to_string(),
            "Features" => "features".to_string(),
            "Operating Systems" => "operating_systems".to_string(),
            "Control Panel" => "control_panel".to_string(),
            "GPU Name" => "gpu_name".to_string(),
            "Payment Methods" => "payment_methods".to_string(),
            _ => header.to_lowercase().replace(' ', "_"),
        }
    }

    /// Convert a JSON value to CSV string representation
    fn value_to_csv_string(value: &Value) -> String {
        match value {
            Value::String(s) => {
                // Handle special enum cases that need CSV-specific formatting
                match s.as_str() {
                    "InStock" => "In stock".to_string(),
                    "OutOfStock" => "Out of stock".to_string(),
                    "Limited" => "Limited".to_string(),
                    _ => s.clone(),
                }
            }
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => arr
                .iter()
                .map(Self::value_to_csv_string)
                .collect::<Vec<_>>()
                .join(", "),
            Value::Object(obj) => {
                if let (Some(lat), Some(lon)) = (obj.get("0"), obj.get("1")) {
                    // Handle coordinate tuple: (f64, f64)
                    format!(
                        "{},{}",
                        Self::value_to_csv_string(lat),
                        Self::value_to_csv_string(lon)
                    )
                } else {
                    // Handle other objects by joining key-value pairs
                    obj.iter()
                        .map(|(k, v)| format!("{}:{}", k, Self::value_to_csv_string(v)))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            }
            Value::Null => String::new(),
        }
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
