# Support Bot & Notification System Implementation
**Status:** In Progress

## Requirements

### Must-have
- [x] Database migration for provider notification preferences (support_config)
- [ ] Chatwoot Help Center article fetching via API
- [ ] AI Bot Service: keyword search + LLM answer generation
- [ ] Escalation trigger (bot sets conversation status to "open")
- [ ] Notification Bridge: conversation_status_changed webhook handler
- [ ] Telegram notification sending
- [ ] Telegram reply webhook receiver
- [ ] Post Telegram replies back to Chatwoot conversation

### Nice-to-have
- [ ] Semantic search (embeddings) for articles
- [ ] SMS notifications via Twilio
- [ ] Article caching with TTL

## Steps

### Step 1: Database Migration for Provider Notification Config
**Success:** Migration runs, `provider_notification_config` table exists with proper schema
**Status:** COMPLETE

Add table for provider notification preferences:
- `provider_pubkey` (BLOB, FK to provider_profiles)
- `chatwoot_portal_slug` (TEXT)
- `notify_via` (TEXT: "telegram" | "sms" | "email")
- `telegram_chat_id` (TEXT, nullable)
- `notify_phone` (TEXT, nullable)

### Step 2: Database Layer for Notification Config CRUD
**Success:** Functions to get/set provider notification config with tests
**Status:** Pending

Add to `api/src/database/`:
- `get_provider_notification_config(pubkey) -> Option<NotificationConfig>`
- `set_provider_notification_config(pubkey, config) -> Result<()>`

### Step 3: Chatwoot Help Center Client
**Success:** Can fetch articles from Help Center API, with tests
**Status:** Pending

Extend `ChatwootClient` with:
- `fetch_help_center_articles(portal_slug) -> Vec<Article>`
- Article struct: `{id, title, content, slug}`

### Step 4: Article Search Service
**Success:** Keyword search returns relevant articles, with tests
**Status:** Pending

Create `api/src/support_bot/search.rs`:
- `search_articles(query, articles) -> Vec<ScoredArticle>`
- Keyword matching with relevance scoring
- Return top N matches above threshold

### Step 5: LLM Integration for Answer Generation
**Success:** Can generate answer from articles via Claude/OpenAI API
**Status:** Pending

Create `api/src/support_bot/llm.rs`:
- `generate_answer(question, articles) -> BotResponse`
- BotResponse: `{answer, sources: Vec<ArticleRef>, confidence}`
- If confidence < threshold, mark for escalation

### Step 6: AI Bot Webhook Handler
**Success:** Bot responds to messages, escalates when needed
**Status:** Pending

Extend `chatwoot_webhook` or add new handler:
- On `message_created` from customer
- Fetch articles for provider's portal
- Search + generate answer
- Reply via Chatwoot API
- If "human" keyword or low confidence → escalate

### Step 7: Notification Bridge - Status Change Handler
**Success:** Provider notified via Telegram on escalation
**Status:** Pending

Add webhook handler for `conversation_status_changed`:
- Filter for status → "open" (human handoff)
- Lookup provider notification config
- Send notification via configured channel

### Step 8: Telegram Bot Integration
**Success:** Can send messages and receive replies via Telegram
**Status:** Pending

Create `api/src/notifications/telegram.rs`:
- `send_message(chat_id, message) -> Result<()>`
- Webhook handler for incoming messages
- Link replies back to Chatwoot conversation

### Step 9: Post Replies to Chatwoot
**Success:** Telegram replies appear in Chatwoot conversation
**Status:** Pending

Extend `ChatwootClient`:
- `send_message(conversation_id, content, sender_type) -> Result<()>`
- Route Telegram replies to correct conversation

### Step 10: OpenAPI Endpoints for Config Management
**Success:** Providers can manage notification preferences via API
**Status:** Pending

Add endpoints:
- `GET /api/v1/providers/me/notification-config`
- `PUT /api/v1/providers/me/notification-config`

## Execution Log

### Step 1
- **Implementation:** Created migration `028_provider_notification_config.sql` with table schema including:
  - `provider_pubkey` (BLOB PRIMARY KEY, FK to provider_profiles.pubkey)
  - `chatwoot_portal_slug` (TEXT)
  - `notify_via` (TEXT with CHECK constraint: telegram/sms/email)
  - `telegram_chat_id` (TEXT, nullable)
  - `notify_phone` (TEXT, nullable)
  - `created_at` and `updated_at` (INTEGER timestamps)
- **Files:** `/code/api/migrations/028_provider_notification_config.sql`
- **Verification:** Migration applied successfully via `cargo make sqlx-prepare` (2.001577ms runtime)
- **Outcome:** SUCCESS - Table created with proper schema, idempotent-safe with `CREATE TABLE IF NOT EXISTS`, query cache updated

### Step 2
- **Implementation:** Created database layer for provider notification config CRUD operations with:
  - `ProviderNotificationConfig` struct with fields: provider_pubkey, chatwoot_portal_slug, notify_via, telegram_chat_id, notify_phone
  - `get_provider_notification_config(pubkey)` - fetches config by provider pubkey, returns Option
  - `set_provider_notification_config(pubkey, config)` - creates or updates config using UPSERT pattern
  - Module added to `/code/api/src/database/notification_config.rs` and exported from mod.rs
