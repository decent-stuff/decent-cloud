# Chatwoot SLA & Response Time System
**Status:** Complete

## Requirements
### Must-have
- [x] Provider dashboard link in UI (link to Chatwoot agent dashboard)
- [x] SLA tracking database schema (breach flags, provider config)
- [x] SLA breach detection (background check for unanswered messages)
- [x] SLA breach email alerts to providers
- [x] Response time API endpoint (expose avg response time, SLA compliance)
- [x] Response time display in provider profile UI

### Nice-to-have
- [ ] In-app notifications for SLA breaches
- [ ] Provider-configurable SLA thresholds via UI

## Design Decisions
- **Default SLA threshold:** 4 hours (14400s) - balanced for business hours
- **SLA check frequency:** Every 5 minutes via background job in api-sync
- **Alert timing:** Send alert when SLA breached (not before)
- **Metrics displayed:** Avg response time, SLA compliance %, breach count (30 days)

## Implementation

### Database Schema
- Migration `026_sla_tracking.sql` adds SLA tracking columns and tables

### Backend
- `api/src/database/chatwoot.rs` - SLA tracking and metrics queries
- `api/src/email_processor.rs` - SLA breach alert emails
- `api/src/openapi/providers.rs` - Response metrics API endpoints

### Frontend
- `DashboardSidebar.svelte` - Provider support dashboard link
- `TrustDashboard.svelte` - Response metrics display with SLA compliance

## Completion Summary
Implemented in commits:
- 874d54b feat(chatwoot): add SLA tracking and alerts
- 32a50e7 feat(api): add provider response metrics
- 4008267 refactor: rename provider response metrics endpoints
