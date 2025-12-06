# Support Bot Architecture

## Overview

Two-tier customer support system:
- **L1 (AI Bot)**: Answers questions from provider's Help Center articles
- **L2 (Provider)**: Human escalation via email/Telegram/SMS

## Flow

```
Customer message → Chatwoot Widget → Inbox → Agent Bot webhook
                                                    ↓
                            ┌───────────────────────────────────────┐
                            │ api/src/openapi/webhooks.rs           │
                            │ chatwoot_webhook() handler            │
                            │   1. Receive message_created event    │
                            │   2. Extract message content          │
                            └───────────────────────────────────────┘
                                                    ↓
                            ┌───────────────────────────────────────┐
                            │ api/src/support_bot/handler.rs        │
                            │ handle_customer_message()             │
                            │   1. Discover all Help Center portals │
                            │   2. Fetch articles from all portals  │
                            │   3. Semantic search for relevance    │
                            │   4. Generate answer via LLM          │
                            │   5. Respond OR escalate              │
                            └───────────────────────────────────────┘
                                                    ↓
                    ┌───────────────────┴───────────────────┐
                    │                                       │
              [Confident]                            [Escalate]
              Bot replies                     Sets status="open"
                                              Notify DEFAULT_ESCALATION_USER
                                                    ↓
                            ┌───────────────────────────────────────┐
                            │ api/src/support_bot/notifications.rs  │
                            │ dispatch_notification()               │
                            │   1. Lookup user notification config  │
                            │   2. Send via Telegram/Email/SMS      │
                            └───────────────────────────────────────┘
                                                    ↓
                            DEFAULT_ESCALATION_USER receives alert
                            User replies in Chatwoot
                            Customer sees response in widget
```

## Key Design Decisions

### Simplified Support Bot
- Bot automatically fetches articles from ALL Help Center portals
- No contract or provider lookup required for bot operation
- Escalations notify `DEFAULT_ESCALATION_USER` (configurable)
- `contract_id` is still tracked in custom_attributes for analytics but not used by bot
- Notification preferences are stored per user and looked up by username or pubkey

### Agent Bot Configuration
The agent bot MUST be:
1. Created with `account_id` (not platform-level)
2. Assigned to the inbox via `CHATWOOT_INBOX_ID`

Without inbox assignment, webhooks won't fire. See `api/src/main.rs` serve_command.

### Escalation Triggers
Bot escalates when:
- User says "human" or "agent"
- No relevant articles found
- LLM confidence below threshold (0.5)
- LLM API fails

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `CHATWOOT_BASE_URL` | Yes | Chatwoot API base (e.g., `http://chatwoot-web:59002`) |
| `CHATWOOT_API_TOKEN` | Yes | Account API token for sending messages |
| `CHATWOOT_PLATFORM_API_TOKEN` | Yes | Platform token for agent bot management |
| `CHATWOOT_ACCOUNT_ID` | Yes | Account ID (usually `1`) |
| `CHATWOOT_INBOX_ID` | Yes | Inbox to assign bot to |
| `DEFAULT_ESCALATION_USER` | No | Username to notify on escalation (e.g., `admin`) |
| `API_PUBLIC_URL` | Yes | Public URL for webhook callbacks |
| `LLM_API_KEY` | No | Anthropic API key (bot disabled if missing) |
| `LLM_API_URL` | No | Custom LLM endpoint |
| `LLM_API_MODEL` | No | Model name (default: claude-4.5-sonnet) |
| `TELEGRAM_BOT_TOKEN` | No | For Telegram notifications |
| `MAILCHANNELS_API_KEY` | No | For email notifications |

## Files

- `handler.rs` - Main message handling logic
- `search.rs` - Article search (keyword + semantic)
- `llm.rs` - LLM prompt building and API calls
- `notifications.rs` - Escalation notification dispatch
- `embeddings.rs` - Semantic embedding generation
- `test_notifications.rs` - Test notification endpoints

## Common Issues

### Bot not receiving webhooks
1. Check agent bot has `account_id` set (not platform-level)
2. Check agent bot is assigned to inbox
3. Verify `API_PUBLIC_URL` is reachable from Chatwoot

### Bot not responding
1. Check `LLM_API_KEY` is set
2. Check Help Center portals exist and have articles in Chatwoot
3. Review logs for portal discovery or article fetch errors

### Escalation not notifying
1. Check `DEFAULT_ESCALATION_USER` is set and user exists
2. Check user has notification config in DB (telegram_chat_id or email)
3. Check `TELEGRAM_BOT_TOKEN` or `MAILCHANNELS_API_KEY` is set
4. Review logs for notification dispatch errors

### Missing configuration warnings
If no Help Center portals are found, bot will escalate all conversations immediately.
If `DEFAULT_ESCALATION_USER` is not set, no notifications will be sent on escalation.
