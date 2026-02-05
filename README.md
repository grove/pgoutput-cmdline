# pgoutput-stream

A high-performance Rust command-line tool that streams PostgreSQL logical replication changes to multiple destinations including stdout, NATS JetStream, and Feldera pipelines.

## Features

- 🚀 Stream PostgreSQL logical replication changes in real-time
- 📊 Multiple output formats: JSON, pretty JSON, human-readable text, Debezium CDC, and Feldera InsertDelete
- 📡 Multiple output targets: stdout, NATS JetStream, and Feldera HTTP ingress (can combine multiple targets)
- 🌐 Feldera HTTP input connector with multi-table support
- 🔄 Automatic replication slot creation
- 🎯 Support for all DML operations: INSERT, UPDATE, DELETE
- ⚡ Built with async Rust (Tokio) for high performance
- 🛑 Graceful shutdown on SIGINT/SIGTERM
- 🧪 Comprehensive test coverage (80 unit tests)

## Quick Start

New to this tool? Check out [GETTING_STARTED.md](GETTING_STARTED.md) for a quick guide to get up and running in minutes.

## Prerequisites

PostgreSQL must be configured for logical replication:

1. **Enable logical replication** in `postgresql.conf`:
   ```conf
   wal_level = logical
   max_replication_slots = 10
   max_wal_senders = 10
   ```

2. **Restart PostgreSQL** and create a publication:
   ```sql
   CREATE PUBLICATION my_publication FOR ALL TABLES;
   ```

3. **Set REPLICA IDENTITY FULL** (recommended):
   ```sql
   ALTER TABLE users REPLICA IDENTITY FULL;
   ```

For complete setup instructions including authentication, permissions, and troubleshooting, see [PREREQUISITES.md](PREREQUISITES.md).

## Installation

```bash
# Clone the repository
git clone https://github.com/grove/pgoutput-stream.git
cd pgoutput-stream

# Build the project
cargo build --release

# The binary will be available at:
# ./target/release/pgoutput-stream
```

## Usage

### Basic Usage

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres password=secret dbname=mydb" \
  --slot my_replication_slot \
  --publication my_publication
```

### Output Formats

Choose from multiple output formats using the `--format` option:

```bash
# JSON (default) - compact, one event per line
pgoutput-stream --connection "..." --slot my_slot --publication my_pub --format json

# Human-readable text - great for monitoring
pgoutput-stream --connection "..." --slot my_slot --publication my_pub --format text

# Debezium CDC - compatible with Debezium ecosystems
pgoutput-stream --connection "..." --slot my_slot --publication my_pub --format debezium

# Feldera InsertDelete - optimized for streaming SQL pipelines
pgoutput-stream --connection "..." --slot my_slot --publication my_pub --format feldera
```

**Format Details:**
- **JSON/Pretty JSON**: Standard event streaming format - see [USAGE.md](USAGE.md#output-formats)
- **Text**: Human-readable for debugging - see [USAGE.md](USAGE.md#text)
- **Debezium**: Standard CDC envelope with `before`/`after`/`source`/`op` - see [DEBEZIUM_FORMAT.md](DEBEZIUM_FORMAT.md)
- **Feldera**: InsertDelete format for incremental view maintenance - see [FELDERA_FORMAT.md](FELDERA_FORMAT.md)

### Output Targets

Stream to one or more destinations using the `--target` option:

```bash
# Stdout (default)
pgoutput-stream --connection "..." --slot my_slot --publication my_pub

# NATS JetStream
pgoutput-stream --connection "..." --slot my_slot --publication my_pub \
  --target nats --nats-server "nats://localhost:4222"

# Feldera HTTP
pgoutput-stream --connection "..." --slot my_slot --publication my_pub \
  --format feldera --target feldera \
  --feldera-url "http://localhost:8080" --feldera-pipeline "my_pipeline"

# Multiple targets simultaneously
pgoutput-stream --connection "..." --slot my_slot --publication my_pub \
  --target "stdout,nats,feldera" \
  --nats-server "nats://localhost:4222" \
  --feldera-url "http://localhost:8080" --feldera-pipeline "my_pipeline"
