use crate::database::providers::ProviderProfile;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct CommonIssue {
    question: String,
    answer: String,
}

/// Parse JSON array from TEXT field, returning empty vec on error
fn parse_json_array<T: for<'de> Deserialize<'de>>(json_str: &Option<String>) -> Vec<T> {
    json_str
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

/// Convert payment method code to human-readable label
fn payment_method_label(method: &str) -> &str {
    match method {
        "crypto" => "Cryptocurrency (BTC, ETH, etc.)",
        "stripe" => "Credit Card (Stripe)",
        "paypal" => "PayPal",
        "bank_transfer" => "Bank Transfer",
        "icp" => "ICP (Internet Computer)",
        _ => method,
    }
}

/// Generate formatted timestamp for article footer
fn format_timestamp(timestamp_ns: i64) -> String {
    let dt =
        DateTime::from_timestamp(timestamp_ns / 1_000_000_000, 0).unwrap_or_else(|| Utc::now());
    dt.format("%Y-%m-%d").to_string()
}

/// Generate a help center article from a ProviderProfile
pub fn generate_provider_article(profile: &ProviderProfile) -> Result<String> {
    let provider_name = &profile.name;
    let mut article = String::new();

    // Title
    article.push_str(&format!("# {} on Decent Cloud\n\n", provider_name));

    // Overview section
    article.push_str("## Overview\n\n");

    // Parse regions
    let regions: Vec<String> = parse_json_array(&profile.regions);
    let regions_text = if regions.is_empty() {
        "multiple regions".to_string()
    } else {
        regions.join(", ")
    };

    article.push_str(&format!(
        "{} is a cloud provider on the Decent Cloud marketplace offering services in {}.\n\n",
        provider_name, regions_text
    ));

    // Description
    if let Some(desc) = &profile.description {
        article.push_str(desc);
        article.push_str("\n\n");
    }

    // Why Choose Us
    if let Some(why_choose) = &profile.why_choose_us {
        article.push_str(&format!("### Why Choose {}?\n\n", provider_name));
        article.push_str(why_choose);
        article.push_str("\n\n");
    }

    // Key Differentiators
    let usps: Vec<String> = parse_json_array(&profile.unique_selling_points);
    if !usps.is_empty() {
        article.push_str("**Key Differentiators:**\n");
        for point in usps {
            article.push_str(&format!("- {}\n", point));
        }
        article.push_str("\n");
    }

    // Getting Started (static)
    article.push_str("## Getting Started\n\n");
    article.push_str("1. Browse the [Decent Cloud Marketplace](https://app.decent-cloud.org/dashboard/marketplace)\n");
    article.push_str(&format!("2. Filter by provider: **{}**\n", provider_name));
    article.push_str("3. Select an offering that meets your needs\n");
    article.push_str("4. Complete rental through the platform\n\n");

    // Pricing & Payment
    let payment_methods: Vec<String> = parse_json_array(&profile.payment_methods);
    let has_payment_or_refund = !payment_methods.is_empty() || profile.refund_policy.is_some();

    if has_payment_or_refund {
        article.push_str("## Pricing & Payment\n\n");

        if !payment_methods.is_empty() {
            article.push_str("**Accepted Payment Methods:**\n");
            for method in payment_methods {
                article.push_str(&format!("- {}\n", payment_method_label(&method)));
            }
            article.push_str("\n");
        }

        if let Some(refund) = &profile.refund_policy {
            article.push_str(&format!("**Refund Policy:** {}\n\n", refund));
        }
    }

    // Support section
    let has_support_info = profile.support_email.is_some()
        || profile.support_hours.is_some()
        || profile.support_channels.is_some()
        || (profile.sla_guarantee.is_some() && profile.sla_guarantee.as_deref() != Some("none"));

    if has_support_info {
        article.push_str("## Support\n\n");

        if let Some(email) = &profile.support_email {
            article.push_str(&format!("**Email:** {}\n\n", email));
        }

        if let Some(hours) = &profile.support_hours {
            article.push_str(&format!("**Hours:** {}\n\n", hours));
        }

        if let Some(channels_json) = &profile.support_channels {
            let channels: Vec<String> = parse_json_array(&Some(channels_json.clone()));
            if !channels.is_empty() {
                article.push_str(&format!(
                    "**Available Channels:** {}\n\n",
                    channels.join(", ")
                ));
            }
        }

        if let Some(sla) = &profile.sla_guarantee {
            if sla != "none" {
                article.push_str(&format!("**SLA Guarantee:** {} uptime\n\n", sla));
            }
        }
    }

    // FAQ section
    let common_issues: Vec<CommonIssue> = parse_json_array(&profile.common_issues);
    if !common_issues.is_empty() {
        article.push_str("## FAQ\n\n");
        for issue in common_issues {
            article.push_str(&format!("### {}\n\n", issue.question));
            article.push_str(&format!("{}\n\n", issue.answer));
        }
    }

    // Need Help footer
    article.push_str("## Need Help?\n\n");
    article.push_str(&format!(
        "If you have questions about {}'s services, you can:\n",
        provider_name
    ));
    article.push_str(&format!(
        "1. Contact {} directly via the channels above\n",
        provider_name
    ));
    article.push_str("2. Use the Decent Cloud support chat for platform-related questions\n\n");

    // Footer
    article.push_str("---\n");
    let updated_date = format_timestamp(profile.updated_at_ns);
    article.push_str(&format!(
        "*This article is maintained by {}. Last updated: {}*\n",
        provider_name, updated_date
    ));

    Ok(article)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_profile() -> ProviderProfile {
        ProviderProfile {
            pubkey: vec![1, 2, 3],
            name: "TestProvider".to_string(),
            description: None,
            website_url: None,
            logo_url: None,
            why_choose_us: None,
            api_version: "1.0".to_string(),
            profile_version: "1.0".to_string(),
            updated_at_ns: 1733500000000000000,
            support_email: None,
            support_hours: None,
            support_channels: None,
            regions: None,
            payment_methods: None,
            refund_policy: None,
            sla_guarantee: None,
            unique_selling_points: None,
            common_issues: None,
            onboarding_completed_at: None,
        }
    }

    #[test]
    fn test_minimal_article_generation() {
        let profile = create_minimal_profile();
        let article = generate_provider_article(&profile).unwrap();

        assert!(article.contains("# TestProvider on Decent Cloud"));
        assert!(article.contains("## Overview"));
        assert!(article.contains("is a cloud provider on the Decent Cloud marketplace"));
        assert!(article.contains("## Getting Started"));
        assert!(article.contains("## Need Help?"));
        assert!(article.contains("*This article is maintained by TestProvider"));
    }

    #[test]
    fn test_full_article_generation() {
        let mut profile = create_minimal_profile();
        profile.description = Some("We provide high-performance cloud services.".to_string());
        profile.why_choose_us = Some("Industry-leading uptime and support.".to_string());
        profile.regions = Some(r#"["US", "EU", "APAC"]"#.to_string());
        profile.unique_selling_points = Some(
            r#"["Low latency global network", "Instant provisioning", "24/7 human support"]"#
                .to_string(),
        );
        profile.payment_methods = Some(r#"["crypto", "stripe", "paypal"]"#.to_string());
        profile.refund_policy = Some("30-day money-back guarantee".to_string());
        profile.support_email = Some("support@testprovider.com".to_string());
        profile.support_hours = Some("24/7".to_string());
        profile.support_channels = Some(r#"["email", "chat", "phone"]"#.to_string());
        profile.sla_guarantee = Some("99.9%".to_string());
        profile.common_issues = Some(
            r#"[{"question": "How do I access my server?", "answer": "SSH credentials are sent to your email within 5 minutes."}]"#
                .to_string(),
        );

        let article = generate_provider_article(&profile).unwrap();

        // Check all sections are present
        assert!(article.contains("# TestProvider on Decent Cloud"));
        assert!(article.contains("offering services in US, EU, APAC"));
        assert!(article.contains("We provide high-performance cloud services"));
        assert!(article.contains("### Why Choose TestProvider?"));
        assert!(article.contains("Industry-leading uptime and support"));
        assert!(article.contains("**Key Differentiators:**"));
        assert!(article.contains("- Low latency global network"));
        assert!(article.contains("- Instant provisioning"));
        assert!(article.contains("- 24/7 human support"));
        assert!(article.contains("## Pricing & Payment"));
        assert!(article.contains("Cryptocurrency (BTC, ETH, etc.)"));
        assert!(article.contains("Credit Card (Stripe)"));
        assert!(article.contains("PayPal"));
        assert!(article.contains("**Refund Policy:** 30-day money-back guarantee"));
        assert!(article.contains("## Support"));
        assert!(article.contains("**Email:** support@testprovider.com"));
        assert!(article.contains("**Hours:** 24/7"));
        assert!(article.contains("**Available Channels:** email, chat, phone"));
        assert!(article.contains("**SLA Guarantee:** 99.9% uptime"));
        assert!(article.contains("## FAQ"));
        assert!(article.contains("### How do I access my server?"));
        assert!(article.contains("SSH credentials are sent to your email within 5 minutes"));
    }

    #[test]
    fn test_payment_method_labels() {
        assert_eq!(
            payment_method_label("crypto"),
            "Cryptocurrency (BTC, ETH, etc.)"
        );
        assert_eq!(payment_method_label("stripe"), "Credit Card (Stripe)");
        assert_eq!(payment_method_label("paypal"), "PayPal");
        assert_eq!(payment_method_label("bank_transfer"), "Bank Transfer");
        assert_eq!(payment_method_label("icp"), "ICP (Internet Computer)");
        assert_eq!(payment_method_label("unknown"), "unknown");
    }

    #[test]
    fn test_parse_json_array_valid() {
        let json = Some(r#"["item1", "item2", "item3"]"#.to_string());
        let result: Vec<String> = parse_json_array(&json);
        assert_eq!(result, vec!["item1", "item2", "item3"]);
    }

    #[test]
    fn test_parse_json_array_invalid() {
        let json = Some("not valid json".to_string());
        let result: Vec<String> = parse_json_array(&json);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_json_array_none() {
        let json = None;
        let result: Vec<String> = parse_json_array(&json);
        assert!(result.is_empty());
    }

    #[test]
    fn test_sla_none_excluded() {
        let mut profile = create_minimal_profile();
        profile.sla_guarantee = Some("none".to_string());
        profile.support_email = Some("support@test.com".to_string());

        let article = generate_provider_article(&profile).unwrap();

        // Support section should exist (has email)
        assert!(article.contains("## Support"));
        // But SLA should not appear
        assert!(!article.contains("SLA Guarantee"));
    }

    #[test]
    fn test_no_payment_section_when_empty() {
        let profile = create_minimal_profile();
        let article = generate_provider_article(&profile).unwrap();

        assert!(!article.contains("## Pricing & Payment"));
    }

    #[test]
    fn test_no_support_section_when_empty() {
        let mut profile = create_minimal_profile();
        profile.sla_guarantee = Some("none".to_string()); // Should not trigger support section

        let article = generate_provider_article(&profile).unwrap();

        assert!(!article.contains("## Support"));
    }

    #[test]
    fn test_common_issues_structure() {
        let mut profile = create_minimal_profile();
        profile.common_issues = Some(
            r#"[
                {"question": "Q1?", "answer": "A1"},
                {"question": "Q2?", "answer": "A2"}
            ]"#
            .to_string(),
        );

        let article = generate_provider_article(&profile).unwrap();

        assert!(article.contains("## FAQ"));
        assert!(article.contains("### Q1?"));
        assert!(article.contains("A1"));
        assert!(article.contains("### Q2?"));
        assert!(article.contains("A2"));
    }
}
