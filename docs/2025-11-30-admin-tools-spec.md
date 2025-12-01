# Admin Tools
**Status:** Planning

## Requirements

### Must-have
- [ ] API CLI with `--env dev|prod` flag (absorbs `test-email` binary)
- [ ] CLI: `api-cli admin grant <username>` - Grant admin access
- [ ] CLI: `api-cli admin revoke <username>` - Revoke admin access
- [ ] CLI: `api-cli admin list` - List all admin accounts
- [ ] CLI: `api-cli test-email --to <email>` - Send test email (replaces test-email binary)
- [ ] Admin Dashboard: View failed emails queue with retry action
- [ ] Admin Dashboard: Email queue stats (pending/sent/failed counts)
- [ ] Admin API: Reset email retry counter for specific email
- [ ] Admin API: Bulk retry all failed emails

### Nice-to-have
- [ ] Admin Dashboard: Account lookup (view keys, disable keys, add recovery keys)
- [ ] Admin Dashboard: View/edit account email verification status
- [ ] Admin Dashboard: Send test email from UI

## Architecture

### CLI Structure (`api/src/bin/api-cli.rs`)
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "api-cli")]
struct Cli {
    #[arg(long, default_value = "dev")]
    env: Environment,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum Environment { Dev, Prod }

#[derive(Subcommand)]
enum Commands {
    Admin {
        #[command(subcommand)]
        action: AdminAction,
    },
    TestEmail {
        #[arg(long)]
        to: String,
        #[arg(long)]
        with_dkim: bool,
    },
}

#[derive(Subcommand)]
enum AdminAction {
    Grant { username: String },
    Revoke { username: String },
    List,
}
```

### Database Schema Change
Add `is_admin` column to `accounts` table:
```sql
ALTER TABLE accounts ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;
```

### Admin Auth Change
Current: `ADMIN_PUBLIC_KEYS` env var with comma-separated keys
New: Check `accounts.is_admin = 1` for the authenticated user's account

### Admin Dashboard Route
- Path: `/dashboard/admin`
- Auth: Check `account.is_admin` flag
- Components: EmailQueuePanel, AccountManagementPanel

### New Admin API Endpoints
```
POST /admin/emails/reset/:email_id     - Reset retry counter for single email
POST /admin/emails/retry-all-failed    - Bulk retry all failed emails
GET  /admin/emails/stats               - Get email queue statistics
```

## Steps

### Step 1: Database Migration - Add is_admin column
**Success:** Migration runs, accounts table has is_admin column, existing accounts have is_admin=0
**Status:** Pending

### Step 2: Update AdminAuthenticatedUser to use is_admin flag
**Success:** Admin auth checks account.is_admin instead of ADMIN_PUBLIC_KEYS env var
**Status:** Complete

### Step 3: Create api-cli binary with admin commands
**Success:** `api-cli admin grant/revoke/list` commands work, `test-email` binary functionality absorbed
**Status:** Pending

### Step 4: Add new admin API endpoints
**Success:** Reset email, bulk retry, and stats endpoints work with tests
**Status:** Pending

### Step 5: Create admin dashboard frontend
**Success:** Admin-only route with email queue management UI
**Status:** Pending

## Execution Log
### Step 1
- **Implementation:** Complete
  - Created migration file `/code/api/migrations/021_admin_accounts.sql`
  - Added `is_admin INTEGER NOT NULL DEFAULT 0` column to accounts table
  - Added index `idx_accounts_is_admin` for efficient admin queries
  - Updated `Account` struct in `/code/api/src/database/accounts.rs` to include `is_admin: i64` field
  - Updated all SELECT queries to include `is_admin` column:
    - `get_account()`
    - `get_account_by_username()`
    - `get_account_by_email()`
  - Updated test helpers to include migration 020 and 021
  - Added test `test_is_admin_migration` to verify migration works
  - Regenerated sqlx offline data (`.sqlx/`)
- **Review:** All tests pass (`cargo make` - 490 tests passed)
- **Outcome:** Success - Database migration complete, all accounts have `is_admin=0` by default

### Step 2
- **Implementation:** Complete
  - Modified `AdminAuthenticatedUser::from_request` in `/code/api/src/auth.rs` to:
    - Get database from request context using `request.data::<Arc<Database>>()`
    - Look up account by public key using `get_account_id_by_public_key()`
    - Fetch account using `get_account()`
    - Check `account.is_admin == 1` and return 403 Forbidden if not admin
  - Deprecated `get_admin_pubkeys()` function with `#[deprecated]` attribute
  - Marked all legacy `test_get_admin_pubkeys_*` tests with `#[allow(deprecated)]`
  - Updated documentation comments to reflect database-based authentication
- **Review:** All tests pass (`SQLX_OFFLINE=true cargo make` - 490 tests passed)
- **Outcome:** Success - Admin authentication now uses `is_admin` database flag instead of `ADMIN_PUBLIC_KEYS` environment variable

## Completion Summary
Steps 1-2 complete. Admin authentication migrated to database-based approach, all tests passing.
