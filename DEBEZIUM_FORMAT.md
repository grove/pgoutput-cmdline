# Debezium Format Guide

## Overview

The Debezium format outputs Change Data Capture (CDC) events in a structure compatible with Debezium, making it easy to integrate with Kafka Connect, data pipelines, and other CDC-based systems.

## Key Characteristics

- **Standard Envelope**: Uses the Debezium envelope format with `before`, `after`, `source`, and `op` fields
- **Data Events Only**: Only outputs INSERT, UPDATE, and DELETE operations
- **Filtered Events**: Transaction markers (BEGIN/COMMIT) and schema events (RELATION) are not included
- **Metadata Rich**: Includes source metadata like schema, table, timestamp, and LSN

## Event Structure

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `before` | object\|null | Row state before the change (null for INSERT) |
| `after` | object\|null | Row state after the change (null for DELETE) |
| `source` | object | Source metadata (database, schema, table, timestamp, etc.) |
| `op` | string | Operation code: `c` (create/INSERT), `u` (UPDATE), `d` (DELETE) |
| `ts_ms` | number | Event timestamp in milliseconds since epoch |
| `transaction` | object\|null | Transaction metadata (currently unused, always null) |

### Source Metadata

| Field | Type | Example | Description |
|-------|------|---------|-------------|
| `version` | string | `"pgoutput-stream-0.1.0"` | Tool version |
| `connector` | string | `"postgresql"` | Database type |
| `name` | string | `"pgoutput-stream"` | Connector name |
| `ts_ms` | number | `1706107200000` | Capture timestamp |
| `db` | string | `"postgres"` | Database name |
| `schema` | string | `"public"` | Schema name |
| `table` | string | `"users"` | Table name |
| `lsn` | string | `"16384"` | Log Sequence Number |

## Operation Codes

| Code | Operation | `before` | `after` | Description |
|------|-----------|----------|---------|-------------|
| `c` | CREATE (INSERT) | null | object | New row created |
| `u` | UPDATE | object\|null | object | Existing row modified |
| `d` | DELETE | object | null | Row removed |

**Note:** For UPDATE operations, `before` will only contain values if the table has `REPLICA IDENTITY FULL` set. Otherwise, it will be null or contain only the primary key.

## Usage Examples

### Basic Usage

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot debezium_slot \
  --publication my_publication \
  --format debezium
```

### With NATS JetStream

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot debezium_slot \
  --publication my_publication \
  --format debezium \
  --nats-server "nats://localhost:4222"
```

Note: When using NATS, events are still formatted as regular JSON on the wire. The Debezium format only affects stdout output.

### Filtering to Specific Tables

Use PostgreSQL publications to control which tables are captured:

```sql
-- Create publication for specific tables
CREATE PUBLICATION debezium_pub FOR TABLE users, orders, products;

-- Run tool with this publication
pgoutput-stream --publication debezium_pub --format debezium ...
```

## Sample Output

### INSERT Event

```json
{
  "before": null,
  "after": {
    "id": "123",
    "name": "Alice",
    "email": "alice@example.com",
    "created_at": "2024-01-25T10:30:00Z"
  },
  "source": {
    "version": "pgoutput-stream-0.1.0",
    "connector": "postgresql",
    "name": "pgoutput-stream",
    "ts_ms": 1706180000000,
    "db": "postgres",
    "schema": "public",
    "table": "users",
    "lsn": "16384"
  },
  "op": "c",
  "ts_ms": 1706180000000
}
```

### UPDATE Event (with REPLICA IDENTITY FULL)

```json
{
  "before": {
    "id": "123",
    "name": "Alice",
    "email": "alice@example.com",
    "created_at": "2024-01-25T10:30:00Z"
  },
  "after": {
    "id": "123",
    "name": "Alice",
    "email": "alice.newemail@example.com",
    "created_at": "2024-01-25T10:30:00Z"
  },
  "source": {
    "version": "pgoutput-stream-0.1.0",
    "connector": "postgresql",
    "name": "pgoutput-stream",
    "ts_ms": 1706180100000,
    "db": "postgres",
    "schema": "public",
    "table": "users",
    "lsn": "16384"
  },
  "op": "u",
  "ts_ms": 1706180100000
}
```

### DELETE Event

