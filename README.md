# pgoutput-cmdline

A high-performance Rust command-line tool that streams PostgreSQL logical replication changes to multiple destinations including stdout, NATS JetStream, and Feldera pipelines.

## Features

- 🚀 Stream PostgreSQL logical replication changes in real-time
- 📊 Multiple output formats: JSON, pretty JSON, human-readable text, Debezium CDC, and Feldera InsertDelete
- 📡 Multiple output targets: stdout, NATS JetStream, and Feldera HTTP ingress (can combine multiple targets)
- 🌐 Feldera HTTP input connector
- 🔄 Automatic replication slot creation
- 🎯 Support for all DML operations: INSERT, UPDATE, DELETE
- ⚡ Built with async Rust (Tokio) for high performance
- 🛑 Graceful shutdown on SIGINT/SIGTERM
- 🧪 Comprehensive test coverage (80 unit tests)

## Quick Start

New to this tool? Check out [GETTING_STARTED.md](GETTING_STARTED.md) for a quick guide to get up and running in minutes.

## Prerequisites

### PostgreSQL Configuration

1. **Enable logical replication** in `postgresql.conf`:
   ```conf
   wal_level = logical
   max_replication_slots = 10
   max_wal_senders = 10
   ```

2. **Configure authentication** in `pg_hba.conf`:
   ```conf
   # Allow replication connections from localhost
   host    replication    all    127.0.0.1/32    md5
   ```

3. **Restart PostgreSQL** to apply changes:
   ```bash
   sudo systemctl restart postgresql
   ```

4. **Create a publication** for the tables you want to replicate:
   ```sql
   -- Connect to your database
   psql -U postgres -d mydb

   -- Create a publication for specific tables
   CREATE PUBLICATION my_publication FOR TABLE users, orders;

   -- Or create a publication for all tables
   CREATE PUBLICATION my_publication FOR ALL TABLES;
   ```

5. **Set REPLICA IDENTITY FULL** (recommended for UPDATE operations):
   ```sql
   -- To capture old values in UPDATE statements
   ALTER TABLE users REPLICA IDENTITY FULL;
   ALTER TABLE orders REPLICA IDENTITY FULL;
   ```
   
   Note: Without `REPLICA IDENTITY FULL`, UPDATE events will only include the primary key in the old tuple. With FULL mode, all column values are included, allowing you to see what changed.

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/pgoutput-cmdline.git
cd pgoutput-cmdline

# Build the project
cargo build --release

# The binary will be available at:
# ./target/release/pgoutput-cmdline
```

## Usage

### Basic Usage

```bash
pgoutput-cmdline \
  --connection "host=localhost user=postgres password=secret dbname=mydb" \
  --slot my_replication_slot \
  --publication my_publication
```

### Create Replication Slot Automatically

```bash
pgoutput-cmdline \
  --connection "host=localhost user=postgres password=secret dbname=mydb" \
  --slot my_replication_slot \
  --publication my_publication \
  --create-slot
```

### Output Formats

#### JSON (default)
```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format json
```

Output:
```json
{"Begin":{"lsn":"0/123456","timestamp":123456789,"xid":1234}}
{"Insert":{"relation_id":16384,"schema":"public","table":"users","new_tuple":{"id":"1","name":"Alice"}}}
{"Commit":{"lsn":"0/123457","timestamp":123456790}}
```

#### Pretty JSON
```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format json-pretty
```

#### Human-Readable Text
```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format text
```

Output:
```
BEGIN [LSN: 0/123456, XID: 1234, Time: 123456789]
INSERT into public.users (ID: 16384)
  New values:
    id: 1
    name: Alice
