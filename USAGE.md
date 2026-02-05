# Usage Guide

Complete reference for pgoutput-stream command-line options and usage patterns.

## Table of Contents

- [Command-Line Options](#command-line-options)
- [Basic Usage](#basic-usage)
- [Output Formats](#output-formats)
- [Output Targets](#output-targets)
- [Advanced Features](#advanced-features)
- [Example Workflow](#example-workflow)
- [Piping to Other Tools](#piping-to-other-tools)

## Command-Line Options

```
pgoutput-stream [OPTIONS]

Required Options:
  -c, --connection <CONNECTION>
          PostgreSQL connection string
          Format: "host=<host> port=<port> user=<user> password=<pass> dbname=<db>"
          
  -s, --slot <SLOT>
          Replication slot name
          Used to track position in the replication stream
          
  -p, --publication <PUBLICATION>
          Publication name
          Defines which tables to replicate

Format Options:
  -f, --format <FORMAT>
          Output format [default: json]
          Values: json, json-pretty, text, debezium, feldera

Replication Options:
      --create-slot
          Create replication slot if it doesn't exist
          
      --start-lsn <START_LSN>
          Starting LSN (Log Sequence Number) to stream from
          Format: "0/12345678" (PostgreSQL LSN format)

Output Target Options:
  -t, --target <TARGET>
          Output target(s) [default: stdout]
          Values: stdout, nats, feldera
          Supports multiple targets (comma-separated): "stdout,nats"

NATS Options (required when target includes 'nats'):
      --nats-server <URL>
          NATS server URL
          Example: "nats://localhost:4222"
          
      --nats-stream <STREAM>
          NATS JetStream stream name [default: postgres_replication]
          
      --nats-subject-prefix <PREFIX>
          NATS subject prefix [default: postgres]
          Subjects format: {prefix}.{schema}.{table}.{operation}

Feldera Options (required when target includes 'feldera'):
      --feldera-url <URL>
          Feldera base URL
          Example: "http://localhost:8080"
          
      --feldera-pipeline <NAME>
          Feldera pipeline name
          
      --feldera-tables <NAMES>
          Comma-separated schema-qualified tables
          Format: "public_users,public_orders"
          Optional: If omitted, routes all tables
          
      --feldera-api-key <KEY>
          Feldera API key for authentication (optional)

Other Options:
  -h, --help
          Print help information
          
  -V, --version
          Print version information
```

## Basic Usage

### Minimal Example

Stream changes to stdout with default JSON format:

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres password=secret dbname=mydb" \
  --slot my_replication_slot \
  --publication my_publication
```

### Create Replication Slot Automatically

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --create-slot
```

**Note:** The slot will persist even after the tool stops. To remove it:
```sql
SELECT pg_drop_replication_slot('my_slot');
```

### Resume from Specific LSN

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --start-lsn "0/16B2D50"
```

## Output Formats

### JSON (default)

Compact JSON format, one event per line (JSONL):

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format json
```

**Output:**
```json
{"Begin":{"lsn":"0/123456","timestamp":123456789,"xid":1234}}
{"Relation":{"relation_id":16384,"schema":"public","table":"users","columns":[{"name":"id","type_id":23,"flags":1},{"name":"name","type_id":1043,"flags":0}]}}
{"Insert":{"relation_id":16384,"schema":"public","table":"users","new_tuple":{"id":"1","name":"Alice"}}}
{"Commit":{"lsn":"0/123457","timestamp":123456790}}
```

**Use Cases:**
- Default output for streaming to files or pipelines
- Easy to parse with jq, Python, or other JSON tools
- Minimal bandwidth for network transmission

### JSON Pretty

Human-readable indented JSON:

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format json-pretty
```

**Output:**
```json
{
  "Begin": {
    "lsn": "0/123456",
    "timestamp": 123456789,
    "xid": 1234
  }
}
{
  "Insert": {
    "relation_id": 16384,
    "schema": "public",
    "table": "users",
    "new_tuple": {
      "id": "1",
      "name": "Alice"
    }
  }
}
```

**Use Cases:**
- Debugging and development
- Manual inspection of events
- Documentation and examples

### Text

Human-readable plain text format:

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format text
```

**Output:**
```
BEGIN [LSN: 0/123456, XID: 1234, Time: 123456789]
RELATION [public.users (ID: 16384)]
  Columns:
    - id (type_id: 23, flags: 1)
    - name (type_id: 1043, flags: 0)
INSERT into public.users (ID: 16384)
  New values:
    id: 1
    name: Alice
COMMIT [LSN: 0/123457, Time: 123456790]
UPDATE in public.users (ID: 16384)
  Old values:
    id: 1
    name: Alice
  New values:
    id: 1
    name: Alice Smith
DELETE from public.users (ID: 16384)
  Old values:
    id: 1
    name: Alice Smith
```

**Use Cases:**
- Terminal monitoring during development
- Quick visibility into replication stream
- Debugging table changes

### Debezium

Standard Debezium CDC envelope format:

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format debezium
```

**Key Features:**
- Compatible with Debezium ecosystems
- Includes `before`, `after`, `source`, `op` fields
- Filters out transaction markers (BEGIN/COMMIT)
- Only outputs INSERT, UPDATE, DELETE operations

**See:** [DEBEZIUM_FORMAT.md](DEBEZIUM_FORMAT.md) for complete format specification and examples.

### Feldera

InsertDelete format optimized for streaming SQL:

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera
```

**Key Features:**
- Explicit `{"insert": {...}}` and `{"delete": {...}}` wrappers
- UPDATE operations decomposed into delete + insert pairs
- Optimized for incremental view maintenance
- Filters out transaction markers

**See:** [FELDERA_FORMAT.md](FELDERA_FORMAT.md) for complete format specification and rationale.

## Output Targets

### Stdout (Default)

Stream to standard output:

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --target stdout
```

### NATS JetStream

Stream to NATS for distributed messaging:

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --target nats \
  --nats-server "nats://localhost:4222" \
  --nats-stream "postgres_replication" \
  --nats-subject-prefix "postgres"
```

**Subject Pattern:** `{prefix}.{schema}.{table}.{operation}`

**Examples:**
- `postgres.public.users.insert`
- `postgres.public.orders.update`
- `postgres.analytics.events.delete`

**See:** [NATS_INTEGRATION.md](NATS_INTEGRATION.md) for complete integration guide.

### Feldera HTTP

Stream to Feldera pipelines via HTTP ingress:

```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "postgres_cdc"
```

**Optional table filtering:**
```bash
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "postgres_cdc" \
  --feldera-tables "public_users,public_orders"
```

**See:** [FELDERA_HTTP_CONNECTOR.md](FELDERA_HTTP_CONNECTOR.md) for complete integration guide.

### Multiple Targets

Combine multiple output targets simultaneously:

```bash
# stdout + NATS
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --target "stdout,nats" \
  --nats-server "nats://localhost:4222"

# stdout + NATS + Feldera
pgoutput-stream \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target "stdout,nats,feldera" \
  --nats-server "nats://localhost:4222" \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "my_pipeline"
```

**Use Cases:**
- Monitor stdout while also streaming to production system
- Fan out changes to multiple downstream consumers
- Debug production streams without disrupting the pipeline

## Advanced Features

### Using Environment Variables

Store sensitive credentials in environment variables:

```bash
export PG_PASSWORD="secret"
export NATS_SERVER="nats://localhost:4222"
export FELDERA_API_KEY="your-api-key"

pgoutput-stream \
  --connection "host=localhost user=postgres password=$PG_PASSWORD dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --target nats \
  --nats-server "$NATS_SERVER"
```

### Connection String Formats

**Key-value pairs:**
```bash
--connection "host=localhost port=5432 user=postgres password=secret dbname=mydb"
```

**PostgreSQL URI:**
```bash
--connection "postgresql://postgres:secret@localhost:5432/mydb"
```

**With SSL:**
```bash
--connection "host=localhost user=postgres password=secret dbname=mydb sslmode=require"
```

### Graceful Shutdown

The tool handles SIGINT (Ctrl+C) and SIGTERM gracefully:

```bash
# Start streaming
pgoutput-stream --connection "..." --slot my_slot --publication my_pub

# Press Ctrl+C to gracefully shutdown
# The replication position is saved automatically
```

On restart, the tool resumes from the last committed position.

## Example Workflow

Complete step-by-step example from setup to streaming:

### 1. Set Up PostgreSQL

```bash
# Connect to PostgreSQL
psql -U postgres

# Create test database
CREATE DATABASE testdb;
\c testdb

# Create test table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100),
    email VARCHAR(100),
    created_at TIMESTAMP DEFAULT NOW()
);

# Set REPLICA IDENTITY FULL
ALTER TABLE users REPLICA IDENTITY FULL;

# Create publication
CREATE PUBLICATION user_changes FOR TABLE users;

# Grant permissions (if needed)
ALTER USER postgres WITH REPLICATION;
```

### 2. Start Streaming

```bash
# In terminal 1: Start pgoutput-stream
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=testdb" \
  --slot test_slot \
  --publication user_changes \
  --create-slot \
  --format text
```

### 3. Generate Changes

```bash
# In terminal 2: Make database changes
psql -U postgres -d testdb

-- Insert data
INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');
INSERT INTO users (name, email) VALUES ('Bob', 'bob@example.com');

-- Update data
UPDATE users SET email = 'alice.new@example.com' WHERE name = 'Alice';

-- Delete data
DELETE FROM users WHERE name = 'Bob';
```

### 4. Observe Output

Terminal 1 will show:

```
Connecting to PostgreSQL...
Created replication slot: test_slot
Starting replication stream...

BEGIN [LSN: 0/16B2D50, XID: 730, Time: 730826470123456]
INSERT into public.users (ID: 16384)
  New values:
    id: 1
    name: Alice
    email: alice@example.com
    created_at: 2026-02-05 10:30:00
COMMIT [LSN: 0/16B2E20, Time: 730826470123457]

BEGIN [LSN: 0/16B2E30, XID: 731, Time: 730826470123460]
INSERT into public.users (ID: 16384)
  New values:
    id: 2
    name: Bob
    email: bob@example.com
    created_at: 2026-02-05 10:30:05
COMMIT [LSN: 0/16B2F00, Time: 730826470123461]

BEGIN [LSN: 0/16B2F10, XID: 732, Time: 730826470123500]
UPDATE in public.users (ID: 16384)
  Old values:
    id: 1
    name: Alice
    email: alice@example.com
    created_at: 2026-02-05 10:30:00
  New values:
    id: 1
    name: Alice
    email: alice.new@example.com
    created_at: 2026-02-05 10:30:00
COMMIT [LSN: 0/16B2FE0, Time: 730826470123501]

BEGIN [LSN: 0/16B2FF0, XID: 733, Time: 730826470123550]
DELETE from public.users (ID: 16384)
  Old values:
    id: 2
    name: Bob
    email: bob@example.com
    created_at: 2026-02-05 10:30:05
COMMIT [LSN: 0/16B30C0, Time: 730826470123551]
```

### 5. Cleanup

```sql
-- Drop replication slot (after stopping the tool)
SELECT pg_drop_replication_slot('test_slot');

-- Drop publication
DROP PUBLICATION user_changes;

-- Drop database (optional)
DROP DATABASE testdb;
```

## Piping to Other Tools

The JSON output format makes it easy to integrate with other tools.

### Filter Specific Operations with jq

```bash
# Only show INSERT operations
pgoutput-stream ... --format json | jq 'select(.Insert != null)'

# Only show changes to specific table
pgoutput-stream ... --format json | jq 'select(.Insert.table == "users" or .Update.table == "users" or .Delete.table == "users")'

# Extract just the data payloads
pgoutput-stream ... --format json | jq 'select(.Insert != null) | .Insert.new_tuple'

# Count operations by type
pgoutput-stream ... --format json | jq -s 'group_by(keys[0]) | map({type: .[0] | keys[0], count: length})'
```

### Save to File

```bash
# Save all changes to JSONL file
pgoutput-stream ... --format json > changes.jsonl

# Append to existing file
pgoutput-stream ... --format json >> changes.jsonl

# Rotate logs by date
pgoutput-stream ... --format json > "changes_$(date +%Y%m%d).jsonl"
```

### Process with Python

```bash
pgoutput-stream ... --format json | python process_changes.py
```

**process_changes.py:**
```python
#!/usr/bin/env python3
import sys
import json

for line in sys.stdin:
    event = json.loads(line)
    
    if 'Insert' in event:
        print(f"New record in {event['Insert']['table']}: {event['Insert']['new_tuple']}")
    elif 'Update' in event:
        print(f"Updated {event['Update']['table']}: {event['Update']['new_tuple']}")
    elif 'Delete' in event:
        print(f"Deleted from {event['Delete']['table']}: {event['Delete']['old_tuple']}")
```

### Monitoring with grep

```bash
# Watch for specific tables
pgoutput-stream ... --format json | grep '"table":"users"'

# Monitor errors
pgoutput-stream ... 2>&1 | grep -i error

# Count events per second
pgoutput-stream ... --format json | pv -l -i 1 > /dev/null
```

### Integration with other streaming tools

```bash
# Send to Kafka
pgoutput-stream ... --format json | kafkacat -P -b localhost:9092 -t postgres-changes

# Send to Redis Streams
pgoutput-stream ... --format json | while read line; do
  redis-cli XADD postgres-stream '*' data "$line"
done

# Send to webhook
pgoutput-stream ... --format json | while read line; do
  curl -X POST http://localhost:8080/webhook \
    -H "Content-Type: application/json" \
    -d "$line"
done
```

## See Also

- [PREREQUISITES.md](PREREQUISITES.md) - PostgreSQL configuration
- [GETTING_STARTED.md](GETTING_STARTED.md) - Quick start guide
- [DEBEZIUM_FORMAT.md](DEBEZIUM_FORMAT.md) - Debezium format details
- [FELDERA_FORMAT.md](FELDERA_FORMAT.md) - Feldera format details
- [NATS_INTEGRATION.md](NATS_INTEGRATION.md) - NATS integration guide
- [FELDERA_HTTP_CONNECTOR.md](FELDERA_HTTP_CONNECTOR.md) - Feldera HTTP guide
