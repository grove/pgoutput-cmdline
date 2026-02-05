#!/bin/bash

# Example script to run pgoutput-stream
# Adjust connection parameters as needed

# Build the project first
echo "Building pgoutput-stream..."
cargo build --release

# Run with JSON output (default)
echo "Starting replication stream with JSON output..."
./target/release/pgoutput-stream \
  --connection "host=localhost user=geir.gronmo dbname=replication_test" \
  --slot test_slot \
  --publication test_publication \
  --create-slot \
  --format json

# Alternative: Run with text output
# ./target/release/pgoutput-stream \
#   --connection "host=localhost user=geir.gronmo password=yourpassword dbname=replication_test" \
#   --slot test_slot \
#   --publication test_publication \
#   --format text

# Alternative: Run with pretty JSON
# ./target/release/pgoutput-stream \
#   --connection "host=localhost user=geir.gronmo dbname=replication_test" \
#   --slot test_slot \
#   --publication test_publication \
#   --format json-pretty
