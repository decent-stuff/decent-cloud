# Messaging Infrastructure
**Status:** In Progress

## Requirements

### Must-have
- [ ] Database: Message storage with discussion threads, participants, read receipts
- [ ] Database: Message notification queue (triggers email notifications)
- [ ] Backend API: Send message within contract context
- [ ] Backend API: List messages/threads for a contract
- [ ] Backend API: Mark message as read
- [ ] Backend API: Get unread message count
- [ ] Backend API: Provider response time metrics
- [ ] Email: Notification when user receives a new message and it wasn't marked as read
- [ ] Frontend: Message list component
- [ ] Frontend: Message composer component
- [ ] Frontend: Unread badge/indicator
- [ ] Frontend: Contract messages page
- [ ] Frontend: Messages inbox page
- [ ] Integration: Link messages to contracts

### Nice-to-have
- [ ] Message attachments
- [ ] Full-text search on messages
- [ ] Email reply integration (reply to notification sends message)
- [ ] Real-time updates (WebSocket/SSE)

## Architecture

- we should reuse existing messaging packages rather than invent new things here
- consider some package/framework that can be used for AI agents as well in the future because we'll do that soon!

### AI Compatibility Notes

Researched `genai` and `talk` Rust crates. Industry standard message format is:
```rust
ChatMessage { role: "user"|"assistant"|"system", content: "..." }
```

Our schema includes `sender_role` field for future AI agent compatibility:
- `user` = human user (requester or provider)
- `assistant` = AI agent response
- `system` = system-generated messages (status updates, etc.)

This allows the same message infrastructure to support:
1. Human-to-human messaging (current)
2. AI agent messaging (future)
3. Mixed human + AI conversations (future)

### Data Model

```sql
-- Core message storage
CREATE TABLE messages (
    id BLOB PRIMARY KEY,
    thread_id BLOB NOT NULL REFERENCES message_threads(id),
    sender_pubkey TEXT NOT NULL,
    sender_role TEXT NOT NULL DEFAULT 'user',  -- user, assistant, system (AI-compatible)
    body TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES message_threads(id)
);

-- Conversation threads (1 thread per contract, expandable later)
CREATE TABLE message_threads (
    id BLOB PRIMARY KEY,
    contract_id BLOB NOT NULL,
    subject TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    last_message_at_ns INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'open'  -- open, resolved, closed
);

-- Track who's in each thread
CREATE TABLE message_thread_participants (
    thread_id BLOB NOT NULL REFERENCES message_threads(id),
    pubkey TEXT NOT NULL,
    role TEXT NOT NULL,  -- requester, provider
    joined_at_ns INTEGER NOT NULL,
    PRIMARY KEY (thread_id, pubkey)
);

-- Track read status per user
CREATE TABLE message_read_receipts (
    message_id BLOB NOT NULL REFERENCES messages(id),
    reader_pubkey TEXT NOT NULL,
    read_at_ns INTEGER NOT NULL,
    PRIMARY KEY (message_id, reader_pubkey)
);

-- Queue for email notifications about messages
CREATE TABLE message_notifications (
    id BLOB PRIMARY KEY,
    message_id BLOB NOT NULL REFERENCES messages(id),
    recipient_pubkey TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, sent, skipped
    created_at_ns INTEGER NOT NULL,
    sent_at_ns INTEGER
);
```

### Rust Types

```rust
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    pub sender_pubkey: String,
    pub body: String,
    pub created_at_ns: i64,
    pub is_read: bool,  // Computed for current user
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct MessageThread {
    pub id: String,
    pub contract_id: String,
    pub subject: String,
    pub created_at_ns: i64,
    pub last_message_at_ns: i64,
    pub status: String,
    pub unread_count: i64,  // Computed for current user
    pub message_count: i64,
}

#[derive(Debug, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct ProviderResponseMetrics {
    pub avg_response_time_hours: Option<f64>,
    pub response_rate_pct: f64,
    pub total_threads: i64,
    pub responded_threads: i64,
}
```

### API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/contracts/{id}/messages` | List all messages for a contract |
| POST | `/contracts/{id}/messages` | Send a message in contract context |
| GET | `/contracts/{id}/thread` | Get thread metadata for contract |
| PUT | `/messages/{id}/read` | Mark message as read |
| GET | `/messages/unread-count` | Get total unread count for user |
| GET | `/messages/inbox` | List all threads for user (paginated) |
| GET | `/providers/{pubkey}/response-metrics` | Provider response time stats |

### Email Notification Flow

1. User sends message via `POST /contracts/{id}/messages`
2. Backend creates message + `message_notifications` entry
3. Email processor picks up notification, looks up recipient email from `account_contacts`
4. Queues email via existing `email_queue` with type `MessageNotification`
5. Email contains: sender name, contract reference, message preview, link to view

### Frontend Components

