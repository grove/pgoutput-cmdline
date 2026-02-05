# NATS JetStream Integration Guide

Stream PostgreSQL replication events to NATS JetStream for distributed messaging, event-driven architectures, and microservices.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Subject Naming](#subject-naming)
- [Usage Examples](#usage-examples)
- [Consumer Patterns](#consumer-patterns)
- [Configuration](#configuration)
- [Use Cases](#use-cases)
- [Troubleshooting](#troubleshooting)
- [Implementation Details](#implementation-details)

## Overview

The NATS integration enables streaming PostgreSQL changes to NATS JetStream, a distributed messaging system perfect for:

- **Event-Driven Architectures**: Multiple microservices react to database changes
- **Real-Time Analytics**: Stream changes to analytics engines and dashboards
- **Data Synchronization**: Keep multiple systems in sync with your database
- **Audit Logs**: Durable, replayable log of all database changes
- **CDC Pipelines**: Feed data warehouses, data lakes, and ETL systems

**Key Features:**
- Automatic JetStream stream creation and management
- Hierarchical subject naming for flexible subscriptions
- Support for multiple simultaneous outputs (stdout + NATS)
- Async, non-blocking I/O for high performance
- Production-ready error handling

## Quick Start

### 1. Start NATS Server

```bash
# Using Docker (recommended for quick start)
docker run -p 4222:4222 -p 8222:8222 nats:latest -js

# Or install NATS locally
# https://docs.nats.io/running-a-nats-service/introduction/installation
```

### 2. Stream to NATS

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --target nats \
  --nats-server "nats://localhost:4222"
```

### 3. Subscribe to Events

```bash
# In another terminal - subscribe to all events
nats sub "postgres.>"

# Or subscribe to specific table
nats sub "postgres.public.users.*"
```

### 4. Make Database Changes

```bash
psql -U postgres -d mydb -c \
  "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');"
```

You should see the event appear in your NATS subscriber!

## Subject Naming

Events are published to hierarchical subjects enabling flexible subscriptions.

### Table Operations

**Pattern:** `{prefix}.{schema}.{table}.{operation}`

**Examples:**
- `postgres.public.users.insert`
- `postgres.public.orders.update`
- `postgres.public.products.delete`
- `postgres.analytics.events.insert`

### Transaction Boundaries

**Pattern:** `{prefix}.transactions.{event}.event`

**Examples:**
- `postgres.transactions.begin.event`
- `postgres.transactions.commit.event`

### Schema Metadata

**Pattern:** `{prefix}.{schema}.{table}.relation`

**Examples:**
- `postgres.public.users.relation`
- `postgres.public.orders.relation`

### Wildcard Subscriptions

NATS supports wildcard patterns for flexible subscriptions:

| Pattern | Matches | Example Use Case |
|---------|---------|------------------|
| `postgres.>` | All events | Audit log, catch-all processor |
| `postgres.public.*.*` | All operations on public schema | Schema-specific handler |
| `postgres.public.users.*` | All operations on users table | User service |
| `postgres.*.*.insert` | All INSERT operations | Insert-only processor |
| `postgres.*.*.update` | All UPDATE operations | Change tracker |
| `postgres.*.*.delete` | All DELETE operations | Deletion audit |
| `postgres.transactions.*event` | Transaction markers | Transaction coordinator |

## Usage Examples

### Stream to NATS Only

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --target nats \
  --nats-server "nats://localhost:4222"
```

### Stream to Both Stdout and NATS

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --target "stdout,nats" \
  --nats-server "nats://localhost:4222"
```

**Tip:** This is great for debugging - see events in your terminal while also streaming to NATS.

### Custom Stream and Subject Prefix

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot my_slot \
  --publication my_pub \
  --target nats \
  --nats-server "nats://localhost:4222" \
  --nats-stream "my_custom_stream" \
  --nats-subject-prefix "myapp"
```

This will publish to subjects like `myapp.public.users.insert` instead of `postgres.public.users.insert`.

### Combine with Other Targets

```bash
# Stream to stdout, NATS, and Feldera simultaneously
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

## Consumer Patterns

### Basic CLI Subscription

```bash
# Subscribe to all events
nats sub "postgres.>"

# Subscribe to specific table
nats sub "postgres.public.users.*"

# Subscribe to specific operation across all tables
nats sub "postgres.*.*.insert"

# Subscribe to specific table and operation
nats sub "postgres.public.users.update"

# Subscribe to all tables in a schema
nats sub "postgres.public.*.*"

# Subscribe to transaction markers
nats sub "postgres.transactions.*.event"
```

### Python Consumer Example

```python
import asyncio
import json
from nats.aio.client import Client as NATS

async def main():
    # Connect to NATS
    nc = NATS()
    await nc.connect("nats://localhost:4222")
    
    # Access JetStream
    js = nc.jetstream()
    
    # Define message handler
    async def handler(msg):
        data = json.loads(msg.data.decode())
        
        # Process the event
        if 'Insert' in data:
            print(f"New record: {data['Insert']}")
        elif 'Update' in data:
            print(f"Updated record: {data['Update']}")
        elif 'Delete' in data:
            print(f"Deleted record: {data['Delete']}")
        
        # Acknowledge processing
        await msg.ack()
    
    # Create durable consumer
    await js.subscribe(
        "postgres.public.users.*",  # Subject pattern
        cb=handler,
        durable="user_processor",  # Durable name
        stream="postgres_replication"
    )
    
    print("Listening for user events...")
    
    # Keep running
    while True:
        await asyncio.sleep(1)

if __name__ == "__main__":
    asyncio.run(main())
```

### Node.js Consumer Example

```javascript
const { connect, StringCodec } = require('nats');

async function main() {
  // Connect to NATS
  const nc = await connect({ servers: 'nats://localhost:4222' });
  
  // Access JetStream
  const js = nc.jetstream();
  const sc = StringCodec();
  
  // Create consumer
  const consumer = await js.consumers.get(
    'postgres_replication',
    'user_processor'
  );
  
  // Consume messages
  const messages = await consumer.consume();
  
  console.log('Listening for user events...');
  
  for await (const msg of messages) {
    const data = JSON.parse(sc.decode(msg.data));
    
    // Process the event
    if (data.Insert) {
      console.log('New record:', data.Insert);
    } else if (data.Update) {
      console.log('Updated record:', data.Update);
    } else if (data.Delete) {
      console.log('Deleted record:', data.Delete);
    }
    
    // Acknowledge processing
    msg.ack();
  }
}

main().catch(console.error);
```

### Go Consumer Example

```go
package main

import (
    "encoding/json"
    "fmt"
    "log"
    
    "github.com/nats-io/nats.go"
)

func main() {
    // Connect to NATS
    nc, err := nats.Connect("nats://localhost:4222")
    if err != nil {
        log.Fatal(err)
    }
    defer nc.Close()
    
    // Access JetStream
    js, err := nc.JetStream()
    if err != nil {
        log.Fatal(err)
    }
    
    // Subscribe with durable consumer
    _, err = js.Subscribe(
        "postgres.public.users.*",
        func(msg *nats.Msg) {
            var data map[string]interface{}
            json.Unmarshal(msg.Data, &data)
            
            fmt.Printf("Received: %+v\n", data)
            
            // Acknowledge processing
            msg.Ack()
        },
        nats.Durable("user_processor"),
        nats.Stream("postgres_replication"),
    )
    if err != nil {
        log.Fatal(err)
    }
    
    fmt.Println("Listening for user events...")
    
    // Keep running
    select {}
}
```

More consumer examples available in [examples/NATS_CONSUMERS.md](examples/NATS_CONSUMERS.md).

## Configuration

### CLI Options

| Option | Description | Default | Required |
|--------|-------------|---------|----------|
| `--target` | Must include `nats` | `stdout` | Yes (for NATS) |
| `--nats-server` | NATS server URL | - | Yes |
| `--nats-stream` | JetStream stream name | `postgres_replication` | No |
| `--nats-subject-prefix` | Subject prefix | `postgres` | No |

### JetStream Stream Configuration

The tool automatically creates a JetStream stream with these settings:

| Setting | Value | Description |
|---------|-------|-------------|
| Name | Configurable via `--nats-stream` | Stream identifier |
| Subjects | `{prefix}.*.*.*` | Captures all events |
| Storage | Memory | Fast, non-persistent (can be changed) |
| Max Messages | 1,000,000 | Message limit before eviction |
| Max Bytes | 1 GB | Size limit before eviction |
| Retention | Limits | Evict old messages when limits reached |

**Customizing Stream:**

To modify the stream configuration, use NATS CLI:

```bash
# Create stream with custom settings before running pgoutput-stream
nats stream add postgres_replication \
  --subjects "postgres.*.*.*" \
  --storage file \
  --retention limits \
  --max-msgs 10000000 \
  --max-bytes 10GB \
  --max-age 7d

# Or edit existing stream
nats stream edit postgres_replication --max-age 24h
```

## Use Cases

### 1. Event-Driven Microservices

Multiple services subscribe to relevant database changes:

```bash
# User service listens to user changes
nats sub "postgres.public.users.*"

# Order service listens to order changes
nats sub "postgres.public.orders.*"

# Notification service listens to all changes
nats sub "postgres.>"
```

### 2. Real-Time Analytics Dashboard

Stream database changes to analytics processors:

```bash
# Analytics service processes all INSERT operations
nats sub "postgres.*.*.insert"

# Dashboard listens to specific tables
nats sub "postgres.public.sales.*"
```

### 3. Data Synchronization

Keep multiple systems synchronized with your database:

```bash
# Elasticsearch indexer
nats sub "postgres.public.products.*"

# Redis cache invalidator
nats sub "postgres.public.*.update,postgres.public.*.delete"

# External API sync
nats sub "postgres.>"
```

### 4. Audit Logging

Durable, replayable audit trail:

```bash
# Create durable consumer for audit
nats consumer add postgres_replication audit_logger \
  --filter "postgres.>" \
  --deliver all \
  --ack explicit \
  --replay instant

# Replay from beginning for audit analysis
nats consumer next postgres_replication audit_logger --no-ack
```

### 5. CDC to Data Warehouse

Feed data pipeline from PostgreSQL to warehouse:

```bash
# ETL service processes all data changes
nats sub "postgres.public.*.*"

# Skip transaction markers if not needed
nats sub "postgres.public.*.*,postgres.analytics.*.*"
```

## Troubleshooting

### NATS Connection Failed

**Problem:** Cannot connect to NATS server

**Solutions:**
- Verify NATS is running: `nats server ping`
- Check server URL is correct (default: `nats://localhost:4222`)
- Verify firewall allows connection to port 4222
- Check NATS logs: `docker logs <container-id>`

### Stream Not Created

**Problem:** JetStream stream doesn't exist

**Solutions:**
- Verify JetStream is enabled on NATS server (`-js` flag)
- Check NATS CLI: `nats stream ls`
- Try creating manually: `nats stream add postgres_replication`
- Check NATS server has sufficient resources (disk space, memory)

### No Messages Appearing

**Problem:** Subscriptions don't receive events

**Solutions:**
- Verify pgoutput-stream is running and connected to NATS
- Check subscription pattern matches published subjects
- Use `nats sub "postgres.>"` to see all events
- Check stream status: `nats stream info postgres_replication`
- Verify database changes are being made and captured by PostgreSQL

### Consumer Falling Behind

**Problem:** Consumer can't keep up with event rate

**Solutions:**
- Increase consumer concurrency (process messages in parallel)
- Add more consumer instances
- Use work queue retention policy for load balancing
- Optimize message processing logic
- Consider batching acknowledgments

### Message Delivery Issues

**Problem:** Messages delivered multiple times or out of order

**Solutions:**
- Use durable consumers with explicit acknowledgment
- Implement idempotent message processing (handle duplicates)
- Note: NATS doesn't guarantee strict ordering across subjects
- Consider using stream sequences for ordering

## Implementation Details

### Architecture

The NATS integration uses a trait-based architecture for extensibility:

```rust
#[async_trait::async_trait]
pub trait OutputTarget: Send + Sync {
    async fn write_change(&self, change: &Change) -> Result<()>;
}
```

**Implementations:**
1. **StdoutOutput** - Terminal output
2. **NatsOutput** - NATS JetStream publisher
3. **FelderaOutput** - Feldera HTTP connector
4. **CompositeOutput** - Multiplexer for multiple outputs

### Dependencies

- `async-nats = "0.33"` - Official NATS Rust client with JetStream support
- `async-trait = "0.1"` - Async trait definitions

### Event Publishing

Events are serialized to JSON and published to NATS:

1. PostgreSQL change captured by replication stream
2. Change decoded from pgoutput binary protocol
3. Change converted to JSON format (respects `--format` option)
4. JSON published to appropriate NATS subject
5. Async publish with error handling

### Performance Characteristics

- **Non-blocking**: All NATS operations use async/await
- **Memory efficient**: Events streamed, not buffered
- **Overhead**: ~1-5KB JSON per event, negligible CPU
- **Throughput**: Limited by PostgreSQL replication rate and network bandwidth

### Error Handling

- Connection failures: Logged and propagated (tool will exit)
- Publish failures: Logged with context (which event failed)
- Stream creation: Idempotent (safe to re-run)

### Future Enhancements

Potential improvements:

1. **Batching**: Group transaction events into single message
2. **Filtering**: CLI flags for selective publishing (specific tables/operations)
3. **Retry**: Configurable retry policies for transient failures
4. **Buffering**: Local buffer for NATS outages with replay
5. **Metrics**: Publish success/failure rates, lag monitoring
6. **Multiple Servers**: Fan-out to  multiple NATS clusters
7. **Schema Registry**: Avro/Protobuf encoding support
8. **Message Deduplication**: Handle redelivery scenarios

## See Also

- **[examples/run_with_nats.sh](examples/run_with_nats.sh)** - Turnkey test script
- **[examples/NATS_CONSUMERS.md](examples/NATS_CONSUMERS.md)** - More consumer examples
- **[USAGE.md](USAGE.md)** - Complete CLI reference
- **[README.md](README.md)** - Project overview
- **[NATS Documentation](https://docs.nats.io/)** - Official NATS docs
- **[JetStream Guide](https://docs.nats.io/nats-concepts/jetstream)** - JetStream concepts
