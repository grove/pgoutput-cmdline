# pgoutput-cmdline Agent Guidelines

A PostgreSQL logical replication streaming tool that decodes pgoutput protocol messages and distributes changes to multiple destinations (stdout, NATS JetStream, Feldera HTTP).

## Architecture

### Module Responsibilities

- [src/decoder.rs](../src/decoder.rs) - Binary pgoutput protocol parser with stateful relation cache
- [src/output.rs](../src/output.rs) - Multi-target output system using trait-based strategy pattern
- [src/replication.rs](../src/replication.rs) - PostgreSQL connection and replication stream management
- [src/main.rs](../src/main.rs) - CLI orchestration and main processing loop

### Data Flow

```
PostgreSQL WAL → ReplicationStream (buffered VecDeque) → Decoder → Change enum → CompositeOutput → OutputTarget(s)
```

**Key Design Decisions:**
- **Global relation cache** (`Lazy<Mutex<HashMap>>`) avoids repeated metadata queries
- **OutputTarget trait** enables composable, simultaneous output to multiple destinations
- **Feldera updates decompose** into delete + insert pairs for incremental view maintenance
- **Type-aware conversion** uses PostgreSQL type OIDs to produce proper JSON types (booleans, numbers, strings)

## Build and Test

```bash
# Development
cargo build
cargo test

# Production
cargo build --release
./target/release/pgoutput-cmdline

# Pre-commit checks
cargo fmt && cargo clippy && cargo test

# Test suites (58 tests total)
cargo test --test decoder_tests     # 12 tests - protocol parsing
cargo test --test output_tests      # 30 tests - format conversions
cargo test --test nats_output_tests # 16 tests - NATS integration

# Examples
./examples/run.sh              # Basic stdout demo
./examples/run_with_nats.sh    # NATS integration
./examples/test_feldera_http.sh # Feldera HTTP connector
```

## Project Conventions

### Error Handling Pattern

- Use `anyhow::Result<T>` for flexible error context
- Never panic in library code - always return `Result`
- Provide contextual error messages: `Err(anyhow!("--nats-server required when target includes 'nats'"))`
- Graceful degradation for non-critical errors (see replication slot handling in [replication.rs](../src/replication.rs))

### Binary Protocol Parsing

Manual byte manipulation with explicit offset tracking ([decoder.rs](../src/decoder.rs)):

```rust
let relation_id = u32::from_be_bytes(data[pos..pos + 4].try_into()?);
pos += 4;
```

Always validate buffer length before reading to prevent panics.

### Output Target Implementation

When adding new output destinations:

1. Implement `OutputTarget` trait in [output.rs](../src/output.rs)
2. Add CLI arguments in [main.rs](../src/main.rs) `Cli` struct
3. Update target match block in `main()` to instantiate your target
4. Add validation for required arguments
5. Write integration tests in `tests/` directory

Example trait implementation:

```rust
#[async_trait::async_trait]
impl OutputTarget for MyOutput {
    async fn write_change(&self, change: &Change) -> anyhow::Result<()> {
        // Handle Change enum variants
    }
}
```

### Format Conversions

- **Debezium CDC**: See [convert_to_debezium()](../src/output.rs#L133) - envelope structure with `before`/`after`/`source`
- **Feldera InsertDelete**: See [convert_to_feldera()](../src/output.rs#L226) - splits updates into separate events
- **Type conversion**: Use [tuple_to_json_with_types()](../src/output.rs#L10) for proper JSON type mapping (OID 16→bool, 20/21/23→int, 700/701→float)

### Testing Patterns

Based on existing test files:

```rust
#[test]
fn test_feature_name() {
    // Setup
    let input = /* ... */;
    
    // Execute
    let result = function_under_test(input).unwrap();
    
    // Assert
    assert_eq!(result.field, expected_value);
}
```

Mock async dependencies using test doubles. See [nats_output_tests.rs](../tests/nats_output_tests.rs) for async test patterns.

## Integration Points

### PostgreSQL Protocol

- Relies on pgoutput logical decoding plugin (core PostgreSQL feature)
- Requires `wal_level=logical` and replication slots configured
- Message types: Begin, Commit, Relation, Insert, Update, Delete, Truncate, Type
- REPLICA IDENTITY FULL required for complete UPDATE old tuples

### NATS JetStream

- Subject hierarchy: `{prefix}.{schema}.{table}.{operation}`
- Auto-provisions stream with 1M message / 1GB limit
- Connection via [async-nats](https://crates.io/crates/async-nats) crate
- See [NATS_INTEGRATION.md](../NATS_INTEGRATION.md) for consumer patterns

### Feldera HTTP Ingress

- Endpoint: `/v0/pipelines/{pipeline}/ingress/{table}?format=json&update_format=insert_delete&array=true`
- Batches changes as JSON array, sends via [reqwest](https://crates.io/crates/reqwest)
- Optional Bearer token authentication
- See [FELDERA_HTTP_CONNECTOR.md](../FELDERA_HTTP_CONNECTOR.md) for details

## Code Style

Follow [rust.instructions.md](../rust.instructions.md) for Rust-specific conventions. Project-specific additions:

- **Line length**: Keep under 100 characters
- **Module structure**: Split binary (`main.rs`) from library (`lib.rs`) for testability
- **Async**: Use `tokio::select!` for graceful shutdown patterns (see [main.rs](../src/main.rs#L163-L185))
- **Imports**: Group by std, external crates, internal modules
- **Naming**: CLI args use kebab-case (`--nats-server`), code uses snake_case

## Key Files for Context

**Quick orientation:**
- [GETTING_STARTED.md](../GETTING_STARTED.md) - Setup walkthrough
- [IMPLEMENTATION_SUMMARY.md](../IMPLEMENTATION_SUMMARY.md) - Recent changes and decisions

**Format specifications:**
- [DEBEZIUM_FORMAT.md](../DEBEZIUM_FORMAT.md) - CDC envelope structure
- [FELDERA_FORMAT.md](../FELDERA_FORMAT.md) - InsertDelete format rationale

**Examples:**
- [examples/setup.sql](../examples/setup.sql) - Test database schema
- [examples/test_changes.sql](../examples/test_changes.sql) - Sample operations
