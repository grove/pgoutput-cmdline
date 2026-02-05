# Prerequisites

This guide covers the PostgreSQL configuration requirements for using pgoutput-stream.

## PostgreSQL Configuration

### 1. Enable Logical Replication

Edit your `postgresql.conf` file to enable logical replication:

```conf
wal_level = logical
max_replication_slots = 10
max_wal_senders = 10
```

**Location of postgresql.conf:**
- Linux: `/etc/postgresql/{version}/main/postgresql.conf` or `/var/lib/pgsql/data/postgresql.conf`
- macOS (Homebrew): `/opt/homebrew/var/postgresql@{version}/postgresql.conf`
- macOS (Postgres.app): `~/Library/Application Support/Postgres/var-{version}/postgresql.conf`

### 2. Configure Authentication

Edit your `pg_hba.conf` file to allow replication connections:

```conf
# TYPE  DATABASE        USER            ADDRESS                 METHOD
# Allow replication connections from localhost
host    replication     all             127.0.0.1/32            md5
host    replication     all             ::1/128                 md5
```

**Location of pg_hba.conf:**
- Same directory as `postgresql.conf` (see above)

### 3. Restart PostgreSQL

Apply the configuration changes by restarting PostgreSQL:

```bash
# Linux (systemd)
sudo systemctl restart postgresql

# Linux (older systems)
sudo service postgresql restart

# macOS (Homebrew)
brew services restart postgresql@{version}

# macOS (Postgres.app)
# Stop and start from the GUI

# Docker
docker restart <container_name>
```

Verify PostgreSQL is running:
```bash
pg_isready
```

### 4. Grant Replication Privileges

Ensure your PostgreSQL user has replication privileges:

```sql
-- Connect to PostgreSQL as superuser
psql -U postgres

-- Grant replication privilege to your user
ALTER USER postgres WITH REPLICATION;

-- Or create a dedicated replication user
CREATE USER replicator WITH REPLICATION PASSWORD 'secret';
GRANT SELECT ON ALL TABLES IN SCHEMA public TO replicator;
```

### 5. Create a Publication

A publication defines which tables to replicate:

```sql
-- Connect to your database
psql -U postgres -d mydb

-- Create a publication for specific tables
CREATE PUBLICATION my_publication FOR TABLE users, orders;

-- Or create a publication for all tables
CREATE PUBLICATION my_publication FOR ALL TABLES;

-- Verify the publication was created
\dRp+
```

**Publication Commands:**
```sql
-- List all publications
SELECT * FROM pg_publication;

-- View tables in a publication
SELECT * FROM pg_publication_tables WHERE pubname = 'my_publication';

-- Add tables to existing publication
ALTER PUBLICATION my_publication ADD TABLE products;

-- Remove tables from publication
ALTER PUBLICATION my_publication DROP TABLE products;

-- Drop a publication
DROP PUBLICATION my_publication;
```

### 6. Set REPLICA IDENTITY (Recommended)

REPLICA IDENTITY controls which column values are included in UPDATE and DELETE events.

```sql
-- Set REPLICA IDENTITY FULL for complete old values
ALTER TABLE users REPLICA IDENTITY FULL;
ALTER TABLE orders REPLICA IDENTITY FULL;
```

**REPLICA IDENTITY Options:**

| Mode | Description | UPDATE old tuple | DELETE tuple | Use Case |
|------|-------------|------------------|--------------|----------|
| `DEFAULT` | Uses primary key columns only | PK only | PK only | Minimal overhead, suitable if you only need to identify which row changed |
| `FULL` | Includes all column values | All columns | All columns | **Recommended** - See what changed, required for UPDATE decomposition |
| `NOTHING` | No old values | None | None | Not recommended for CDC |
| `INDEX` | Uses specified unique index | Index columns | Index columns | Compromise between DEFAULT and FULL |

**Why REPLICA IDENTITY FULL?**

