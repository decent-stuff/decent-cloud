---
description: PostgreSQL access and management in agent container
---

# PostgreSQL in Agent Container

A PostgreSQL 16 instance is automatically available when running in the agent container.

## IMPORTANT: Use the environment variable

```bash
# ALWAYS use this - it has correct host/credentials:
psql $DATABASE_URL_PG
```

**DO NOT use localhost** - postgres runs in a separate container. The hostname is `postgres` (docker service name).

**Credentials** (already set in `$DATABASE_URL_PG`):
- Host: `postgres` (NOT localhost!)
- Port: `5432`
- User: `test`
- Password: `test`
- Database: `test`

## Verify Connection

```bash
psql $DATABASE_URL_PG -c "SELECT 1;"
```

## Reset Database

```bash
# Drop and recreate schema
psql $DATABASE_URL_PG -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
```

## Run Migrations

```bash
DATABASE_URL=$DATABASE_URL_PG sqlx migrate run --source api/migrations_pg
```