COMMIT [LSN: 0/123457, Time: 123456790]
```

#### Debezium CDC Format
```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format debezium
```

The Debezium format outputs Change Data Capture (CDC) events compatible with Debezium-based ecosystems. This format is ideal for integration with Kafka Connect, data pipelines, and standard CDC tooling.

**Key Features:**
- Standard Debezium envelope structure with `before`, `after`, `source`, and `op` fields
- Only outputs data change events (INSERT, UPDATE, DELETE)
- Transaction markers (BEGIN/COMMIT) and RELATION events are filtered out
- Compatible with Debezium consumers and downstream processors

**Output Example - INSERT:**
```json
{
  "before": null,
  "after": {
    "id": "1",
    "name": "Alice",
    "email": "alice@example.com"
  },
  "source": {
    "version": "pgoutput-cmdline-0.1.0",
    "connector": "postgresql",
    "name": "pgoutput-cmdline",
    "ts_ms": 1706107200000,
    "db": "postgres",
    "schema": "public",
    "table": "users",
    "lsn": "16384"
  },
  "op": "c",
  "ts_ms": 1706107200000
}
```

**Output Example - UPDATE:**
```json
{
  "before": {
    "id": "1",
    "name": "Alice",
    "email": "alice@example.com"
  },
  "after": {
    "id": "1",
    "name": "Alice",
    "email": "alice.updated@example.com"
  },
  "source": {
    "version": "pgoutput-cmdline-0.1.0",
    "connector": "postgresql",
    "name": "pgoutput-cmdline",
    "ts_ms": 1706107210000,
    "db": "postgres",
    "schema": "public",
    "table": "users",
    "lsn": "16384"
  },
  "op": "u",
  "ts_ms": 1706107210000
}
```

**Output Example - DELETE:**
```json
{
  "before": {
    "id": "1",
    "name": "Alice",
    "email": "alice@example.com"
  },
  "after": null,
  "source": {
    "version": "pgoutput-cmdline-0.1.0",
    "connector": "postgresql",
    "name": "pgoutput-cmdline",
    "ts_ms": 1706107220000,
    "db": "postgres",
    "schema": "public",
    "table": "users",
    "lsn": "16384"
  },
  "op": "d",
  "ts_ms": 1706107220000
}
```

**Operation Codes:**
- `c` = CREATE (INSERT)
- `u` = UPDATE
- `d` = DELETE

**Note:** To capture full `before` values in UPDATE operations, set `REPLICA IDENTITY FULL` on your tables (see Prerequisites section).

#### Feldera JSON Format
```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera
```

The Feldera format outputs Change Data Capture events using the InsertDelete structure, ideal for streaming pipelines and incremental view maintenance.

**Key Features:**
- Explicit operation wrapping: `{"insert": {...}}`, `{"delete": {...}}`
- Updates encoded as delete (old) + insert (new) pairs
- Supports all DML operations
- Transaction markers (BEGIN/COMMIT) and RELATION events are filtered out
- Compatible with Feldera streaming pipelines and similar systems

**Output Example - INSERT:**
```json
{"insert": {"id": "1", "name": "Alice", "email": "alice@example.com"}}
```

**Output Example - UPDATE (produces 2 events):**
```json
{"delete": {"id": "1", "name": "Alice", "email": "alice@example.com"}}
{"insert": {"id": "1", "name": "Alice", "email": "alice.updated@example.com"}}
```

**Output Example - DELETE:**
```json
{"delete": {"id": "1", "name": "Alice", "email": "alice@example.com"}}
```

**Why Updates = Delete + Insert:**

This pattern follows incremental view maintenance principles used in streaming systems. When a row is updated, it's treated as:
1. Remove the old state from the view (delete)
2. Add the new state to the view (insert)

This approach ensures correct computation in aggregations, joins, and other stream operations.

**Use Cases:**
- Feldera streaming SQL pipelines
- Incremental view maintenance systems
- Stream processing with aggregations
- Change data capture for analytics
- Real-time data synchronization

### Command-Line Options

```
Options:
  -c, --connection <CONNECTION>  PostgreSQL connection string
  -s, --slot <SLOT>             Replication slot name
  -p, --publication <PUBLICATION> Publication name
  -f, --format <FORMAT>         Output format: json, json-pretty, text, debezium, or feldera [default: json]
      --create-slot             Create replication slot if it doesn't exist
      --start-lsn <START_LSN>   Starting LSN (Log Sequence Number) to stream from
  -t, --target <TARGET>         Output target(s): stdout, nats, feldera (comma-separated for multiple) [default: stdout]
      --nats-server <URL>       NATS server URL (required when target includes 'nats')
      --nats-stream <STREAM>    NATS JetStream stream name [default: postgres_replication]
      --nats-subject-prefix <PREFIX> NATS subject prefix [default: postgres]
      --feldera-url <URL>       Feldera base URL (required when target includes 'feldera')
      --feldera-pipeline <NAME> Feldera pipeline name (required when target includes 'feldera')
      --feldera-table <NAME>    Feldera table name (required when target includes 'feldera')
      --feldera-api-key <KEY>   Feldera API key for authentication (optional)
  -h, --help                    Print help