- **See what changed**: Compare before/after values in UPDATE operations
- **Debezium format**: Populate the `before` field properly
- **Feldera format**: Generate correct delete + insert pairs for updates
- **Downstream processing**: Enable filtering, auditing, and conditional logic based on old values

**Performance Consideration:**
- `FULL` mode writes more data to WAL (Write-Ahead Log)
- For high-traffic tables, evaluate the trade-off between completeness and overhead
- For most use cases, the benefits outweigh the small performance cost

**Set REPLICA IDENTITY for specific index:**
```sql
-- Create a unique index
CREATE UNIQUE INDEX users_email_idx ON users(email);

-- Use the index for replica identity
ALTER TABLE users REPLICA IDENTITY USING INDEX users_email_idx;
```

## Verification

### Check Configuration

```sql
-- Check wal_level
SHOW wal_level;  -- Should return 'logical'

-- Check replication slots capacity
SHOW max_replication_slots;  -- Should be > 0

-- Check WAL senders capacity
SHOW max_wal_senders;  -- Should be > 0

-- List existing replication slots
SELECT * FROM pg_replication_slots;

-- Check publications
\dRp+

-- Check REPLICA IDENTITY for a table
SELECT relname, relreplident
FROM pg_class
WHERE relname = 'users';
-- relreplident: 'd' = default, 'f' = full, 'i' = index, 'n' = nothing
```

### Test Connection

```bash
# Test regular connection
psql -h localhost -U postgres -d mydb -c "SELECT version();"

# Test replication connection
psql -h localhost -U postgres -d mydb replication=database -c "IDENTIFY_SYSTEM;"
```

## Troubleshooting

### "wal_level is not set to logical"

**Problem:** PostgreSQL is not configured for logical replication.

**Solution:**
1. Edit `postgresql.conf` and set `wal_level = logical`
2. Restart PostgreSQL
3. Verify: `psql -U postgres -c "SHOW wal_level;"`

### "permission denied for replication"

**Problem:** User doesn't have replication privileges.

**Solution:**
```sql
-- Grant replication to existing user
ALTER USER your_user WITH REPLICATION;

-- Or create new replication user
CREATE USER replicator WITH REPLICATION PASSWORD 'secret';
```

### "no pg_hba.conf entry for replication connection"

**Problem:** PostgreSQL doesn't allow replication connections from your host.

**Solution:**
1. Edit `pg_hba.conf` and add:
   ```conf
   host    replication     all             127.0.0.1/32            md5
   ```
2. Reload configuration:
   ```sql
   SELECT pg_reload_conf();
   ```
   Or restart PostgreSQL.

### "publication does not exist"

**Problem:** The specified publication wasn't created.

**Solution:**
```sql
-- Create the publication
CREATE PUBLICATION my_publication FOR ALL TABLES;

-- Or for specific tables
CREATE PUBLICATION my_publication FOR TABLE users, orders;
```

### "too many replication slots"

**Problem:** Reached the limit of `max_replication_slots`.

**Solution:**
1. Drop unused slots:
   ```sql
   SELECT pg_drop_replication_slot('unused_slot');
   ```
2. Or increase the limit in `postgresql.conf`:
   ```conf
   max_replication_slots = 20
   ```
3. Restart PostgreSQL

## Next Steps

Once PostgreSQL is properly configured, proceed to:
- [GETTING_STARTED.md](GETTING_STARTED.md) - Quick start guide
- [USAGE.md](USAGE.md) - Detailed usage examples
- [README.md](README.md) - Overview and installation

## See Also

- [PostgreSQL Logical Replication Documentation](https://www.postgresql.org/docs/current/logical-replication.html)
- [PostgreSQL Replication Slots](https://www.postgresql.org/docs/current/logicaldecoding-explanation.html)
- [REPLICA IDENTITY Documentation](https://www.postgresql.org/docs/current/sql-altertable.html#SQL-ALTERTABLE-REPLICA-IDENTITY)
