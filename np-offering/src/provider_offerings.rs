use crate::csv_constants::CSV_HEADERS;
use crate::enums::*;
use crate::errors::OfferingError;
use crate::server_offering::ServerOffering;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{DecodePublicKey, EncodePublicKey};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::str::FromStr;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProviderOfferings {
    pub provider_pubkey: Vec<u8>,
    pub server_offerings: Vec<ServerOffering>,
}

impl ProviderOfferings {
    pub fn new(provider_pubkey: Vec<u8>, server_offerings: Vec<ServerOffering>) -> Self {
        Self {
            provider_pubkey,
            server_offerings,
        }
    }

    /// Get all instance IDs - backward compatibility method
    pub fn get_all_instance_ids(&self) -> Vec<String> {
        self.server_offerings
            .iter()
            .map(|offering| offering.unique_internal_identifier.clone())
            .collect()
    }

    /// Search for matches - backward compatibility method
    pub fn matches_search(&self, search_filter: &str) -> Vec<String> {
        let mut matches = Vec::new();
        for offering in &self.server_offerings {
            let offering_matches = offering.matches_search(search_filter);
            matches.extend(offering_matches);
        }
        matches
    }

    pub fn new_from_file(provider_pubkey: &[u8], path: &str) -> Result<Self, OfferingError> {
        let file = std::fs::File::open(path)?;
        Self::from_reader(provider_pubkey, file)
    }

    pub fn new_from_str(provider_pubkey: &[u8], csv_data: &str) -> Result<Self, OfferingError> {
        let cursor = std::io::Cursor::new(csv_data.as_bytes());
        Self::from_reader(provider_pubkey, cursor)
    }

