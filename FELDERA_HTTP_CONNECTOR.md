# Feldera HTTP Connector Guide

This guide covers how to stream PostgreSQL logical replication changes directly to Feldera pipelines using the HTTP ingress connector.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [How It Works](#how-it-works)
- [Examples](#examples)
- [Multiple Targets](#multiple-targets)
- [Authentication](#authentication)
- [Error Handling](#error-handling)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Overview

The Feldera HTTP connector enables direct streaming from PostgreSQL to Feldera pipelines via the HTTP ingress API. This integration allows you to:

- Build real-time data pipelines with streaming SQL
- Perform incremental view maintenance on PostgreSQL data
- Run streaming analytics and transformations
- Synchronize data between PostgreSQL and other systems
- Build event-driven architectures

## Quick Start

### 1. Prerequisites

- PostgreSQL with logical replication enabled
- Feldera instance running (local or cloud)
- A Feldera pipeline with a table matching your PostgreSQL schema

### 2. Basic Example

```bash
# Build the tool
cargo build --release

# Stream PostgreSQL changes to Feldera
./target/release/pgoutput-cmdline \
  --connection "host=localhost user=postgres dbname=mydb replication=database" \
  --slot feldera_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "postgres_cdc" \
  --feldera-table "users" \
  --create-slot
```

### 3. Verify in Feldera

Check the Feldera web UI or use the API to verify data is flowing:

```bash
# Query the pipeline status
curl http://localhost:8080/v0/pipelines/postgres_cdc

# Query the table data
curl http://localhost:8080/v0/pipelines/postgres_cdc/egress/users
```

## Configuration

### Required Arguments

When `--target` includes `feldera`, these arguments are required:

| Argument | Description | Example |
|----------|-------------|---------|
| `--feldera-url` | Base URL of Feldera instance | `http://localhost:8080` |
| `--feldera-pipeline` | Name of the pipeline | `postgres_cdc` |
| `--feldera-table` | Name of the table in pipeline | `users` |

### Optional Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--feldera-api-key` | API key for authentication | None (no auth) |
| `--format` | Must be `feldera` for HTTP connector | `json` |

### Connection URL Format

The connector constructs the ingress URL automatically:

```
{feldera-url}/v0/pipelines/{pipeline}/ingress/{table}?format=json&update_format=insert_delete&array=true
```

Example:
```
http://localhost:8080/v0/pipelines/postgres_cdc/ingress/users?format=json&update_format=insert_delete&array=true
```

The `array=true` parameter allows UPDATE operations to be sent as a JSON array containing both delete and insert events in a single HTTP request.

## How It Works

### Event Conversion

PostgreSQL replication events are converted to Feldera InsertDelete format:

1. **INSERT** → Single-element array
   ```json
   [{"insert": {"id": 1, "name": "Alice", "email": "alice@example.com"}}]
   ```

2. **UPDATE** → Two-element array (delete old + insert new)
   ```json
   [
     {"delete": {"id": 1, "name": "Alice", "email": "alice@example.com"}},
     {"insert": {"id": 1, "name": "Alice", "email": "alice.updated@example.com"}}
   ]
   ```

3. **DELETE** → Single-element array
   ```json
   [{"delete": {"id": 1, "name": "Alice", "email": "alice@example.com"}}]
   ```

All events are sent as JSON arrays because the `array=true` parameter is used.

### Filtered Events

These PostgreSQL events are NOT sent to Feldera:
- `BEGIN` (transaction start)
- `COMMIT` (transaction end)
- `RELATION` (schema metadata)

Only data changes (INSERT, UPDATE, DELETE) are streamed.

### HTTP Request Details

- **Method**: POST
- **Content-Type**: application/json
- **Body**: Single JSON event or array of events (for UPDATEs) in InsertDelete format
- **Authentication**: Optional Bearer token via `--feldera-api-key`
- **Array Support**: UPDATE operations send arrays with `array=true` parameter

## Examples

### Example 1: Local Development

Stream from local PostgreSQL to local Feldera:

```bash
pgoutput-cmdline \
  --connection "host=localhost user=postgres dbname=mydb replication=database" \
  --slot dev_slot \
  --publication all_tables \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "dev_pipeline" \
  --feldera-table "users" \
  --create-slot
```

### Example 2: Cloud Deployment

Stream to Feldera Cloud with authentication:

```bash
pgoutput-cmdline \
  --connection "host=db.example.com user=replicator dbname=prod sslmode=require replication=database" \
  --slot prod_feldera_slot \
  --publication prod_pub \
  --format feldera \
  --target feldera \
  --feldera-url "https://cloud.feldera.com" \
  --feldera-pipeline "production_pipeline" \
  --feldera-table "events" \
  --feldera-api-key "${FELDERA_API_KEY}"
```

### Example 3: Complete Pipeline Setup

**Step 1: Set up PostgreSQL**

```sql
-- Enable logical replication in postgresql.conf
-- wal_level = logical

-- Create publication
CREATE PUBLICATION feldera_pub FOR TABLE users, orders;

-- Set replica identity for full row capture
ALTER TABLE users REPLICA IDENTITY FULL;
ALTER TABLE orders REPLICA IDENTITY FULL;
```

**Step 2: Create Feldera Pipeline**

```sql
-- In Feldera SQL editor
CREATE TABLE users (
    id INT,
    name VARCHAR,
    email VARCHAR,
    created_at TIMESTAMP
);

CREATE TABLE orders (
    id INT,
    user_id INT,
    product VARCHAR,
    amount DECIMAL(10,2),
    order_date TIMESTAMP
);

-- Example view
CREATE VIEW user_orders AS
SELECT 
    u.name,
    u.email,
    COUNT(o.id) as order_count,
    SUM(o.amount) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name, u.email;
```

**Step 3: Stream Data**

```bash
# Terminal 1: Stream users table
pgoutput-cmdline \
  --connection "..." \
  --slot users_slot \
  --publication feldera_pub \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "users" \
  --create-slot

# Terminal 2: Stream orders table
pgoutput-cmdline \
  --connection "..." \
  --slot orders_slot \
  --publication feldera_pub \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "orders" \
  --create-slot
```

**Step 4: Query Results**

```bash
# Query the view in Feldera
curl http://localhost:8080/v0/pipelines/my_pipeline/egress/user_orders | jq
```

## Multiple Targets

Stream to Feldera and other targets simultaneously using comma-separated values:

### Feldera + Stdout

```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target "stdout,feldera" \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "users"
```

### Feldera + NATS + Stdout

```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target "stdout,nats,feldera" \
  --nats-server "nats://localhost:4222" \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "users"
```

This allows you to:
- Monitor events in real-time (stdout)
- Archive events for replay (NATS)
- Process events in streaming pipeline (Feldera)

## Authentication

### API Key Authentication

Feldera supports Bearer token authentication:

```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "https://secure.feldera.com" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "users" \
  --feldera-api-key "your-api-key-here"
```

### Environment Variables

For security, use environment variables:

```bash
export FELDERA_API_KEY="your-api-key-here"

pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "https://secure.feldera.com" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "users" \
  --feldera-api-key "${FELDERA_API_KEY}"
```

## Error Handling

The connector implements robust error handling:

### Connection Errors

If the Feldera server is unreachable:
```
Error: Failed to send data to Feldera: error sending request for url (...)
```

**Solutions:**
- Verify Feldera server is running
- Check network connectivity
- Verify firewall rules
- Check URL is correct (http:// or https://)

### Authentication Errors

HTTP 401 Unauthorized:
```
Error: Feldera ingress API returned error status 401: Unauthorized
```

**Solutions:**
- Verify API key is correct
- Check API key has necessary permissions
- Ensure API key is properly formatted

### Pipeline/Table Errors

HTTP 404 Not Found:
```
Error: Feldera ingress API returned error status 404: Not Found
```

**Solutions:**
- Verify pipeline exists and is running
- Check pipeline name for typos
- Verify table exists in pipeline SQL
- Ensure table name matches exactly (case-sensitive)

### Format Errors

HTTP 400 Bad Request:
```
Error: Feldera ingress API returned error status 400: Invalid JSON format
```

**Solutions:**
- Ensure `--format feldera` is specified
- Verify PostgreSQL schema matches Feldera schema
- Check data type compatibility
- Review Feldera logs for details

## Troubleshooting

### Common Issues

#### 1. No Data Flowing

**Symptom**: Tool runs but no data appears in Feldera

**Checks:**
- Verify publication includes the table: `SELECT * FROM pg_publication_tables WHERE pubname = 'my_pub';`
- Check replication slot is active: `SELECT * FROM pg_replication_slots WHERE slot_name = 'my_slot';`
- Ensure PostgreSQL changes are being made to published tables
- Verify pipeline is running in Feldera

#### 2. Schema Mismatch

**Symptom**: 400 errors about data format

**Solution:**
- Ensure PostgreSQL and Feldera schemas match
- Check column names (case-sensitive)
- Verify data types are compatible
- Use `REPLICA IDENTITY FULL` for complete UPDATE events

#### 3. High Latency

**Symptom**: Delays between PostgreSQL changes and Feldera updates

**Checks:**
- Network latency between PostgreSQL and Feldera
- Feldera pipeline performance
- PostgreSQL replication lag: `SELECT * FROM pg_stat_replication;`
- Check for CPU/memory constraints

#### 4. Connection Drops

**Symptom**: Tool exits with connection errors

**Solutions:**
- Implement process supervision (systemd, supervisord)
- Check PostgreSQL connection limits
- Monitor network stability
- Review PostgreSQL logs for disconnection reasons

### Debug Mode

For detailed debugging, use stdout alongside Feldera:

```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target "stdout,feldera" \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "users" \
  2>&1 | tee debug.log
```

This logs both:
- Events being sent (stdout)
- Error messages (stderr)

## Best Practices

### 1. Use REPLICA IDENTITY FULL

Always set `REPLICA IDENTITY FULL` for complete UPDATE events:

```sql
ALTER TABLE users REPLICA IDENTITY FULL;
```

Without this, UPDATE events only include primary key in old tuple.

### 2. Monitor Replication Lag

Check PostgreSQL replication status:

```sql
SELECT slot_name, 
       confirmed_flush_lsn,
       pg_current_wal_lsn(),
       (pg_current_wal_lsn() - confirmed_flush_lsn) AS lag_bytes
FROM pg_replication_slots
WHERE slot_name = 'my_slot';
```

### 3. Handle Backpressure

If Feldera can't keep up:
- Add more Feldera workers
- Optimize pipeline queries
- Consider batching updates
- Use NATS as buffer

### 4. Schema Evolution

When changing schemas:
1. Stop the replication tool
2. Update both PostgreSQL and Feldera schemas
3. Restart with new schema

Alternatively, use separate slots for backward-compatible changes.

### 5. High Availability

For production:
- Use process supervision (systemd)
- Monitor replication lag
- Set up alerting for errors
- Consider multiple Feldera instances
- Use connection pooling if needed

### 6. Testing

Test pipeline locally before production:
```bash
# Local Feldera
docker run -p 8080:8080 felderadb/feldera

# Run tool against test database
pgoutput-cmdline \
  --connection "host=localhost user=postgres dbname=test_db replication=database" \
  --slot test_slot \
  --publication test_pub \
  --format feldera \
  --target "stdout,feldera" \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "test_pipeline" \
  --feldera-table "test_table" \
  --create-slot
```

### 7. Performance Tuning

**PostgreSQL:**
- `max_wal_senders = 10` (adjust based on slots)
- `max_replication_slots = 10`
- `wal_sender_timeout = 60s`

**Network:**
- Use same region/datacenter for low latency
- Consider dedicated network for replication traffic

**Feldera:**
- Tune worker count based on load
- Monitor pipeline metrics
- Optimize SQL queries in pipeline

## Related Documentation

- [FELDERA_FORMAT.md](FELDERA_FORMAT.md) - Details on InsertDelete format
- [README.md](README.md) - General usage and features
- [GETTING_STARTED.md](GETTING_STARTED.md) - Quick start guide
- [Feldera Documentation](https://www.feldera.com/docs/) - Official Feldera docs

## Support

For issues or questions:
- GitHub Issues: [Report a bug](https://github.com/yourusername/pgoutput-cmdline/issues)
- Feldera Community: [Feldera Discord/Forum]
- PostgreSQL Replication: [PostgreSQL Documentation](https://www.postgresql.org/docs/current/logical-replication.html)
