use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use dcc_common::{
    LABEL_CONTRACT_SIGN_REPLY, LABEL_CONTRACT_SIGN_REQUEST, LABEL_DC_TOKEN_APPROVAL,
    LABEL_DC_TOKEN_TRANSFER, LABEL_LINKED_IC_IDS, LABEL_NP_CHECK_IN, LABEL_NP_OFFERING,
    LABEL_NP_PROFILE, LABEL_NP_REGISTER, LABEL_PROV_CHECK_IN, LABEL_PROV_OFFERING,
    LABEL_PROV_PROFILE, LABEL_PROV_REGISTER, LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE,
    LABEL_REWARD_DISTRIBUTION, LABEL_USER_REGISTER,
};
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

        // Process known labels in the defined order, to ensure foreign key constraints are met
        let known_labels = [
            LABEL_REPUTATION_CHANGE,
            LABEL_REPUTATION_AGE,
            LABEL_DC_TOKEN_TRANSFER,
            LABEL_DC_TOKEN_APPROVAL,
            LABEL_PROV_REGISTER,
            LABEL_NP_REGISTER,
            LABEL_PROV_CHECK_IN,
            LABEL_NP_CHECK_IN,
            LABEL_PROV_PROFILE,
            LABEL_NP_PROFILE,
            LABEL_USER_REGISTER,
            LABEL_PROV_OFFERING,
            LABEL_NP_OFFERING,
            LABEL_REWARD_DISTRIBUTION,
            LABEL_CONTRACT_SIGN_REQUEST,
            LABEL_CONTRACT_SIGN_REPLY,
            LABEL_LINKED_IC_IDS,
        ];

        for label in known_labels {
            if let Some(entries) = grouped_entries.remove(label) {
                match label {
                    LABEL_REPUTATION_CHANGE => {
                        self.insert_reputation_changes(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert reputation changes: {}", e)
                            })?;
                    }
                    LABEL_REPUTATION_AGE => {
                        self.insert_reputation_aging(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert reputation aging: {}", e)
                            })?;
                    }
                    LABEL_DC_TOKEN_TRANSFER => {
                        self.insert_token_transfers(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert token transfers: {}", e)
                            })?;
                    }
                    LABEL_DC_TOKEN_APPROVAL => {
                        self.insert_token_approvals(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert token approvals: {}", e)
                            })?;
                    }
                    LABEL_PROV_REGISTER | LABEL_NP_REGISTER => {
                        self.insert_provider_registrations(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert provider registrations: {}", e)
                            })?;
                    }
                    LABEL_PROV_CHECK_IN | LABEL_NP_CHECK_IN => {
                        self.insert_provider_check_ins(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert provider check-ins: {}", e)
                            })?;
                    }
                    LABEL_PROV_PROFILE | LABEL_NP_PROFILE => {
                        self.insert_provider_profiles(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert provider profiles: {}", e)
                            })?;
                    }
                    LABEL_USER_REGISTER => {
                        self.insert_user_registrations(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert user registrations: {}", e)
                            })?;
                    }
                    LABEL_PROV_OFFERING | LABEL_NP_OFFERING => {
                        // Skip offering entries - will be handled directly in DB
                        tracing::debug!(
                            "Skipping ProvOffering entries - will be handled directly in DB"
                        );
                    }
                    LABEL_REWARD_DISTRIBUTION => {
                        self.insert_reward_distributions(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert reward distributions: {}", e)
                            })?;
                    }
                    LABEL_CONTRACT_SIGN_REQUEST => {
                        self.insert_contract_sign_requests(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert contract sign requests: {}", e)
                            })?;
                    }
                    LABEL_CONTRACT_SIGN_REPLY => {
                        self.insert_contract_sign_replies(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert contract sign replies: {}", e)
                            })?;
                    }
                    LABEL_LINKED_IC_IDS => {
                        self.insert_linked_ic_ids(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert linked IC IDs: {}", e)
                            })?;
                    }
                    _ => unreachable!(), // All labels in known_labels are handled above
                }
            }
        }

        for (label, entries) in grouped_entries {
            tracing::error!(
                "Unknown and unhandled ledger entry label: {} with {} entries",
                label,
                entries.len()
            );
        }

        tx.commit().await?;
        Ok(())
    }
}
