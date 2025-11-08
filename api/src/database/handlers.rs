use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use std::collections::HashMap;

impl Database {
    pub async fn insert_entries(&self, entries: Vec<LedgerEntryData>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Group entries by label for batch processing
        let mut grouped_entries: HashMap<String, Vec<LedgerEntryData>> = HashMap::new();
        for entry in entries {
            grouped_entries
                .entry(entry.label.clone())
                .or_default()
                .push(entry);
        }

        // Process each group efficiently
        for (label, entries) in grouped_entries {
            match label.as_str() {
                "ProvRegister" => {
                    self.insert_provider_registrations(&mut tx, &entries)
                        .await?
                }
                "ProvCheckIn" => self.insert_provider_check_ins(&mut tx, &entries).await?,
                "ProvProfile" => self.insert_provider_profiles(&mut tx, &entries).await?,
                "DCTokenTransfer" => self.insert_token_transfers(&mut tx, &entries).await?,
                "DCTokenApproval" => self.insert_token_approvals(&mut tx, &entries).await?,
                "UserRegister" => self.insert_user_registrations(&mut tx, &entries).await?,
                "ContractSignReq" => {
                    self.insert_contract_sign_requests(&mut tx, &entries)
                        .await?
                }
                "ContractSignReply" => self.insert_contract_sign_replies(&mut tx, &entries).await?,
                "RepChange" => self.insert_reputation_changes(&mut tx, &entries).await?,
                "RepAge" => self.insert_reputation_aging(&mut tx, &entries).await?,
                "RewardDistr" => self.insert_reward_distributions(&mut tx, &entries).await?,
                "LinkedIcIds" => self.insert_linked_ic_ids(&mut tx, &entries).await?,
                // Handle NP-prefixed labels (namespace providers)
                "NPRegister" => {
                    self.insert_provider_registrations(&mut tx, &entries)
                        .await?
                }
                "NPCheckIn" => self.insert_provider_check_ins(&mut tx, &entries).await?,
                // Skip all offering entries (ProvOffering, NPOffering) - will be handled in DB
                "ProvOffering" | "NPOffering" => {
                    tracing::debug!("Skipping offering entry: {} - offerings will be handled directly in DB", entries.first().map(|e| e.label.as_str()).unwrap_or("unknown"));
                },
                _ => tracing::warn!("Unknown ledger entry label: {}", label),
            }
        }

        tx.commit().await?;
        Ok(())
    }
}
