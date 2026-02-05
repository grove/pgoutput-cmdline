# Getting Started with pgoutput-stream

## Quick Start

### 1. Ensure PostgreSQL is Configured

Edit your `postgresql.conf`:
```conf
wal_level = logical
max_replication_slots = 10
max_wal_senders = 10
```

Restart PostgreSQL:
```bash
sudo systemctl restart postgresql  # Linux
brew services restart postgresql   # macOS
```

**Note:** For complete setup instructions including authentication, permissions, and troubleshooting, see [PREREQUISITES.md](PREREQUISITES.md).

### 2. Set Up Test Database

```bash
psql -U postgres -f examples/setup.sql
```

This creates:
- Database: `replication_test`
- Tables: `users`, `orders`
- Publication: `test_publication`

### 3. Build and Run

```bash
# Build the project
cargo build --release

# Start streaming changes
./target/release/pgoutput-stream \
  --connection "host=localhost user=postgres dbname=replication_test" \
  --slot test_slot \
  --publication test_publication \
  --create-slot
```

Or use the example script:
```bash
./examples/run.sh
```

### 4. Generate Test Changes

In another terminal:
```bash
psql -U postgres -d replication_test -f examples/test_changes.sql
```

You should see the changes streaming in JSON format in the first terminal!

## Output Examples

### JSON Output (default)
```json
{"Begin":{"lsn":"0/16B2D50","timestamp":730826470123456,"xid":730}}
{"Insert":{"relation_id":16384,"schema":"public","table":"users","new_tuple":{"id":"1","name":"Alice","email":"alice@example.com"}}}
{"Commit":{"lsn":"0/16B2E20","timestamp":730826470123457}}
```

### Text Output
```
BEGIN [LSN: 0/16B2D50, XID: 730, Time: 730826470123456]
INSERT into public.users (ID: 16384)
  New values:
    id: 1
    name: Alice
    email: alice@example.com
COMMIT [LSN: 0/16B2E20, Time: 730826470123457]
```

## Common Issues

### Permission Denied
```bash
# Grant replication permission
psql -U postgres -c "ALTER USER postgres WITH REPLICATION;"
```

### Slot Already Exists
```bash
# Drop existing slot
psql -U postgres -d replication_test -c "SELECT pg_drop_replication_slot('test_slot');"
```

### Connection Refused
- Verify PostgreSQL is running: `pg_isready`
- Check `pg_hba.conf` allows connections
- Verify connection string parameters

## Next Steps

- **Filter specific tables**: Modify the publication to include only certain tables
- **Process changes**: Pipe JSON output to your application
- **Monitor lag**: Use `pg_replication_slots` view to monitor slot lag
- **Checkpoint**: Implement custom LSN tracking for resume capability

## Useful PostgreSQL Commands

```sql
-- View all publications
\dRp+

-- View replication slots
SELECT * FROM pg_replication_slots;

-- View current WAL position
SELECT pg_current_wal_lsn();

-- Monitor slot lag
SELECT slot_name, pg_size_pretty(pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn)) AS lag
FROM pg_replication_slots;
```

## Architecture

```
PostgreSQL → Logical Replication Protocol → pgoutput plugin
                                               ↓
                                    pgoutput-stream decoder
                                               ↓
                                    JSON/Text Formatter
                                               ↓
                                            stdout
```

The tool:
1. Connects to PostgreSQL with replication mode
2. Creates/uses a logical replication slot
3. Subscribes to changes via the publication
4. Decodes pgoutput protocol messages
5. Formats and outputs changes to stdout

## See Also

- **[PREREQUISITES.md](PREREQUISITES.md)** - Complete PostgreSQL setup guide
- **[USAGE.md](USAGE.md)** - Detailed CLI reference and usage examples
- **[README.md](README.md)** - Project overview
- **[DEBEZIUM_FORMAT.md](DEBEZIUM_FORMAT.md)** - Debezium CDC format
- **[FELDERA_FORMAT.md](FELDERA_FORMAT.md)** - Feldera InsertDelete format
- **[NATS_INTEGRATION.md](NATS_INTEGRATION.md)** - NATS JetStream integration
- **[FELDERA_HTTP_CONNECTOR.md](FELDERA_HTTP_CONNECTOR.md)** - Feldera HTTP connector