```

## Output Targets

Stream PostgreSQL changes to one or more destinations using the `--target` option.

### Stdout (Default)

Output changes to standard output:

```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --target stdout
```

### Multiple Targets

Combine multiple targets with comma-separated values:

```bash
# Output to both stdout and NATS
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --target "stdout,nats" \
  --nats-server "nats://localhost:4222"

# Output to stdout, NATS, and Feldera simultaneously
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target "stdout,nats,feldera" \
  --nats-server "nats://localhost:4222" \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "postgres_cdc" \
  --feldera-table "users"
```

## Feldera HTTP Connector

Stream PostgreSQL changes directly to Feldera pipelines via HTTP ingress API. Perfect for real-time data pipelines, streaming SQL, and incremental view maintenance.

### Basic Usage

```bash
pgoutput-cmdline \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "postgres_cdc" \
  --feldera-table "users"
```

### With API Authentication

```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "https://cloud.feldera.com" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "my_table" \
  --feldera-api-key "your-api-key-here"
```

### How It Works

1. PostgreSQL replication events are converted to Feldera InsertDelete format with proper JSON types
2. **Type Conversion**: Integers, floats, and booleans are sent as JSON numbers/booleans (not strings)
3. Events are sent via HTTP POST to: `/v0/pipelines/{pipeline}/ingress/{table}?format=json&update_format=insert_delete&array=true`
4. **INSERT** operations: `[{"insert": {...}}]`
5. **DELETE** operations: `[{"delete": {...}}]`
6. **UPDATE** operations: `[{"delete": {...}}, {"insert": {...}}]`
7. All events are wrapped in JSON arrays (required by `array=true` parameter)
8. Transaction boundaries (BEGIN/COMMIT) and schema events (RELATION) are filtered out

### Feldera Pipeline Example

Create a simple pipeline in Feldera:

```sql
-- Define input table matching PostgreSQL schema
CREATE TABLE users (
    id INT,
    name VARCHAR,
    email VARCHAR
) WITH (
    'connectors' = '[{
        "name": "postgres_input",
        "transport": {
            "name": "http_input",
            "config": {}
        }
    }]'
);

-- Define a view
CREATE VIEW active_users AS
SELECT id, name, email
FROM users
WHERE email IS NOT NULL;
```

Then stream from PostgreSQL:

```bash
pgoutput-cmdline \
  --connection "..." \
  --slot my_slot \
  --publication my_pub \
  --format feldera \
  --target feldera \
  --feldera-url "http://localhost:8080" \
  --feldera-pipeline "my_pipeline" \
  --feldera-table "users"
```

### Use Cases

1. **Real-Time Analytics**: Feed PostgreSQL changes directly into streaming SQL pipelines
2. **Incremental View Maintenance**: Keep materialized views in sync with source data
3. **Data Pipelines**: Build ETL pipelines with streaming transformations
4. **Change Data Capture**: Stream database changes to data warehouses
5. **Event Processing**: React to database changes in real-time

### Troubleshooting

**Connection Errors:**
- Verify Feldera server is running and accessible
- Check firewall rules and network connectivity
- Ensure correct base URL (with http:// or https://)

**Authentication Errors (401/403):**
- Verify API key is correct if using authentication
- Check that the API key has necessary permissions

**Pipeline/Table Not Found (404):**
- Ensure pipeline exists and is running in Feldera
- Verify table name matches the table defined in Feldera SQL
- Check for typos in pipeline or table names

**Format Errors (400):**
- Ensure `--format feldera` is specified
- Verify PostgreSQL table schema matches Feldera table schema
- Check that data types are compatible
```