| Component | Purpose |
|-----------|---------|
| `MessageList.svelte` | Display messages in a thread |
| `MessageComposer.svelte` | Text input + send button |
| `MessageBubble.svelte` | Single message display (sender-aware styling) |
| `UnreadBadge.svelte` | Shows unread count |
| `ThreadListItem.svelte` | Thread preview for inbox |

### Frontend Pages

| Route | Purpose |
|-------|---------|
| `/dashboard/messages` | Inbox - all threads |
| `/dashboard/rentals/[id]/messages` | Messages for specific contract |

## Steps

### Step 1: Database Schema + Backend DB Layer
**Success:** Migration runs, Rust types compile, basic CRUD queries work
**Status:** Complete

Files:
- `api/migrations/022_messaging.sql` ✓
- `api/src/database/messages.rs` ✓
- `api/src/database/mod.rs` (update) ✓
- `api/src/database/test_helpers.rs` (update) ✓

### Step 2: Backend API Endpoints
**Success:** All endpoints return correct responses, integration tests pass
**Status:** Pending

Files:
- `api/src/openapi/messages.rs`
- `api/src/openapi/mod.rs` (update)
- `api/src/main.rs` (update routes)

### Step 3: Email Notification Integration
**Success:** Sending message queues email notification, emails sent correctly
**Status:** Pending

Files:
- `api/src/database/email.rs` (add MessageNotification type)
- `api/src/database/messages.rs` (notification queueing)
- `api/src/email_processor.rs` (process message notifications)

### Step 4: Frontend Service Layer
**Success:** TypeScript types generated, API client works, store manages state
**Status:** Pending

Files:
- `website/src/lib/services/message-api.ts`
- `website/src/lib/stores/messages.ts`
- `website/src/lib/types/generated/` (auto-generated)

### Step 5: Frontend Components
**Success:** Components render correctly, send/receive messages works
**Status:** Pending

Files:
- `website/src/lib/components/MessageList.svelte`
- `website/src/lib/components/MessageComposer.svelte`
- `website/src/lib/components/MessageBubble.svelte`
- `website/src/lib/components/UnreadBadge.svelte`
- `website/src/lib/components/ThreadListItem.svelte`

### Step 6: Frontend Pages + Integration
**Success:** Full messaging flow works end-to-end, E2E tests pass
**Status:** Pending

Files:
- `website/src/routes/dashboard/messages/+page.svelte`
- `website/src/routes/dashboard/rentals/[id]/messages/+page.svelte`
- `website/src/routes/dashboard/rentals/[id]/+page.svelte` (add messages link)

## Execution Log

### Step 1
- **Implementation:** COMPLETE
  - Created migration 022_messaging.sql with all tables (message_threads, messages, message_thread_participants, message_read_receipts, message_notifications)
  - Implemented api/src/database/messages.rs with all required functions:
    - `create_thread()` - creates thread with participants
    - `get_thread_by_contract()` - retrieves thread by contract_id
    - `create_message()` - creates message and updates thread timestamp
    - `get_messages_for_thread()` - fetches messages with read status
    - `mark_message_read()` - marks message as read (idempotent)
    - `get_unread_count()` - counts unread messages for user
    - `get_threads_for_user()` - lists all threads for user
    - `queue_message_notification()` - queues notification for email
  - Added ts-rs exports for TypeScript type generation
  - Updated api/src/database/mod.rs to include messages module
  - Updated api/src/database/test_helpers.rs to include migration 022
- **Testing:** COMPLETE - All 20 unit tests passing
  - Thread creation (including duplicate prevention)
  - Message creation and retrieval
  - Read receipts and idempotent marking
  - Unread count calculation (excludes own messages)
  - Thread listing and sorting
  - Notification queueing
- **Outcome:** SUCCESS - Database layer complete and fully tested

### Step 2
- **Implementation:** COMPLETE
  - Created `api/src/openapi/messages.rs` with all required endpoints:
    - `GET /contracts/{id}/messages` - List messages (with auto-thread creation)
    - `POST /contracts/{id}/messages` - Send message (with notification queueing)
    - `GET /contracts/{id}/thread` - Get thread metadata
    - `PUT /messages/{id}/read` - Mark message as read
    - `GET /messages/unread-count` - Get unread count
    - `GET /messages/inbox` - List all threads for user
    - `GET /providers/{pubkey}/response-metrics` - Provider response statistics
  - All endpoints use proper authentication via `ApiAuthenticatedUser`
  - Contract participant verification (requester/provider only)
  - Thread auto-creation when sending first message
  - Notification queueing for recipients when messages sent
  - Added `Messages` tag to `ApiTags` enum in common.rs
  - Updated `api/src/openapi.rs` to include messages module and API registration
- **Testing:** COMPLETE - All tests passing (832 tests, 0 failures)
  - Cargo make clean run with no errors or warnings
  - Integration with existing database layer verified
- **Outcome:** SUCCESS - All message API endpoints implemented and tested

### Step 3
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 4
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 5
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 6
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

## Completion Summary
(To be filled in Phase 4)