    pub fn from_reader<R: Read>(provider_pubkey: &[u8], reader: R) -> Result<Self, OfferingError> {
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let mut server_offerings = Vec::new();

        for result in csv_reader.records() {
            match result {
                Ok(record) => match Self::parse_record(&record) {
                    Ok(offering) => server_offerings.push(offering),
                    Err(e) => {
                        eprintln!("Skipping invalid record: {}", e);
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("Error reading CSV record: {}", e);
                    continue;
                }
            }
        }

        Ok(Self {
            provider_pubkey: provider_pubkey.to_vec(),
            server_offerings,
        })
    }

    pub fn to_writer<W: Write>(&self, writer: W) -> Result<(), OfferingError> {
        let mut buffer = Vec::new();
        
        // Write headers first
        {
            let mut csv_writer = csv::WriterBuilder::new()
                .has_headers(false) // We handle headers manually to ensure consistency
                .from_writer(&mut buffer);
            
            csv_writer.write_record(CSV_HEADERS)?;
            
            // Use each offering's serialize method to ensure consistency and DRY principle
            for offering in &self.server_offerings {
                let offering_csv = offering.serialize()?;
                let offering_str = String::from_utf8_lossy(&offering_csv);
                
                // Parse the offering CSV to extract just the data row (skip header)
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(offering_str.as_bytes());
                    
                for result in reader.records() {
                    let record = result?;
                    csv_writer.write_record(&record)?;
                }
            }
            
            csv_writer.flush()?;
        } // csv_writer is dropped here, releasing mutable borrow on buffer
        
        // Write the final result to the output writer
        let mut output_writer = writer;
        output_writer.write_all(&buffer)?;
        output_writer.flush()?;
        
        Ok(())
    }

    pub fn to_str(&self) -> Result<String, OfferingError> {
        let mut buffer = Vec::new();
        self.to_writer(&mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    pub fn parse_record(record: &csv::StringRecord) -> Result<ServerOffering, OfferingError> {
        let get_field =
            |index: usize| -> String { record.get(index).unwrap_or("").trim().to_string() };

        let parse_optional_u32 = |index: usize| -> Option<u32> {
            let value = get_field(index);
            if value.is_empty() || value == "0" || value == "N/A" || value == "-" {
                None
            } else {
                value.parse().ok()
            }
        };

        let parse_list = |index: usize, separator: &str| -> Vec<String> {
            let value = get_field(index);
            if value.is_empty() {
                Vec::new()
            } else {
                value
                    .split(separator)
                    .map(|s| s.trim().to_string())
                    .collect()
            }
        };

        // Parse coordinates
        let coordinates = {
            let coord_str = get_field(29);
            if coord_str.is_empty() {
                None
            } else {
                let parts: Vec<&str> = coord_str.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(lat), Ok(lon)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        Some((lat, lon))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        let offering = ServerOffering {
            offer_name: get_field(0),
            description: get_field(1),
            unique_internal_identifier: get_field(2),
            product_page_url: get_field(3),
            currency: Currency::from_str(&get_field(4)).map_err(|_| {
                OfferingError::ParseError(format!("Invalid currency: {}", get_field(4)))
            })?,
            monthly_price: get_field(5).parse().map_err(|_| {
                OfferingError::ParseError(format!("Invalid monthly price: {}", get_field(5)))
            })?,
            setup_fee: get_field(6).parse().map_err(|_| {
                OfferingError::ParseError(format!("Invalid setup fee: {}", get_field(6)))
            })?,
            visibility: Visibility::from_str(&get_field(7)).map_err(|_| {
                OfferingError::ParseError(format!("Invalid visibility: {}", get_field(7)))
            })?,
            product_type: ProductType::from_str(&get_field(8)).map_err(|_| {
                OfferingError::ParseError(format!("Invalid product type: {}", get_field(8)))
            })?,
            virtualization_type: {
                let virt_type = get_field(9);
                if virt_type.is_empty() || virt_type.to_lowercase() == "none" {
                    Some(VirtualizationType::None)
                } else {
                    Some(VirtualizationType::from_str(&virt_type).map_err(|_| {
                        OfferingError::ParseError(format!(
                            "Invalid virtualization type: {}",
                            virt_type
                        ))
                    })?)
                }
            },
            billing_interval: BillingInterval::from_str(&get_field(10)).map_err(|_| {
                OfferingError::ParseError(format!("Invalid billing interval: {}", get_field(10)))
            })?,
            stock: StockStatus::from_str(&get_field(11)).map_err(|_| {
                OfferingError::ParseError(format!("Invalid stock status: {}", get_field(11)))
            })?,
            processor_brand: {
                let brand = get_field(12);
                if brand.is_empty() {
                    None
                } else {
                    Some(brand)
                }
            },
            processor_amount: parse_optional_u32(13),
            processor_cores: parse_optional_u32(14),
            processor_speed: {
                let speed = get_field(15);
                if speed.is_empty() {
                    None
                } else {
                    Some(speed)
                }
            },
            processor_name: {
                let name = get_field(16);
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            },
            memory_error_correction: {
                let ecc = get_field(17);
                if ecc.is_empty() {
                    None
                } else {
                    Some(ErrorCorrection::from_str(&ecc).map_err(|_| {
                        OfferingError::ParseError(format!("Invalid error correction: {}", ecc))
                    })?)
                }
            },
            memory_type: {
                let mem_type = get_field(18);
                if mem_type.is_empty() {
                    None
                } else {
                    Some(mem_type)
                }
            },
            memory_amount: {
                let amount = get_field(19);
                if amount.is_empty() {
                    None
                } else {
                    Some(amount)
                }
            },
            hdd_amount: parse_optional_u32(20).unwrap_or(0),
            total_hdd_capacity: {
                let capacity = get_field(21);
                if capacity.is_empty() || capacity == "0" {
                    None
                } else {
                    Some(capacity)
                }
            },
            ssd_amount: parse_optional_u32(22).unwrap_or(0),
            total_ssd_capacity: {
                let capacity = get_field(23);
                if capacity.is_empty() || capacity == "0" {
                    None
                } else {
                    Some(capacity)
                }
            },
            unmetered: parse_list(24, ","),
            uplink_speed: {
                let speed = get_field(25);
                if speed.is_empty() {
                    None
                } else {
                    Some(speed)
                }
            },
            traffic: parse_optional_u32(26),
            datacenter_country: get_field(27),
            datacenter_city: get_field(28),
            datacenter_coordinates: coordinates,
            features: parse_list(30, ","),
            operating_systems: parse_list(31, ","),
            control_panel: {
                let panel = get_field(32);
                if panel.is_empty() {
                    None
                } else {
                    Some(panel)
                }
            },
            gpu_name: {
                let gpu = get_field(33);
                if gpu.is_empty() {
                    None
                } else {
                    Some(gpu)
                }
            },
            payment_methods: parse_list(34, ","),
        };

        Ok(offering)
    }

    /// Filter offerings by criteria
    pub fn filter<F>(&self, predicate: F) -> Vec<&ServerOffering>
    where
        F: Fn(&ServerOffering) -> bool,
    {
        self.server_offerings
            .iter()
            .filter(|offering| predicate(offering))
            .collect()
    }

    /// Find offerings by name
    pub fn find_by_name(&self, name: &str) -> Vec<&ServerOffering> {
        let name_lower = name.to_lowercase();
        self.filter(|offering| offering.offer_name.to_lowercase().contains(&name_lower))
    }

    /// Find offerings by product type
    pub fn find_by_product_type(&self, product_type: &ProductType) -> Vec<&ServerOffering> {
        self.filter(|offering| {
            std::mem::discriminant(&offering.product_type) == std::mem::discriminant(product_type)
        })
    }

    /// Find offerings by price range
    pub fn find_by_price_range(&self, min_price: f64, max_price: f64) -> Vec<&ServerOffering> {
        self.filter(|offering| {
            offering.monthly_price >= min_price && offering.monthly_price <= max_price
        })
    }

    /// Find offerings by country
    pub fn find_by_country(&self, country: &str) -> Vec<&ServerOffering> {
        let country_lower = country.to_lowercase();
        self.filter(|offering| offering.datacenter_country.to_lowercase() == country_lower)
    }

    /// Find offerings with GPU
    pub fn find_with_gpu(&self) -> Vec<&ServerOffering> {
        self.filter(|offering| offering.gpu_name.is_some())
    }

    /// Serialize to PEM + CSV format (recommended for canister interfaces)
    pub fn serialize_as_pem_csv(&self) -> Result<(String, String), OfferingError> {
        // Convert pubkey to PEM
        let pubkey: [u8; 32] = self.provider_pubkey.clone().try_into().map_err(|_| {
            OfferingError::ParseError(format!(
                "Invalid provider pubkey length: {} (expected 32)",
                self.provider_pubkey.len()
            ))
        })?;

        let verifying_key = VerifyingKey::from_bytes(&pubkey)
            .map_err(|e| OfferingError::ParseError(format!("Invalid verifying key: {}", e)))?;

        let pubkey_pem = verifying_key
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| OfferingError::ParseError(format!("PEM encoding failed: {}", e)))?;

        // Convert offerings to CSV
        let csv_data = self.to_str()?;

        Ok((pubkey_pem, csv_data))
    }

    /// Deserialize from PEM + CSV format
    pub fn deserialize_from_pem_csv(
        pubkey_pem: &str,
        csv_data: &str,
    ) -> Result<Self, OfferingError> {
        // Parse PEM pubkey
        let verifying_key = VerifyingKey::from_public_key_pem(pubkey_pem)
            .map_err(|e| OfferingError::ParseError(format!("Invalid PEM key: {}", e)))?;

        let pubkey_bytes = verifying_key.to_bytes().to_vec();

        // Parse CSV data
        let provider_offerings = Self::new_from_str(&pubkey_bytes, csv_data)?;

        Ok(provider_offerings)
    }

    /// Serialize to compact format for canister responses (PEM + CSV in JSON wrapper)
    pub fn serialize_as_json(&self) -> Result<String, OfferingError> {
        let (pubkey_pem, csv_data) = self.serialize_as_pem_csv()?;

        let compact_format = serde_json::json!({
            "provider_pubkey_pem": pubkey_pem,
            "server_offerings_csv": csv_data
        });

        serde_json::to_string(&compact_format).map_err(|e| {
            OfferingError::SerializationError(format!("JSON serialization failed: {}", e))
        })
    }

    /// Deserialize from compact JSON format
    pub fn deserialize_from_json(json_str: &str) -> Result<Self, OfferingError> {
        let json_val: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            OfferingError::SerializationError(format!("JSON parsing failed: {}", e))
        })?;

        let pubkey_pem = json_val["provider_pubkey_pem"].as_str().ok_or_else(|| {
            OfferingError::ParseError("Missing provider_pubkey_pem in compact JSON".to_string())
        })?;

        let csv_data = json_val["server_offerings_csv"].as_str().ok_or_else(|| {
            OfferingError::ParseError("Missing server_offerings_csv in compact JSON".to_string())
        })?;

        Self::deserialize_from_pem_csv(pubkey_pem, csv_data)
    }
}