## NATS JetStream Integration

Stream PostgreSQL replication events to NATS JetStream for distributed processing, event-driven architectures, and microservices.

### Setup NATS Server

```bash
# Run NATS with JetStream enabled (Docker)
docker run -p 4222:4222 -p 8222:8222 nats:latest -js

# Or install NATS locally
# https://docs.nats.io/running-a-nats-service/introduction/installation
```

### Stream to NATS

```bash
# Stream to NATS only
pgoutput-cmdline \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --target nats \
  --nats-server "nats://localhost:4222" \
  --nats-stream "postgres_replication" \
  --nats-subject-prefix "postgres"

# Or combine with stdout
pgoutput-cmdline \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --target "stdout,nats" \
  --nats-server "nats://localhost:4222"
```

### NATS Subject Naming

Events are published to subjects following this pattern:

- **Table operations**: `{prefix}.{schema}.{table}.{operation}`
  - Example: `postgres.public.users.insert`
  - Example: `postgres.public.orders.update`
  - Example: `postgres.public.products.delete`
  
- **Transaction boundaries**: `{prefix}.transactions.{event}.event`
  - Example: `postgres.transactions.begin.event`
  - Example: `postgres.transactions.commit.event`

- **Schema metadata**: `{prefix}.{schema}.{table}.relation`
  - Example: `postgres.public.users.relation`

### NATS Consumer Example

Subscribe to specific table changes:

```bash
# Using NATS CLI - Subscribe to all user INSERT operations
nats sub "postgres.public.users.insert"

# Subscribe to all operations on users table
nats sub "postgres.public.users.*"

# Subscribe to all INSERT operations across all tables
nats sub "postgres.*.*.insert"

# Subscribe to everything
nats sub "postgres.>"
```

### JetStream Stream Configuration

The tool automatically creates a JetStream stream with these defaults:
- **Name**: Configurable via `--nats-stream`
- **Subjects**: `{prefix}.*.*.*` (captures all events)
- **Storage**: Memory (default, can be modified)
- **Max Messages**: 1,000,000
- **Max Bytes**: 1GB
- **Retention**: Limits-based (can be modified for work-queue patterns)

### Use Cases

1. **Event-Driven Microservices**: Multiple services subscribe to relevant table changes
2. **Real-Time Analytics**: Stream database changes to analytics engines
3. **Data Synchronization**: Keep multiple systems in sync with PostgreSQL
4. **Audit Logging**: Durable event log with replay capability
5. **Change Data Capture (CDC)**: Feed data warehouses and data lakes

## Example Workflow

A complete working example is available in the [examples/](examples/) directory:
- `setup.sql` - Database setup script
- `test_changes.sql` - Sample DML operations
- `run.sh` - Shell script to run the tool

### 1. Set Up PostgreSQL

```sql
-- Connect to PostgreSQL
psql -U postgres

-- Create a test database
CREATE DATABASE testdb;
\c testdb

-- Create a test table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100),
    email VARCHAR(100)
);

-- Set REPLICA IDENTITY FULL for UPDATE old values
ALTER TABLE users REPLICA IDENTITY FULL;

-- Create a publication
CREATE PUBLICATION user_changes FOR TABLE users;
```

### 2. Run the Tool

```bash
cargo run -- \
  --connection "host=localhost user=postgres dbname=testdb" \
  --slot test_slot \
  --publication user_changes \
  --create-slot \
  --format text
```

### 3. Make Changes in Another Terminal

```sql
-- Connect to PostgreSQL
psql -U postgres -d testdb

-- Insert data
INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');
INSERT INTO users (name, email) VALUES ('Bob', 'bob@example.com');

-- Update data
UPDATE users SET email = 'alice.new@example.com' WHERE name = 'Alice';

-- Delete data
DELETE FROM users WHERE name = 'Bob';
```