```

**Integration Guides:**
- **NATS JetStream**: Distributed messaging and event-driven architectures - see [NATS_INTEGRATION.md](NATS_INTEGRATION.md)
- **Feldera HTTP**: Real-time streaming SQL pipelines - see [FELDERA_HTTP_CONNECTOR.md](FELDERA_HTTP_CONNECTOR.md)

For complete usage examples, CLI options reference, and advanced patterns, see [USAGE.md](USAGE.md).

## Documentation

Complete guides for specific features:

- **[GETTING_STARTED.md](GETTING_STARTED.md)** - Quick start guide for new users
- **[PREREQUISITES.md](PREREQUISITES.md)** - PostgreSQL configuration and setup
- **[USAGE.md](USAGE.md)** - Complete CLI reference and usage examples
- **[DEBEZIUM_FORMAT.md](DEBEZIUM_FORMAT.md)** - Debezium CDC format specification
- **[FELDERA_FORMAT.md](FELDERA_FORMAT.md)** - Feldera InsertDelete format specification
- **[NATS_INTEGRATION.md](NATS_INTEGRATION.md)** - NATS JetStream integration guide
- **[FELDERA_HTTP_CONNECTOR.md](FELDERA_HTTP_CONNECTOR.md)** - Feldera HTTP connector guide
- **[TEST_COVERAGE.md](TEST_COVERAGE.md)** - Test coverage details

## Troubleshooting

### Connection Errors

**Problem:** Cannot connect to PostgreSQL

**Solutions:**
- Ensure PostgreSQL is running: `pg_isready`
- Verify connection string parameters are correct
- Check user has replication privileges: `ALTER USER postgres WITH REPLICATION;`
- Verify `pg_hba.conf` allows replication connections

See [PREREQUISITES.md](PREREQUISITES.md#troubleshooting) for detailed troubleshooting.

### Slot Already Exists

**Problem:** Replication slot name conflicts with existing slot

**Solutions:**
- Use a different slot name
- Drop the existing slot: `SELECT pg_drop_replication_slot('my_slot');`
- Don't use the `--create-slot` flag (connect to existing slot)

### No Changes Appearing

**Problem:** Tool runs but doesn't show database changes

**Solutions:**
- Verify publication includes your tables: `\dRp+` in psql
- Ensure tables have REPLICA IDENTITY configured
- Check `wal_level = logical`: `SHOW wal_level;`
- Make sure changes are committed in PostgreSQL

### UPDATE Operations Missing Old Values

**Problem:** UPDATE events only show primary key in `before`/old_tuple

**Solution:**
- Set `REPLICA IDENTITY FULL` on your tables:
  ```sql
  ALTER TABLE your_table REPLICA IDENTITY FULL;
  ```
- Default mode only includes primary key columns
- FULL mode includes all columns for complete change history

## Architecture

Brief overview of the tool's design:

### Core Modules

1. **[main.rs](src/main.rs)** - CLI argument parsing and application lifecycle
2. **[replication.rs](src/replication.rs)** - PostgreSQL connection and replication stream management
3. **[decoder.rs](src/decoder.rs)** - Binary pgoutput protocol parser with stateful relation cache
4. **[output.rs](src/output.rs)** - Multi-target output system with format conversions
5. **[lib.rs](src/lib.rs)** - Library exports for testing

### Key Design Patterns

- **SQL-based polling**: Uses `pg_logical_slot_get_binary_changes()` for compatibility
- **Global relation cache**: Thread-safe metadata caching with `Lazy<Mutex<HashMap>>`
- **OutputTarget trait**: Composable multi-destination streaming
- **Type-aware conversion**: PostgreSQL type OIDs → proper JSON types (booleans, numbers, strings)

### Testing

- **[tests/decoder_tests.rs](tests/decoder_tests.rs)** - 12 tests for protocol decoding
- **[tests/output_tests.rs](tests/output_tests.rs)** - 30 tests for format conversions
- **[tests/nats_output_tests.rs](tests/nats_output_tests.rs)** - 16 tests for NATS integration
- **[tests/feldera_output_tests.rs](tests/feldera_output_tests.rs)** - 22 tests for Feldera output
- **Total**: 80 tests with comprehensive coverage

See [TEST_COVERAGE.md](TEST_COVERAGE.md) for details.

## License

[Apache License Version 2.0](LICENSE)

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

For development guidelines and project conventions, see [.github/copilot-instructions.md](.github/copilot-instructions.md).
