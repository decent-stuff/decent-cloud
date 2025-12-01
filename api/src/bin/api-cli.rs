use anyhow::Result;
use api::database::Database;
use clap::{Parser, Subcommand, ValueEnum};
use email_utils::{validate_email, EmailService};
use std::env;

#[derive(Parser)]
#[command(name = "api-cli")]
#[command(about = "Decent Cloud API CLI for admin and testing tasks")]
struct Cli {
    /// Environment (dev or prod)
    #[arg(long, default_value = "dev")]
    env: Environment,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum Environment {
    Dev,
    Prod,
}

#[derive(Subcommand)]
enum Commands {
    /// Admin account management
    Admin {
        #[command(subcommand)]
        action: AdminAction,
    },
    /// Send test email (for testing email configuration)
    TestEmail {
        /// Recipient email address
        #[arg(long)]
        to: String,
        /// Test DKIM signing (default: false)
        #[arg(long)]
        with_dkim: bool,
    },
}

#[derive(Subcommand)]
enum AdminAction {
    /// Grant admin access to a user
    Grant { username: String },
    /// Revoke admin access from a user
    Revoke { username: String },
    /// List all admin accounts
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load environment-specific .env file
    match cli.env {
        Environment::Dev => {
            dotenv::from_filename("/code/api/.env").ok();
        }
        Environment::Prod => {
            dotenv::from_filename("/code/cf/.env.prod").ok();
        }
    }

    match cli.command {
        Commands::Admin { action } => handle_admin_action(action).await,
        Commands::TestEmail { to, with_dkim } => handle_test_email(&to, with_dkim).await,
    }
}

async fn handle_admin_action(action: AdminAction) -> Result<()> {
    // Get database URL from environment or use default
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/ledger.db?mode=rwc".to_string());

    // Connect to database
    let db = Database::new(&database_url).await?;

    match action {
        AdminAction::Grant { username } => {
            db.set_admin_status(&username, true).await?;
            println!("✓ Admin access granted to: {}", username);
        }
        AdminAction::Revoke { username } => {
            db.set_admin_status(&username, false).await?;
            println!("✓ Admin access revoked from: {}", username);
        }
        AdminAction::List => {
            let admins = db.list_admins().await?;
            if admins.is_empty() {
                println!("No admin accounts found.");
            } else {
                println!("\nAdmin Accounts:");
                println!("{}", "=".repeat(80));
                println!("{:<20} {:<40} {:<20}", "Username", "Email", "Created At");
                println!("{}", "-".repeat(80));
                for admin in &admins {
                    let email = admin.email.as_deref().unwrap_or("N/A");
                    let created = chrono::DateTime::from_timestamp(admin.created_at, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Invalid".to_string());
                    println!("{:<20} {:<40} {:<20}", admin.username, email, created);
                }
                println!("{}", "=".repeat(80));
                println!("Total: {} admin account(s)", admins.len());
            }
        }
    }

    Ok(())
}

async fn handle_test_email(to: &str, with_dkim: bool) -> Result<()> {
    println!("\n========================================");
    println!("  Email Configuration Test");
    println!("========================================\n");

    // Validate email address
    if let Err(e) = validate_email(to) {
        eprintln!("Invalid email address: {}", e);
        anyhow::bail!("Invalid email: {}", e);
    }

    // Check for MailChannels API key
    let api_key = match env::var("MAILCHANNELS_API_KEY") {
        Ok(key) if !key.is_empty() => {
            println!("✓ MailChannels API key found");
            key
        }
        _ => {
            eprintln!("MAILCHANNELS_API_KEY environment variable not set or empty");
            eprintln!("\nPlease set MAILCHANNELS_API_KEY in your .env file or environment.");
            eprintln!("Get your API key from: https://app.mailchannels.com/");
            anyhow::bail!("Missing MAILCHANNELS_API_KEY");
        }
    };

    // Check DKIM configuration if requested
    let (dkim_domain, dkim_selector, dkim_private_key) = if with_dkim {
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
                eprintln!("\nDKIM requested but configuration incomplete:");
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
        to,
        if with_dkim { "enabled" } else { "disabled" },
        chrono::Utc::now().to_rfc3339()
    );

    println!("\nSending test email...");
    println!("  From: {}", from_addr);
    println!("  To: {}", to);
    println!("  Subject: {}", subject);

    // Send directly (not using queue since we don't have DB access)
    match email_service
        .send_email(from_addr, to, subject, &body, false)
        .await
    {
        Ok(()) => {
            println!("\n✓ SUCCESS! Test email sent successfully.");
            println!("\nPlease check your inbox at: {}", to);
            println!(
                "\nNote: Email may take a few minutes to arrive and may be in your spam folder."
            );

            if with_dkim {
                println!("\nDKIM Configuration Test:");
                println!("  - Check email headers for 'DKIM-Signature' field");
                println!("  - Verify signature shows as valid in your email client");
                println!("  - Run online DKIM checker tools to validate signature");
            }

            Ok(())
        }
        Err(e) => {
            eprintln!("\nFAILED to send test email:");
            eprintln!("\n{:#}", e);

            eprintln!("\nTroubleshooting:");
            eprintln!("  1. Verify your MAILCHANNELS_API_KEY is correct");
            eprintln!("  2. Check that your MailChannels account is active");
            eprintln!("  3. Verify sender domain is authorized in MailChannels");

            if with_dkim {
                eprintln!("  4. Verify DKIM private key is correctly base64 encoded");
                eprintln!("  5. Check DKIM DNS records are published for your domain");
            }

            Err(e)
        }
    }
}
