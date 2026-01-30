# pgoutput-cmdline

A Rust command-line tool that consumes PostgreSQL logical replication streams using the pgoutput plugin and outputs the changes to stdout.

## Features

- 🚀 Stream PostgreSQL logical replication changes in real-time
- 📊 Multiple output formats: JSON, pretty JSON, human-readable text, and Debezium CDC
- 📡 **Publish to NATS JetStream** for distributed event streaming
- 🔄 Automatic replication slot creation
- 🎯 Support for all DML operations: INSERT, UPDATE, DELETE
- ⚡ Built with async Rust (Tokio) for high performance
- 🛑 Graceful shutdown on SIGINT/SIGTERM
- 🧪 Comprehensive test coverage (68 unit tests)

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

### Command-Line Options

```
Options:
  -c, --connection <CONNECTION>  PostgreSQL connection string
  -s, --slot <SLOT>             Replication slot name
  -p, --publication <PUBLICATION> Publication name
  -f, --format <FORMAT>         Output format: json, json-pretty, text, or debezium [default: json]
      --create-slot             Create replication slot if it doesn't exist
      --start-lsn <START_LSN>   Starting LSN (Log Sequence Number) to stream from
      --nats-server <URL>       NATS server URL (e.g., "nats://localhost:4222")
      --nats-stream <STREAM>    NATS JetStream stream name [default: postgres_replication]
      --nats-subject-prefix <PREFIX> NATS subject prefix [default: postgres]
  -h, --help                    Print help
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
# Stream to both stdout and NATS
pgoutput-cmdline \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --nats-server "nats://localhost:4222" \
  --nats-stream "postgres_replication" \
  --nats-subject-prefix "postgres"
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

MIT

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