```json
{
  "before": {
    "id": "123",
    "name": "Alice",
    "email": "alice.newemail@example.com",
    "created_at": "2024-01-25T10:30:00Z"
  },
  "after": null,
  "source": {
    "version": "pgoutput-stream-0.1.0",
    "connector": "postgresql",
    "name": "pgoutput-stream",
    "ts_ms": 1706180200000,
    "db": "postgres",
    "schema": "public",
    "table": "users",
    "lsn": "16384"
  },
  "op": "d",
  "ts_ms": 1706180200000
}
```

## Best Practices

### 1. Set REPLICA IDENTITY FULL

To capture complete `before` values in UPDATE and DELETE operations:

```sql
ALTER TABLE users REPLICA IDENTITY FULL;
ALTER TABLE orders REPLICA IDENTITY FULL;
```

Without this, only the primary key will be available in the `before` field.

### 2. Use Publications for Filtering

Instead of filtering events in your consumer, use PostgreSQL publications:

```sql
-- Only capture user-related tables
CREATE PUBLICATION user_changes FOR TABLE users, user_profiles, user_preferences;
```

### 3. Handle NULL Values

The Debezium format preserves NULL values in the data. Your consumer should handle them appropriately:

```javascript
// JavaScript example
if (event.after.email === null) {
  console.log("Email is NULL");
}
```

### 4. Parse Timestamps

All timestamps are in milliseconds since Unix epoch:

```python
# Python example
import datetime
ts = event['ts_ms']
dt = datetime.datetime.fromtimestamp(ts / 1000.0)
```

### 5. Consider Event Ordering

Events are emitted in the order they are received from PostgreSQL's logical replication stream, which respects transaction boundaries.

## Integration Patterns

### With Kafka Connect

While this tool doesn't directly integrate with Kafka, you can pipe the output:

```bash
pgoutput-stream --format debezium ... | kafka-console-producer --topic cdc-events ...
```

### With Data Pipelines

The JSON output can be consumed by any tool that processes JSON streams:

```bash
# Stream to file
pgoutput-stream --format debezium > cdc-events.jsonl

# Process with jq
pgoutput-stream --format debezium | jq 'select(.op == "c")'

# Stream to analytics
pgoutput-stream --format debezium | your-analytics-consumer
```

### With NATS for Distributed Processing

Combine Debezium format with NATS to distribute CDC events:

```bash
pgoutput-stream \
  --format debezium \
  --nats-server "nats://localhost:4222"
  
# Multiple consumers can process events
nats sub "postgres.public.users.insert"
nats sub "postgres.public.orders.*"
```

## Differences from Standard Debezium

This implementation differs from standard Debezium in these ways:

1. **Simplified Source**: Uses a simplified source structure optimized for pgoutput
2. **No Transaction Events**: BEGIN/COMMIT are filtered out in Debezium mode
3. **No Schema Events**: RELATION events are not included
4. **Fixed DB Name**: Source always shows `"postgres"` (can be enhanced in future)
5. **No Snapshot Support**: Only captures streaming changes, not initial snapshots

## Testing

Test files are available in the `examples/` directory:

- `test_debezium.sql` - SQL script to generate various Debezium events

Run the test:

```bash
# Terminal 1: Start the tool
pgoutput-stream --connection "..." --slot test_slot --publication my_pub --format debezium

# Terminal 2: Execute test operations
psql -U postgres -d mydb -f examples/test_debezium.sql
```

## Troubleshooting

### No Events Appearing

1. Verify the publication includes your tables:
   ```sql
   SELECT * FROM pg_publication_tables WHERE pubname = 'my_publication';
   ```

2. Check that operations are actually happening:
   ```sql
   SELECT * FROM pg_stat_replication;
   ```

3. Ensure REPLICA IDENTITY is set (for UPDATE/DELETE):
   ```sql
   SELECT relname, relreplident FROM pg_class WHERE relname = 'users';
   ```

### Missing `before` Values

If UPDATE events show `null` or incomplete `before` data:

```sql
-- Set REPLICA IDENTITY FULL
ALTER TABLE your_table REPLICA IDENTITY FULL;
```

### JSON Parsing Errors

The output is newline-delimited JSON (one event per line). Use appropriate parsers:

```bash
# Good: Process line by line
while read line; do echo "$line" | jq '.op'; done

# Bad: Try to parse entire stream as single JSON
jq '.' < stream  # This will fail
```

## Further Reading

- [Debezium Documentation](https://debezium.io/documentation/)
- [PostgreSQL Logical Replication](https://www.postgresql.org/docs/current/logical-replication.html)
- [pgoutput Plugin](https://www.postgresql.org/docs/current/protocol-logical-replication.html)