- **Files:**
  - `/code/api/src/database/notification_config.rs` (new, 230 lines)
  - `/code/api/src/database/mod.rs` (updated exports)
  - `/code/api/src/database/test_helpers.rs` (added migration 028)
- **Tests:** 5 unit tests covering:
  - Get config when not exists (returns None)
  - Set and get config (positive path)
  - Update existing config (upsert behavior)
  - Invalid notify_via value (CHECK constraint enforcement)
  - Nonexistent provider (foreign key constraint enforcement)
- **Verification:** `cargo make` passed cleanly - all tests pass, no compilation warnings or errors
- **Outcome:** SUCCESS - Database layer fully functional with proper error handling and constraint validation

### Step 3
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 4
- **Implementation:** Created article search service with simple keyword-based matching:
  - `HelpCenterArticle` struct: id (u64), title, content, slug
  - `ScoredArticle` struct: article, score (f32, 0.0-1.0)
  - `search_articles(query, articles)` - tokenizes query to lowercase keywords, counts matches in title (2x weight) and content (1x weight), normalizes scores, filters by threshold (0.1), returns sorted by score descending
  - Simple tokenization: lowercase, split on whitespace
  - Module created at `/code/api/src/support_bot/search.rs` and exported from `mod.rs`
- **Files:**
  - `/code/api/src/support_bot/search.rs` (new, 212 lines)
  - `/code/api/src/support_bot/mod.rs` (updated to export search module)
  - `/code/api/src/chatwoot/client.rs` (fixed Deserialize on ListHelpCenterArticlesResponse)
  - `/code/api/src/support_bot/llm.rs` (updated to use search::HelpCenterArticle and search::ScoredArticle)
- **Tests:** 10 unit tests covering:
  - Empty query returns empty results
  - Whitespace-only query returns empty results
  - Empty articles list returns empty results
  - No matches returns empty results
  - Single match found and scored
  - Title weight higher than content weight (ranking verification)
  - Multiple keywords ranked correctly (both in title ranks highest)
  - Case-insensitive matching
  - Tokenization correctness
  - Score normalization (0.0-1.0 range)
- **Verification:** `SQLX_OFFLINE=true cargo test --lib support_bot::search::tests` - all 10 tests pass cleanly, `SQLX_OFFLINE=true cargo clippy --lib` - no warnings for support_bot code
- **Outcome:** SUCCESS - Search returns correctly ranked results using simple keyword matching (KISS, MINIMAL, YAGNI principles followed), tests pass, no clippy warnings

### Step 5
- **Implementation:** Created LLM integration for answer generation with:
  - `BotResponse` struct with fields: answer (String), sources (Vec<ArticleRef>), confidence (f32, 0.0-1.0), should_escalate (bool)
  - `ArticleRef` struct with fields: title, slug
  - `generate_answer(question, articles)` - main function that:
    - Detects escalation keywords ("human", "agent") and immediately escalates
    - Returns escalation response if no articles provided
    - Builds prompt with top 3 articles (configurable via MAX_ARTICLES_IN_PROMPT)
    - Calls Claude API via `call_llm_api(prompt)`
    - Calculates confidence as average of article scores (capped at 1.0)
    - Sets should_escalate = true if confidence < 0.5 (LOW_CONFIDENCE_THRESHOLD)
    - Returns BotResponse with answer, article sources, confidence, and escalation flag
  - `build_prompt(question, articles)` - constructs prompt with knowledge base articles and user question
  - `call_llm_api(prompt)` - calls Claude API (Anthropic) with:
    - Reads LLM_API_KEY from environment (required)
    - Reads LLM_API_URL from environment (defaults to "https://api.anthropic.com/v1/messages")
    - Uses claude-3-5-sonnet-20241022 model
    - Max tokens: 1024
    - Returns extracted text from first content block
    - On API error: logs error and returns escalation response (fail-safe)
  - Module created at `/code/api/src/support_bot/llm.rs` and exported from `mod.rs`
- **Files:**
  - `/code/api/src/support_bot/llm.rs` (new, 270 lines)
  - `/code/api/src/support_bot/mod.rs` (already exporting llm module)
  - `/code/api/src/lib.rs` (updated to export support_bot module)
- **Tests:** 7 unit tests covering:
  - Escalate on "human" keyword in question
  - Escalate on "agent" keyword in question
  - Escalate when no articles provided
  - Confidence calculation from article scores
  - Low confidence (< 0.5) triggers escalation
  - Prompt building includes article titles and content
  - Prompt limits to MAX_ARTICLES_IN_PROMPT (3 articles max)
- **Verification:** `cargo make` passed cleanly with exit code 0 - all tests pass (including 7 LLM tests), no compilation errors or warnings
- **Outcome:** SUCCESS - LLM integration compiles, handles errors gracefully (escalates on API failure), tests pass, follows KISS/MINIMAL principles

### Step 6
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 7
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 8
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 9
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 10
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

## Completion Summary
(To be filled in Phase 4)
