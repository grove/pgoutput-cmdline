#!/bin/bash
# Test script for Feldera HTTP connector
# This demonstrates how to stream PostgreSQL changes directly to Feldera

# Configuration
POSTGRES_HOST="localhost"
POSTGRES_USER="postgres"
POSTGRES_DB="replication_test"
SLOT_NAME="feldera_test_slot"
PUBLICATION="test_pub"
FELDERA_URL="http://localhost:8080"
FELDERA_PIPELINE="postgres_cdc"
FELDERA_TABLE="users"

# Build the project
echo "Building pgoutput-cmdline..."
cargo build --release

# Run with Feldera target
echo "Starting replication to Feldera..."
echo "Target: $FELDERA_URL/v0/pipelines/$FELDERA_PIPELINE/ingress/$FELDERA_TABLE"
echo ""

./target/release/pgoutput-cmdline \
    --connection "host=$POSTGRES_HOST user=$POSTGRES_USER dbname=$POSTGRES_DB replication=database" \
    --slot "$SLOT_NAME" \
    --publication "$PUBLICATION" \
    --format feldera \
    --target feldera \
    --feldera-url "$FELDERA_URL" \
    --feldera-pipeline "$FELDERA_PIPELINE" \
    --feldera-table "$FELDERA_TABLE" \
    --create-slot

# Note: For multiple targets, use comma-separated values:
# --target "stdout,feldera" will output to both stdout and Feldera
# --target "stdout,nats,feldera" will output to all three targets
