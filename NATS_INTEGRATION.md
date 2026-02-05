# NATS JetStream Integration - Implementation Summary

## Overview

Added NATS JetStream integration to pgoutput-stream, enabling PostgreSQL replication events to be published to a distributed message streaming platform. The implementation supports dual output (stdout + NATS) with a clean, extensible architecture.

## Changes Made

### 1. Dependencies Added ([Cargo.toml](Cargo.toml))
- `async-nats = "0.33"` - Official NATS client with JetStream support
- `async-trait = "0.1"` - Required for async trait definitions

### 2. Output Architecture Refactoring ([src/output.rs](src/output.rs))

#### New `OutputTarget` Trait
```rust
#[async_trait::async_trait]
pub trait OutputTarget: Send + Sync {
    async fn write_change(&self, change: &Change) -> Result<()>;
}
```

#### Implementations
1. **`StdoutOutput`** - Existing stdout functionality wrapped in trait
   - Supports JSON, JSON-pretty, and text formats
   
2. **`NatsOutput`** - New NATS JetStream publisher
   - Auto-creates or connects to existing JetStream stream
   - Publishes events to hierarchical subjects
   - Subjects: `{prefix}.{schema}.{table}.{operation}`
   - Stream configuration: 1M messages, 1GB max, limit-based retention

3. **`CompositeOutput`** - Multiplexer for multiple outputs
   - Enables simultaneous stdout and NATS publishing
   - Extensible for future output targets (Kafka, files, etc.)

### 3. CLI Arguments ([src/main.rs](src/main.rs))

Added three new optional arguments:
- `--nats-server <URL>` - NATS server connection (e.g., "nats://localhost:4222")
- `--nats-stream <STREAM>` - JetStream stream name (default: "postgres_replication")
- `--nats-subject-prefix <PREFIX>` - Subject namespace (default: "postgres")

### 4. Main Loop Refactoring ([src/main.rs](src/main.rs))

- Builds output target pipeline based on CLI flags
- Always includes `StdoutOutput` (preserves existing behavior)
- Conditionally adds `NatsOutput` when `--nats-server` provided
- Uses async `write_change()` instead of synchronous `print_change()`

### 5. Documentation

- Updated [README.md](README.md) with NATS integration section
  - Setup instructions
  - Usage examples
  - Subject naming conventions
  - Use cases
  
- Created [examples/run_with_nats.sh](examples/run_with_nats.sh)
  - Turnkey script for testing NATS integration
  
- Created [examples/NATS_CONSUMERS.md](examples/NATS_CONSUMERS.md)
  - Comprehensive guide for consuming events from NATS
  - Examples in Bash, Python, Node.js
  - Stream management commands
  - Troubleshooting guide

## NATS Subject Hierarchy

Events are published to subjects following this pattern:

### Table Operations
```
{prefix}.{schema}.{table}.{operation}
```
Examples:
- `postgres.public.users.insert`
- `postgres.public.orders.update`
- `postgres.sales.products.delete`

### Transaction Boundaries
```
{prefix}.transactions.{event}.event
```
Examples:
- `postgres.transactions.begin.event`
- `postgres.transactions.commit.event`

### Schema Metadata
```
{prefix}.{schema}.{table}.relation
```
Example:
- `postgres.public.users.relation`

## Usage Examples

### Basic Usage (stdout only)
```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub
```

### With NATS (stdout + NATS)
```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --nats-server "nats://localhost:4222"
```

### Custom Stream Configuration
```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --nats-server "nats://localhost:4222" \
  --nats-stream "my_custom_stream" \
  --nats-subject-prefix "myapp"
```

## Consumer Examples

### Subscribe to All Events
```bash
nats sub "postgres.>"
```

### Subscribe to Specific Table
```bash
nats sub "postgres.public.users.*"
```

### Subscribe to Specific Operation
```bash
nats sub "postgres.*.*.insert"  # All inserts
nats sub "postgres.public.users.update"  # User updates only
```

### Programmatic Consumer (Python)
```python
import asyncio
import json
from nats.aio.client import Client as NATS

async def main():
    nc = NATS()
    await nc.connect("nats://localhost:4222")
    js = nc.jetstream()
    
    async def handler(msg):
        data = json.loads(msg.data.decode())
        print(f"Received: {data}")
        await msg.ack()
    
    await js.subscribe(
        "postgres.public.users.insert",
        cb=handler,
        durable="user_insert_handler",
        stream="postgres_replication"
    )
    
    while True:
        await asyncio.sleep(1)

asyncio.run(main())
```

## Architecture Benefits

### 1. Clean Separation of Concerns
- Replication logic unchanged
- Output handling abstracted into trait
- Easy to add new output targets

### 2. Zero Impact on Existing Functionality
- Stdout output preserved by default
- No breaking changes to CLI
- Backward compatible

### 3. Extensibility
- New output targets (Kafka, RabbitMQ, files) trivial to add
- Subject naming strategy configurable
- Stream configuration customizable

### 4. Production Ready
- Async/await throughout
- Graceful error handling
- Auto-creates JetStream stream
- Works with existing or new streams

## Testing

### Build and Test
```bash
cargo build --release
cargo test

# Verify no errors
cargo check
```

### Manual Testing
```bash
# Terminal 1: Start NATS
docker run -p 4222:4222 nats:latest -js

# Terminal 2: Start PostgreSQL (if not running)
# Ensure setup.sql has been run

# Terminal 3: Start pgoutput-stream
./examples/run_with_nats.sh

# Terminal 4: Subscribe to events
nats sub "postgres.>"

# Terminal 5: Make database changes
psql -U postgres -d replication_test -c \
  "INSERT INTO users (name, email) VALUES ('Test', 'test@example.com');"
```

You should see the event in both Terminal 3 (stdout) and Terminal 4 (NATS subscriber).

## Future Enhancements

Potential improvements for future iterations:

1. **Batching**: Group transaction events (Begin → Changes → Commit) into single message
2. **Filtering**: CLI flags to publish only specific tables/operations
3. **Error Handling**: Configurable retry policies and buffering for NATS failures
4. **Metrics**: Expose publish success/failure rates
5. **Multiple NATS Connections**: Fan-out to multiple NATS servers
6. **Schema Registry**: Integrate with schema registry for Avro/Protobuf encoding
7. **Dead Letter Queue**: Handle failed messages with DLQ pattern
8. **Observability**: OpenTelemetry tracing integration

## Performance Considerations

- **Async I/O**: Both PostgreSQL and NATS operations are async, no blocking
- **Connection Pooling**: Could add connection pool for high-throughput scenarios
- **Memory**: Stream configured with 1GB limit, adjust based on retention needs
- **Network**: Events serialized to JSON, ~1-5KB per event typically

## Known Limitations

1. **No Transaction Grouping**: Events published individually, not grouped by transaction
2. **No Message Deduplication**: Redelivery scenarios could cause duplicates
3. **Fixed JSON Serialization**: No support for Avro, Protobuf, or other formats
4. **No Filtering**: All events published, no selective publishing

## Conclusion

The NATS JetStream integration is fully functional and production-ready for many use cases. The clean trait-based architecture makes it easy to extend with additional features or output targets as needed.