### 4. See the Output

The tool will stream the changes in real-time:

```
Connecting to PostgreSQL...
Slot: test_slot
Publication: user_changes
Output format: text
Starting replication stream...

BEGIN [LSN: 0/16B2D50, XID: 730, Time: 730826470123456]
RELATION [public.users (ID: 16384)]
  Columns:
    - id (type_id: 23, flags: 1)
    - name (type_id: 1043, flags: 0)
    - email (type_id: 1043, flags: 0)
INSERT into public.users (ID: 16384)
  New values:
    id: 1
    name: Alice
    email: alice@example.com
COMMIT [LSN: 0/16B2E20, Time: 730826470123457]
...
```

## Piping to Other Tools

The JSON output format makes it easy to pipe changes to other tools:

```bash
# Stream to a file
pgoutput-cmdline ... --format json > changes.jsonl

# Filter specific operations with jq
pgoutput-cmdline ... --format json | jq 'select(.Insert != null)'

# Process with custom scripts
pgoutput-cmdline ... --format json | python process_changes.py
```

## Documentation

For detailed guides on specific features:

- **[GETTING_STARTED.md](GETTING_STARTED.md)** - Quick start guide for new users
- **[FELDERA_HTTP_CONNECTOR.md](FELDERA_HTTP_CONNECTOR.md)** - Complete guide to Feldera HTTP streaming
- **[FELDERA_FORMAT.md](FELDERA_FORMAT.md)** - Feldera InsertDelete format specification
- **[DEBEZIUM_FORMAT.md](DEBEZIUM_FORMAT.md)** - Debezium CDC format specification  
- **[TEST_COVERAGE.md](TEST_COVERAGE.md)** - Test coverage details

## Troubleshooting

### Connection Errors

- Ensure PostgreSQL is running and accessible
- Verify connection string parameters
- Check that the user has replication privileges:
  ```sql
  ALTER USER postgres WITH REPLICATION;
  ```

### Slot Already Exists

If you get an error about the slot already existing, either:
- Use a different slot name
- Drop the existing slot: `SELECT pg_drop_replication_slot('my_slot');`
- Don't use the `--create-slot` flag

### No Changes Appearing

- Verify the publication includes your tables: `\dRp+` in psql
- Ensure tables have a `REPLICA IDENTITY` (default is PRIMARY KEY)
- Check that `wal_level = logical` in PostgreSQL configuration

### UPDATE Operations Missing Old Values

- Set `REPLICA IDENTITY FULL` on your tables:
  ```sql
  ALTER TABLE your_table REPLICA IDENTITY FULL;
  ```
- Default `REPLICA IDENTITY` only includes primary key columns in the old tuple
- `FULL` mode includes all columns, allowing you to see previous values in UPDATE events

## Architecture

The tool consists of the following modules:

### Core Modules
1. **main.rs**: CLI argument parsing and application lifecycle
2. **replication.rs**: PostgreSQL connection and change polling using `pg_logical_slot_get_binary_changes()`
3. **decoder.rs**: pgoutput protocol message decoding with relation caching
4. **output.rs**: Multiple output format support (JSON, pretty JSON, text)
5. **lib.rs**: Library exports for testing

### Testing
- **tests/decoder_tests.rs**: 12 unit tests for protocol decoding
- **tests/output_tests.rs**: 17 unit tests for output formatting
- **Total coverage**: 29 tests with 100% pass rate

For detailed test coverage information, see [TEST_COVERAGE.md](TEST_COVERAGE.md).

### Implementation Details
- Uses SQL-based polling approach with `pg_logical_slot_get_binary_changes()`
- Thread-safe relation metadata caching using `Lazy<Mutex<HashMap>>`
- Change buffering with `VecDeque` to handle multiple changes per poll
- LSN format: `upper32/lower32` hexadecimal representation

## License

[Apache License Version 2.0](LICENSE)

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
