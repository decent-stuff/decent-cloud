# Chatwoot SLA & Response Time System
**Status:** In Progress

## Requirements
### Must-have
- [ ] Provider dashboard link in UI (link to Chatwoot agent dashboard)
- [ ] SLA tracking database schema (breach flags, provider config)
- [ ] SLA breach detection (background check for unanswered messages)
- [ ] SLA breach email alerts to providers
- [ ] Response time API endpoint (expose avg response time, SLA compliance)
- [ ] Response time display in provider profile UI

### Nice-to-have
- [ ] In-app notifications for SLA breaches
- [ ] Provider-configurable SLA thresholds via UI

## Design Decisions
- **Default SLA threshold:** 4 hours (14400s) - balanced for business hours
- **SLA check frequency:** Every 5 minutes via background job in api-sync
- **Alert timing:** Send alert when SLA breached (not before)
- **Metrics displayed:** Avg response time, SLA compliance %, breach count (30 days)

## Steps
### Step 1: Provider Dashboard Link
**Success:** Link visible in provider dashboard pointing to Chatwoot
**Status:** Pending

### Step 2: Database Schema for SLA
**Success:** Migration adds sla columns and provider_sla_config table
**Status:** Pending

### Step 3: SLA Breach Detection Logic
**Success:** Background job detects and marks breaches
**Status:** Pending

### Step 4: SLA Alert Emails
**Success:** Email sent to provider on breach
**Status:** Pending

### Step 5: Response Time API Endpoint
**Success:** GET /api/v1/providers/{id}/response-metrics returns data
**Status:** Pending

### Step 6: Response Time UI Display
**Success:** Provider profile shows response time metrics
**Status:** Pending

## Execution Log
(filled during implementation)

## Completion Summary
(filled when done)
