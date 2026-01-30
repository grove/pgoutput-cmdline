# NATS Consumer Examples

This directory contains examples for consuming PostgreSQL replication events from NATS JetStream.

## Prerequisites

Install the NATS CLI tool:

```bash
# macOS
brew install nats-io/nats-tools/nats

# Linux
curl -sf https://binaries.nats.dev/nats-io/natscli/nats@latest | sh

# Or download from: https://github.com/nats-io/natscli/releases
```

## Starting NATS Server

### Using Docker

```bash
# Start NATS with JetStream enabled
docker run -d --name nats -p 4222:4222 -p 8222:8222 nats:latest -js

# View logs
docker logs -f nats

# Stop
docker stop nats && docker rm nats
```

### Using Local Installation

```bash
# Download NATS server
# https://docs.nats.io/running-a-nats-service/introduction/installation

# Run with JetStream
nats-server -js
```

## Subscribing to Events

### Subscribe to All Events

```bash
nats sub "postgres.>"
```

### Subscribe to Specific Table

```bash
# All operations on users table
nats sub "postgres.public.users.*"

# Only INSERT operations on users table
nats sub "postgres.public.users.insert"

# Only UPDATE operations on users table
nats sub "postgres.public.users.update"
```

### Subscribe to Specific Operation Across All Tables

```bash
# All INSERT operations
nats sub "postgres.*.*.insert"

# All UPDATE operations
nats sub "postgres.*.*.update"

# All DELETE operations
nats sub "postgres.*.*.delete"
```

### Subscribe to Transaction Boundaries

```bash
# All transaction events
nats sub "postgres.transactions.*"

# Only BEGIN events
nats sub "postgres.transactions.begin.event"

# Only COMMIT events
nats sub "postgres.transactions.commit.event"
```

## JetStream Consumer Examples

### Create a Durable Consumer

```bash
# Create a durable consumer that tracks progress
nats consumer add postgres_replication user_insert_consumer \
  --filter "postgres.public.users.insert" \
  --deliver all \
  --replay instant \
  --ack explicit

# Consume messages
nats consumer next postgres_replication user_insert_consumer
```

### Pull-Based Consumer

```bash
# Create pull consumer
nats consumer add postgres_replication all_events_pull \
  --pull \
  --deliver all \
  --ack explicit

# Pull messages (with auto-ack)
nats consumer next postgres_replication all_events_pull --count 10
```

### Queue Group for Load Balancing

```bash
# Multiple instances will share the workload
nats sub "postgres.public.orders.*" --queue order-processors

# Run this command in multiple terminals to see load balancing
```

## Programmatic Consumer (Python)

```python
#!/usr/bin/env python3
import asyncio
import json
from nats.aio.client import Client as NATS
from nats.js.api import ConsumerConfig

async def main():
    nc = NATS()
    await nc.connect("nats://localhost:4222")
    
    # Get JetStream context
    js = nc.jetstream()
    
    # Subscribe to user inserts
    async def message_handler(msg):
        data = json.loads(msg.data.decode())
        print(f"Received INSERT: {data}")
        await msg.ack()
    
    # Create subscription
    await js.subscribe(
        "postgres.public.users.insert",
        cb=message_handler,
        durable="user_insert_handler",
        stream="postgres_replication"
    )
    
    # Keep running
    while True:
        await asyncio.sleep(1)

if __name__ == '__main__':
    asyncio.run(main())
```

Install dependencies:
```bash
pip install nats-py
```

## Programmatic Consumer (Node.js)

```javascript
const { connect, StringCodec } = require('nats');

async function main() {
    // Connect to NATS
    const nc = await connect({ servers: 'nats://localhost:4222' });
    const js = nc.jetstream();
    const sc = StringCodec();
    
    // Subscribe to user inserts
    const sub = await js.subscribe('postgres.public.users.insert', {
        stream: 'postgres_replication',
        durable: 'user_insert_handler',
        deliver_policy: 'all',
        ack_policy: 'explicit'
    });
    
    console.log('Listening for user inserts...');
    
    for await (const msg of sub) {
        const data = JSON.parse(sc.decode(msg.data));
        console.log('Received INSERT:', data);
        msg.ack();
    }
}

main().catch(console.error);
```

Install dependencies:
```bash
npm install nats
```

## Stream Inspection

### View Stream Information

```bash
# List all streams
nats stream ls

# Get stream info
nats stream info postgres_replication

# View subjects in stream
nats stream subjects postgres_replication
```

### View Messages

```bash
# View messages in stream
nats stream view postgres_replication

# View last 10 messages
nats stream view postgres_replication --last 10

# View messages for specific subject
nats stream view postgres_replication --subject "postgres.public.users.insert"
```

### Consumer Management

```bash
# List consumers
nats consumer ls postgres_replication

# Get consumer info
nats consumer info postgres_replication user_insert_consumer

# Delete consumer
nats consumer rm postgres_replication user_insert_consumer
```

## Event Format

All events are published as JSON. Example INSERT event:

```json
{
  "Insert": {
    "relation_id": 16384,
    "schema": "public",
    "table": "users",
    "new_tuple": {
      "id": "1",
      "name": "Alice",
      "email": "alice@example.com",
      "created_at": "2024-01-15 10:30:00"
    }
  }
}
```

Example UPDATE event:

```json
{
  "Update": {
    "relation_id": 16384,
    "schema": "public",
    "table": "users",
    "old_tuple": {
      "id": "1",
      "name": "Alice",
      "email": "alice@example.com"
    },
    "new_tuple": {
      "id": "1",
      "name": "Alice",
      "email": "alice.new@example.com"
    }
  }
}
```

## Complete Workflow Example

### Terminal 1: Start NATS

```bash
docker run -p 4222:4222 -p 8222:8222 nats:latest -js
```

### Terminal 2: Start pgoutput-cmdline

```bash
./examples/run_with_nats.sh
```

### Terminal 3: Subscribe to Events

```bash
nats sub "postgres.public.users.*"
```

### Terminal 4: Make Database Changes

```bash
psql -U postgres -d replication_test -c "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');"
```

You should see the event in Terminal 3!

## Troubleshooting

### Connection Refused

- Ensure NATS server is running: `docker ps` or `ps aux | grep nats-server`
- Check port 4222 is accessible: `nc -zv localhost 4222`

### Stream Not Found

- The stream is created automatically by pgoutput-cmdline when it starts
- Or create manually: `nats stream add postgres_replication --subjects "postgres.*.*.*"`

### No Messages Appearing

- Verify pgoutput-cmdline is running with `--nats-server` flag
- Check stream has messages: `nats stream info postgres_replication`
- Verify subject pattern matches: `nats stream subjects postgres_replication`

## Additional Resources

- [NATS Documentation](https://docs.nats.io/)
- [JetStream Guide](https://docs.nats.io/nats-concepts/jetstream)
- [NATS CLI Reference](https://docs.nats.io/using-nats/nats-tools/nats_cli)
