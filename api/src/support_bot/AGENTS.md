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
                            │   1. Extract contract_id from attrs   │
                            │   2. Lookup contract → provider       │
                            │   3. Get provider's portal_slug       │
                            └───────────────────────────────────────┘
                                                    ↓
                            ┌───────────────────────────────────────┐
                            │ api/src/support_bot/handler.rs        │
                            │ handle_customer_message()             │
                            │   1. Fetch Help Center articles       │
                            │   2. Semantic search for relevance    │
                            │   3. Generate answer via LLM          │
                            │   4. Respond OR escalate              │
                            └───────────────────────────────────────┘
                                                    ↓
                    ┌───────────────────┴───────────────────┐
                    │                                       │
              [Confident]                            [Escalate]
              Bot replies                     Sets status="open"
                                                    ↓
                            ┌───────────────────────────────────────┐
                            │ Chatwoot sends conversation_status_   │
                            │ changed webhook                       │
                            └───────────────────────────────────────┘
                                                    ↓
                            ┌───────────────────────────────────────┐
                            │ api/src/support_bot/notifications.rs  │
                            │ dispatch_notification()               │
                            │   1. Lookup provider notification cfg │
                            │   2. Send via Telegram/Email/SMS      │
                            └───────────────────────────────────────┘
                                                    ↓
                            Provider receives alert with Chatwoot link
                            Provider replies in Chatwoot or Telegram
                            Customer sees response in widget
```

## Key Design Decisions

### Single Inbox, Multi-Provider
- One Chatwoot inbox handles ALL customer conversations
- Each conversation is tagged with `contract_id` (set by widget)
- `contract_id` → lookup contract → get `provider_pubkey`
- Provider's `chatwoot_portal_slug` determines which Help Center to use
- Provider's notification preferences determine escalation channel

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
2. Check provider has `chatwoot_portal_slug` configured
3. Check portal has articles

### Escalation not notifying
1. Check provider notification config in DB
2. Check `TELEGRAM_BOT_TOKEN` or `MAILCHANNELS_API_KEY` set
3. Check provider's telegram_chat_id or email is configured
