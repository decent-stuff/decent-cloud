use crate::errors::OfferingError;
use crate::server_offering::ServerOffering;
use crate::types::{Offering, OfferingFilter, OfferingKey, ProviderPubkey, SearchQuery};
use crate::legacy::ProviderOfferings;
use std::collections::{HashMap, HashSet};

/// Central registry for all offerings with efficient search
pub struct OfferingRegistry {
    // Primary index: (provider, key) -> Offering
    offerings: HashMap<(ProviderPubkey, OfferingKey), Offering>,
    // Provider index: provider -> set of offering keys
    by_provider: HashMap<ProviderPubkey, HashSet<OfferingKey>>,
    // Text search index: keyword -> set of (provider, key)
    text_index: HashMap<String, HashSet<(ProviderPubkey, OfferingKey)>>,
}

impl Default for OfferingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OfferingRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            offerings: HashMap::new(),
            by_provider: HashMap::new(),
            text_index: HashMap::new(),
        }
    }
    
    /// Get a specific offering by provider and key - O(1)
    pub fn get_offering(&self, provider: &ProviderPubkey, key: &str) -> Option<&Offering> {
        self.offerings.get(&(provider.clone(), key.to_string()))
    }
    
    /// Get all offerings from a specific provider
    pub fn get_provider_offerings(&self, provider: &ProviderPubkey) -> Vec<&Offering> {
        self.by_provider
            .get(provider)
            .map(|keys| {
                keys.iter()
                    .filter_map(|key| self.offerings.get(&(provider.clone(), key.clone())))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Add provider offerings from CSV data
    pub fn add_provider_offerings(
        &mut self,
        provider: ProviderPubkey,
        csv_data: &str,
    ) -> Result<usize, OfferingError> {
        let server_offerings = Self::parse_csv_data(csv_data)?;
        let count = server_offerings.len();
        
        // Remove existing offerings for this provider
        self.remove_provider(&provider);
        
        // Add new offerings
        for server_offering in server_offerings {
            let offering = Offering::new(provider.clone(), server_offering);
            self.add_offering(offering);
        }
        
        Ok(count)
    }
    
    /// Update provider offerings (same as add - replaces existing)
    pub fn update_provider_offerings(
        &mut self,
        provider: ProviderPubkey,
        csv_data: &str,
    ) -> Result<usize, OfferingError> {
        self.add_provider_offerings(provider, csv_data)
    }
    
    /// Remove all offerings from a provider
    pub fn remove_provider(&mut self, provider: &ProviderPubkey) -> usize {
        let keys_to_remove = self.by_provider.remove(provider).unwrap_or_default();
        let count = keys_to_remove.len();
        
        // Remove from primary index
        for key in &keys_to_remove {
            self.offerings.remove(&(provider.clone(), key.clone()));
        }
        
        // Remove from text index
        for keyword_set in self.text_index.values_mut() {
            keyword_set.retain(|(p, k)| p != provider || !keys_to_remove.contains(k));
        }
        
        // Clean up empty keyword entries
        self.text_index.retain(|_, set| !set.is_empty());
        
        count
    }
    
    /// Search offerings using a query
    pub fn search(&self, query: &SearchQuery) -> Vec<&Offering> {
        // Direct lookup if both provider and key specified
        if let (Some(provider), Some(key)) = (&query.provider_pubkey, &query.offering_key) {
            return self.get_offering(provider, key).into_iter().collect();
        }
        
        // Start with all offerings and filter
        let mut candidates: HashSet<(ProviderPubkey, OfferingKey)> = if let Some(provider) = &query.provider_pubkey {
            // Limit to specific provider
            self.by_provider
                .get(provider)
                .map(|keys| keys.iter().map(|k| (provider.clone(), k.clone())).collect())
                .unwrap_or_default()
        } else {
            // All offerings
            self.offerings.keys().cloned().collect()
        };
        
        // Apply text filter
        if let Some(text) = &query.text_filter {
            let text_candidates = self.search_text_internal(text);
            candidates.retain(|key| text_candidates.contains(key));
        }
        
        // Convert to offerings and apply structured filters
        let mut results: Vec<&Offering> = candidates
            .iter()
            .filter_map(|(p, k)| self.offerings.get(&(p.clone(), k.clone())))
            .filter(|offering| self.matches_filters(offering, &query.filters))
            .collect();
        
        // Apply pagination
        if let Some(offset) = query.offset {
            if offset < results.len() {
                results = results.into_iter().skip(offset).collect();
            } else {
                results.clear();
            }
        }
        
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        
        results
    }
    
    /// Simple text search
    pub fn search_text(&self, text: &str) -> Vec<&Offering> {
        let candidates = self.search_text_internal(text);
        candidates
            .iter()
            .filter_map(|(p, k)| self.offerings.get(&(p.clone(), k.clone())))
            .collect()
    }
    
    /// Get total number of offerings
    pub fn count(&self) -> usize {
        self.offerings.len()
    }
    
    /// Get number of providers
    pub fn provider_count(&self) -> usize {
        self.by_provider.len()
    }
    
    // Private helper methods
    
    fn add_offering(&mut self, offering: Offering) {
        let provider = offering.provider().clone();
        let key = offering.key().to_string();
        let lookup_key = (provider.clone(), key.clone());
        
        // Add to primary index
        self.offerings.insert(lookup_key.clone(), offering);
        
        // Add to provider index
        self.by_provider
            .entry(provider)
            .or_default()
            .insert(key);
        
        // Add to text index
        self.index_offering_text(&lookup_key);
    }
    
    fn index_offering_text(&mut self, lookup_key: &(ProviderPubkey, OfferingKey)) {
        if let Some(offering) = self.offerings.get(lookup_key) {
            let keywords = self.extract_keywords(&offering.server_offering);
            for keyword in keywords {
                self.text_index
                    .entry(keyword)
                    .or_default()
                    .insert(lookup_key.clone());
            }
        }
    }
    
    fn extract_keywords(&self, offering: &ServerOffering) -> Vec<String> {
        let mut keywords = Vec::new();
        
        // Helper to add words from text
        let mut add_words = |text: &str| {
            for word in text.split_whitespace() {
                let word = word.to_lowercase();
                if word.len() > 2 && !self.is_stop_word(&word) {
                    keywords.push(word);
                }
            }
        };
        
        add_words(&offering.offer_name);
        add_words(&offering.description);
        add_words(&offering.datacenter_country);
        add_words(&offering.datacenter_city);
        
        if let Some(brand) = &offering.processor_brand {
            add_words(brand);
        }
        
        if let Some(gpu) = &offering.gpu_name {
            add_words(gpu);
        }
        
        for feature in &offering.features {
            add_words(feature);
        }
        
        for os in &offering.operating_systems {
            add_words(os);
        }
        
        keywords.dedup();
        keywords
    }
    
    fn is_stop_word(&self, word: &str) -> bool {
        matches!(
            word,
            "the" | "and" | "or" | "but" | "in" | "on" | "at" | "to" | "for" | "of" | "with" | "by"
        )
    }
    
    fn search_text_internal(&self, text: &str) -> HashSet<(ProviderPubkey, OfferingKey)> {
        let query_words: Vec<String> = text
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 2 && !self.is_stop_word(w))
            .collect();
        
        if query_words.is_empty() {
            return HashSet::new();
        }
        
        // Find offerings that match any query word
        let mut results = HashSet::new();
        for word in &query_words {
            if let Some(candidates) = self.text_index.get(word) {
                results.extend(candidates.iter().cloned());
            }
        }
        
        results
    }
    
    fn matches_filters(&self, offering: &Offering, filters: &[OfferingFilter]) -> bool {
        for filter in filters {
            if !self.matches_filter(offering, filter) {
                return false;
            }
        }
        true
    }
    
    fn matches_filter(&self, offering: &Offering, filter: &OfferingFilter) -> bool {
        let server = &offering.server_offering;
        match filter {
            OfferingFilter::PriceRange(min, max) => {
                server.monthly_price >= *min && server.monthly_price <= *max
            }
            OfferingFilter::ProductType(product_type) => {
                std::mem::discriminant(&server.product_type) == std::mem::discriminant(product_type)
            }
            OfferingFilter::Country(country) => {
                server.datacenter_country.to_lowercase() == country.to_lowercase()
            }
            OfferingFilter::City(city) => {
                server.datacenter_city.to_lowercase().contains(&city.to_lowercase())
            }
            OfferingFilter::HasGPU(has_gpu) => server.gpu_name.is_some() == *has_gpu,
            OfferingFilter::Currency(currency) => {
                std::mem::discriminant(&server.currency) == std::mem::discriminant(currency)
            }
            OfferingFilter::StockStatus(status) => {
                std::mem::discriminant(&server.stock) == std::mem::discriminant(status)
            }
            OfferingFilter::MinMemoryGB(min_gb) => {
                if let Some(memory_str) = &server.memory_amount {
                    if let Ok(memory_mb) = memory_str.replace(" MB", "").parse::<u32>() {
                        return memory_mb >= min_gb * 1024;
                    }
                    if let Ok(memory_gb) = memory_str.replace(" GB", "").parse::<u32>() {
                        return memory_gb >= *min_gb;
                    }
                }
                false
            }
            OfferingFilter::MinCores(min_cores) => {
                server.processor_cores.unwrap_or(0) >= *min_cores
            }
            OfferingFilter::VirtualizationType(virt_type) => {
                if let Some(server_virt) = &server.virtualization_type {
                    std::mem::discriminant(server_virt) == std::mem::discriminant(virt_type)
                } else {
                    false
                }
            }
        }
    }
    
    fn parse_csv_data(csv_data: &str) -> Result<Vec<ServerOffering>, OfferingError> {
        let cursor = std::io::Cursor::new(csv_data.as_bytes());
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(cursor);

        let mut server_offerings = Vec::new();

        for result in csv_reader.records() {
            match result {
                Ok(record) => match ProviderOfferings::parse_record(&record) {
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

        Ok(server_offerings)
    }
}