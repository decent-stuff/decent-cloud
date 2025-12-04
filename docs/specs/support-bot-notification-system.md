# Support Bot & Provider Notification System

**Stack: Chatwoot (MIT) + Custom AgentBot + Notification Bridge**

## Architecture

```
User starts chat → Chatwoot widget → AgentBot webhook → AI Bot Service
                                                              ↓
                                    Fetch articles from Chatwoot Help Center
                                         GET /hc/:provider-slug/en/articles
                                                              ↓
                                    Search articles (keyword + semantic)
                                                              ↓
                              ┌─────────────────────────────────────────┐
                              │ Found relevant articles?                │
                              │   YES → Generate answer + cite sources  │
                              │   NO  → Escalate to human               │
                              └─────────────────────────────────────────┘
                                                              ↓
                                          [If escalating]
                                          Bot sets status="open"
                                                              ↓
                                    Chatwoot webhook (conversation_status_changed)
                                                              ↓
                                          Notification Bridge
                                                              ↓
                            Lookup provider contact preferences (our DB)
                                                              ↓
                    Send notification via Telegram Bot API / Twilio SMS
                                                              ↓
                              Provider replies in Telegram/SMS
                                                              ↓
                    Notification Bridge posts reply to Chatwoot API
                                                              ↓
                              User sees provider response in widget
```

## Components

### 1. AI Bot Service (~200 lines Rust)

Webhook endpoint for Chatwoot AgentBot:

```
POST /webhook/chatwoot
  ← message_created event
  → Fetch provider's Help Center articles
  → Search for relevant content
  → Generate answer with LLM
  → Respond via Chatwoot API with answer + source links
  → Or escalate if low confidence / user requests human
```

**Response format to user:**
```
Based on the documentation:

[Generated answer from articles]

Sources:
• [Article Title 1](/hc/provider/en/articles/setup-guide)
• [Article Title 2](/hc/provider/en/articles/pricing-faq)

Need more help? Type "human" to speak with the provider.
```

**Search approach:**
1. Fetch all articles from provider's portal (cache with TTL)
2. Keyword match first (fast, exact)
3. If no match → semantic search (embed question + articles, cosine similarity)
4. If top match score < threshold → escalate

### 2. Notification Bridge (~150 lines Rust)

- Listens to Chatwoot `conversation_status_changed` webhook
- On status change to "open" (human handoff):
  - Query our DB for provider's contact preferences
  - Send notification with conversation summary + link
- Receives replies via Telegram/Twilio webhooks
- Posts replies back to Chatwoot conversation

### 3. Provider Knowledge Base

**No separate storage needed** - uses Chatwoot Help Center natively.

Provider workflow:
1. Log into Chatwoot dashboard
2. Go to Help Center → Create Portal (e.g., `acme-hosting`)
3. Add categories and articles
4. Bot automatically uses them

## Chatwoot Setup

1. Self-host Chatwoot (MIT, free)
2. Create AgentBot via API:
   ```
   POST /api/v1/accounts/:id/agent_bots
   {
     "name": "Support Bot",
     "outgoing_url": "https://our-api/webhook/chatwoot"
   }
   ```
3. Attach bot to inbox (conversations start in "pending" with bot)
4. Configure webhook for `conversation_status_changed` → Notification Bridge
5. Each provider creates their Help Center portal

## Database Schema (our side)

```sql
-- Provider notification preferences
ALTER TABLE providers ADD COLUMN support_config JSONB;

-- Example:
-- {
--   "chatwoot_portal_slug": "acme-hosting",
--   "chatwoot_agent_id": 123,
--   "notify_via": "telegram",
--   "telegram_chat_id": "123456789",
--   "notify_phone": "+1234567890"
-- }
```

## API Dependencies

**Chatwoot APIs used:**
- `GET /hc/:slug/:locale/articles` - Fetch KB articles (public)
- `POST /api/v1/accounts/:id/conversations/:id/messages` - Send bot reply
- `PATCH /api/v1/accounts/:id/conversations/:id` - Change status (escalate)
- Webhooks: `message_created`, `conversation_status_changed`

**External APIs:**
- Telegram Bot API - Send notifications, receive replies
- Twilio (optional) - SMS notifications
- OpenAI/Claude API - Generate answers from articles
