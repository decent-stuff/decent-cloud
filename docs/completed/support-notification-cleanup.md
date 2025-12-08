# Spec: Clean Up Support Notification Architecture

## Problem

The current support bot notification system is overly complex and tied to `contract_id`:
- Requires `contract_id` in conversation custom_attributes
- Looks up contract → provider_pubkey → notification config
- Has multiple code paths (handler.rs, webhooks.rs)
- Falls back to `DEFAULT_ESCALATION_USER` for general inquiries

This is wrong because:
1. Support inquiries shouldn't require a contract
2. Chatwoot already has a native assignment model (assignee, teams)
3. We already link Chatwoot users to our users via `chatwoot_user_id`

## Solution

Use Chatwoot's native assignment model. Notify based on **who is assigned** to the conversation, not which contract it relates to.

### Data Model

```
Chatwoot User (agent) ←──chatwoot_user_id──→ Our Account
                                                  │
                                                  ↓
                                       user_notification_config
                                       (telegram, email, sms)
```

### Notification Flow

```
Conversation escalates (bot sets status = "open")
                    │
                    ↓
         Webhook: conversation_status_changed
                    │
                    ↓
         Extract from payload:
         - meta.assignee.id (chatwoot_user_id)
         - meta.team.id (optional)
                    │
                    ↓
         ┌─────────┴─────────┐
         │                   │
    Has assignee?       Has team?
         │                   │
         ↓                   ↓
    Lookup user by      Lookup all team
    chatwoot_user_id    members' user_ids
         │                   │
         └────────┬──────────┘
                  │
                  ↓
         Send notification to each user
         (based on their notification_config)
                  │
                  ↓
         No assignee/team?
         → Notify DEFAULT_ESCALATION_USER
```

## Implementation Tasks

### 1. Database: Add lookup by chatwoot_user_id

File: `api/src/database/accounts.rs`

```rust
/// Get account by Chatwoot user ID
pub async fn get_account_by_chatwoot_user_id(&self, chatwoot_user_id: i64) -> Result<Option<Account>>
```

### 2. Database: Add team member lookup (optional, for team notifications)

If we want to notify all team members, we need to either:
- Call Chatwoot API to get team members
- Or just notify the assignee (simpler)

**Recommendation**: Start simple - only notify assignee. Add team support later if needed.

### 3. Remove contract_id from SupportNotification

File: `api/src/support_bot/notifications.rs`

Before:
```rust
pub struct SupportNotification {
    pub provider_pubkey: Vec<u8>,
    pub conversation_id: i64,
    pub contract_id: String,  // REMOVE
    pub summary: String,
    pub chatwoot_link: String,
}
```

After:
```rust
pub struct SupportNotification {
    pub user_pubkey: Vec<u8>,  // renamed for clarity
    pub conversation_id: i64,
    pub summary: String,
    pub chatwoot_link: String,
}
```

Update all notification message templates to remove contract_id references.

### 4. Remove contract_id from handler.rs

File: `api/src/support_bot/handler.rs`

Remove:
- `get_contract_info()` function
- `ContractInfo` struct
- `contract_id` parameter from `handle_customer_message()`
- All contract lookup logic

The handler should ONLY:
1. Process the message with AI
2. Send response or escalate
3. On escalation, just set status to "open" - notifications handled by webhook

### 5. Update webhook handler for notifications

File: `api/src/openapi/webhooks.rs`

On `conversation_status_changed` where status == "open":

```rust
// Extract assignee from webhook payload
if let Some(meta) = &conv.meta {
    if let Some(assignee) = &meta.assignee {
        let chatwoot_user_id = assignee.id;

        // Look up our user
        if let Some(account) = db.get_account_by_chatwoot_user_id(chatwoot_user_id).await? {
            // Get their pubkey and send notification
            if let Some(pubkey) = db.get_pubkey_by_account_id(&account.id).await? {
                let notification = SupportNotification::new(
                    pubkey,
                    conv.id,
                    format!("New support conversation requires attention"),
                    &chatwoot_url,
                );
                dispatch_notification(&db, email_service, &notification).await?;
            }
        }
    }
}

// Fallback to DEFAULT_ESCALATION_USER if no assignee
```

### 6. Update webhook payload parsing

File: `api/src/openapi/webhooks.rs`

Add structs for meta.assignee and meta.team:

```rust
#[derive(Debug, Deserialize)]
struct ConversationMeta {
    assignee: Option<AssigneeInfo>,
    team: Option<TeamInfo>,
}

#[derive(Debug, Deserialize)]
struct AssigneeInfo {
    id: i64,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TeamInfo {
    id: i64,
    name: Option<String>,
}
```

### 7. Remove contract_id from webhooks.rs call site

File: `api/src/openapi/webhooks.rs`

In the message_created handler, remove contract_id extraction and passing:

```rust
// Before
let contract_id = conv.custom_attributes...
handle_customer_message(..., contract_id, ...).await

// After
handle_customer_message(...).await  // no contract_id
```

### 8. Clean up AGENTS.md documentation

File: `api/src/support_bot/AGENTS.md`

Update the architecture documentation to reflect the new flow.

### 9. Update tests

Update all tests that reference contract_id in the notification path.

## Files to Modify

1. `api/src/database/accounts.rs` - Add `get_account_by_chatwoot_user_id()`
2. `api/src/support_bot/notifications.rs` - Remove contract_id from struct
3. `api/src/support_bot/handler.rs` - Remove contract logic, simplify signature
4. `api/src/openapi/webhooks.rs` - Handle notifications via assignee lookup
5. `api/src/support_bot/AGENTS.md` - Update documentation

## Testing

1. Create a Chatwoot agent linked to our user (chatwoot_user_id set)
2. Configure notification preferences for that user (telegram)
3. Start a conversation, trigger escalation
4. Verify notification is sent to the assigned agent

## Rollout

1. Deploy changes
2. Ensure all Chatwoot agents have corresponding accounts with chatwoot_user_id set
3. Configure DEFAULT_ESCALATION_USER as fallback
4. Monitor logs for "no assignee" warnings

## Future Enhancements

- Team notifications: Notify all team members, not just assignee
- Auto-assignment: When bot escalates, auto-assign to available agent
- Routing rules: Route to different teams based on conversation topic
