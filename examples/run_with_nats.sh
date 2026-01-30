#!/bin/bash

# Example script to run pgoutput-cmdline with NATS JetStream integration
# 
# Prerequisites:
# 1. PostgreSQL with logical replication enabled
# 2. NATS server with JetStream enabled
# 3. Database and publication set up (see setup.sql)

# PostgreSQL connection details
PG_HOST="localhost"
PG_USER="postgres"
PG_DB="replication_test"
PG_SLOT="test_slot"
PG_PUBLICATION="test_publication"

# NATS connection details
NATS_SERVER="nats://localhost:4222"
NATS_STREAM="postgres_replication"
NATS_PREFIX="postgres"

echo "Starting pgoutput-cmdline with NATS JetStream integration..."
echo ""
echo "PostgreSQL:"
echo "  Host: $PG_HOST"
echo "  Database: $PG_DB"
echo "  Slot: $PG_SLOT"
echo "  Publication: $PG_PUBLICATION"
echo ""
echo "NATS:"
echo "  Server: $NATS_SERVER"
echo "  Stream: $NATS_STREAM"
echo "  Subject Prefix: $NATS_PREFIX"
echo ""
echo "Events will be:"
echo "  - Printed to stdout (JSON format)"
echo "  - Published to NATS JetStream"
echo ""
echo "To subscribe to events, run in another terminal:"
echo "  nats sub 'postgres.>'"
echo "  nats sub 'postgres.public.users.insert'"
echo ""
echo "Press Ctrl-C to stop"
echo ""

# Build and run the tool
cargo run --release -- \
  --connection "host=$PG_HOST user=$PG_USER dbname=$PG_DB" \
  --slot "$PG_SLOT" \
  --publication "$PG_PUBLICATION" \
  --create-slot \
  --format json \
  --nats-server "$NATS_SERVER" \
  --nats-stream "$NATS_STREAM" \
  --nats-subject-prefix "$NATS_PREFIX"
