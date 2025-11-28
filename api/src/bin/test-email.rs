use anyhow::Result;
use clap::Parser;
use email_utils::{validate_email, EmailService};
use std::env;

#[derive(Parser)]
#[command(name = "test-email")]
#[command(about = "Test email configuration by sending a test email")]
struct Cli {
    /// Recipient email address
    #[arg(short, long)]
    to: String,
    /// Test DKIM signing (default: false)
    #[arg(long)]
    with_dkim: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load .env file if it exists
    dotenv::dotenv().ok();

    println!("\n========================================");
    println!("  Email Configuration Test");
    println!("========================================\n");

    // Validate email address
    if let Err(e) = validate_email(&cli.to) {
        eprintln!("❌ Invalid email address: {}", e);
        anyhow::bail!("Invalid email: {}", e);
    }

    // Check for MailChannels API key
    let api_key = match env::var("MAILCHANNELS_API_KEY") {
        Ok(key) if !key.is_empty() => {
            println!("✓ MailChannels API key found");
            key
        }
        _ => {
            eprintln!("❌ MAILCHANNELS_API_KEY environment variable not set or empty");
            eprintln!("\nPlease set MAILCHANNELS_API_KEY in your .env file or environment.");
            eprintln!("Get your API key from: https://app.mailchannels.com/");
            anyhow::bail!("Missing MAILCHANNELS_API_KEY");
        }
    };

    // Check DKIM configuration if requested
    let (dkim_domain, dkim_selector, dkim_private_key) = if cli.with_dkim {
        let domain = env::var("DKIM_DOMAIN").ok();
        let selector = env::var("DKIM_SELECTOR").ok();
        let private_key = env::var("DKIM_PRIVATE_KEY").ok();

        match (&domain, &selector, &private_key) {
            (Some(d), Some(s), Some(k)) if !d.is_empty() && !s.is_empty() && !k.is_empty() => {
                println!("✓ DKIM configuration found:");
                println!("  - Domain: {}", d);
                println!("  - Selector: {}", s);
                println!(
                    "  - Private key: {}...{} ({} bytes)",
                    &k.chars().take(10).collect::<String>(),
                    &k.chars().rev().take(10).collect::<String>(),
                    k.len()
                );
                (domain, selector, private_key)
            }
            _ => {
                eprintln!("\n⚠️  DKIM requested but configuration incomplete:");
                eprintln!("  DKIM_DOMAIN: {}", domain.as_deref().unwrap_or("not set"));
                eprintln!(
                    "  DKIM_SELECTOR: {}",
                    selector.as_deref().unwrap_or("not set")
                );
                eprintln!(
                    "  DKIM_PRIVATE_KEY: {}",
                    if private_key.is_some() {
                        "set"
                    } else {
                        "not set"
                    }
                );
                eprintln!("\nProceeding without DKIM signing...\n");
                (None, None, None)
            }
        }
    } else {
        println!("✓ DKIM signing: disabled (use --with-dkim to enable)");
        (None, None, None)
    };

    // Create email service
    let email_service = EmailService::new(api_key, dkim_domain, dkim_selector, dkim_private_key);

    // Create test email
    let from_addr = "noreply@decent-cloud.org";
    let subject = "Decent Cloud Email Test";
    let body = format!(
        "This is a test email from the Decent Cloud API server.\n\n\
        Test details:\n\
        - Recipient: {}\n\
        - DKIM signing: {}\n\
        - Timestamp: {}\n\n\
        If you received this email, your email configuration is working correctly!\n\n\
        Best regards,\n\
        The Decent Cloud Team",
        cli.to,
        if cli.with_dkim { "enabled" } else { "disabled" },
        chrono::Utc::now().to_rfc3339()
    );

    println!("\nSending test email...");
    println!("  From: {}", from_addr);
    println!("  To: {}", cli.to);
    println!("  Subject: {}", subject);

    // Send directly (not using queue since we don't have DB access)
    match email_service
        .send_email(from_addr, &cli.to, subject, &body, false)
        .await
    {
        Ok(()) => {
            println!("\n✅ SUCCESS! Test email sent successfully.");
            println!("\nPlease check your inbox at: {}", cli.to);
            println!(
                "\nNote: Email may take a few minutes to arrive and may be in your spam folder."
            );

            if cli.with_dkim {
                println!("\n🔒 DKIM Configuration Test:");
                println!("  - Check email headers for 'DKIM-Signature' field");
                println!("  - Verify signature shows as valid in your email client");
                println!("  - Run online DKIM checker tools to validate signature");
            }

            Ok(())
        }
        Err(e) => {
            eprintln!("\n❌ FAILED to send test email:");
            eprintln!("\n{:#}", e);

            eprintln!("\nTroubleshooting:");
            eprintln!("  1. Verify your MAILCHANNELS_API_KEY is correct");
            eprintln!("  2. Check that your MailChannels account is active");
            eprintln!("  3. Verify sender domain is authorized in MailChannels");

            if cli.with_dkim {
                eprintln!("  4. Verify DKIM private key is correctly base64 encoded");
                eprintln!("  5. Check DKIM DNS records are published for your domain");
            }

            Err(e)
        }
    }
}
