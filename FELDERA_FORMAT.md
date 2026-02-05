# Feldera Format Guide

## Overview

The Feldera InsertDelete format outputs Change Data Capture (CDC) events in a structure designed for streaming SQL pipelines and incremental view maintenance. Each event explicitly wraps the record with an operation type.

## Key Characteristics

- **Explicit Operations**: Records are wrapped with `{"insert": {...}}` or `{"delete": {...}}`
- **Update Decomposition**: UPDATE operations are encoded as delete (old) + insert (new) pairs
- **Data Events Only**: Only outputs INSERT, UPDATE, and DELETE operations
- **Filtered Events**: Transaction markers (BEGIN/COMMIT) and schema events (RELATION) are not included
- **Streaming Compatible**: Optimized for streaming systems and incremental computation

## Event Structure

### Insert Event

```json
{"insert": {"column1": "value1", "column2": "value2", ...}}
```

- Single event with `insert` key
- Contains the full record to be added

### Update Event (produces 2 events)

```json
{"delete": {"column1": "old_value1", "column2": "old_value2", ...}}
{"insert": {"column1": "new_value1", "column2": "new_value2", ...}}
```

- First event: `delete` with old record state
- Second event: `insert` with new record state
- This decomposition supports incremental view maintenance

### Delete Event

```json
{"delete": {"column1": "value1", "column2": "value2", ...}}
```

- Single event with `delete` key
- Contains the record to be removed

## Why Updates = Delete + Insert?

This pattern follows the principles of **incremental view maintenance** used in streaming systems:

1. **Correctness in Aggregations**: When computing COUNT, SUM, AVG, etc., an update must:
   - Remove the contribution of the old value (delete)
   - Add the contribution of the new value (insert)

2. **Joins and Group By**: Stream joins and grouping operations need to:
   - Retract the old state from the view
   - Add the new state to the view

3. **Deterministic Results**: Ensures that streaming queries produce the same results as batch queries

### Example: Counting Orders by Status

Consider this streaming query:
```sql
SELECT status, COUNT(*) FROM orders GROUP BY status
```

When an order updates from 'pending' → 'completed':
- **Delete**: `{"status": "pending", "order_id": 123}` → decrement pending count
- **Insert**: `{"status": "completed", "order_id": 123}` → increment completed count

This ensures the counts remain accurate in real-time.

## Usage Examples

### Basic Usage

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot feldera_slot \
  --publication my_publication \
  --format feldera
```

### With NATS JetStream

```bash
pgoutput-stream \
  --connection "host=localhost user=postgres dbname=mydb" \
  --slot feldera_slot \
  --publication my_publication \
  --format feldera \
  --nats-server "nats://localhost:4222"
```

### Filtering to Specific Tables

Use PostgreSQL publications to control which tables are captured:

```sql
-- Create publication for specific tables
CREATE PUBLICATION feldera_pub FOR TABLE orders, customers, products;

-- Run tool with this publication
pgoutput-stream --publication feldera_pub --format feldera ...
```

## Sample Output

### Scenario: Order Lifecycle

#### 1. New Order (INSERT)

```json
{"insert":{"order_id":"1001","customer":"Alice","amount":"150.00","status":"pending"}}
```

#### 2. Order Status Update (UPDATE → delete + insert)

```json
{"delete":{"order_id":"1001","customer":"Alice","amount":"150.00","status":"pending"}}
{"insert":{"order_id":"1001","customer":"Alice","amount":"150.00","status":"completed"}}
```

#### 3. Order Cancellation (DELETE)

```json
{"delete":{"order_id":"1001","customer":"Alice","amount":"150.00","status":"completed"}}
```

## Best Practices

### 1. Set REPLICA IDENTITY FULL

To capture complete old values in UPDATE and DELETE operations:

```sql
ALTER TABLE orders REPLICA IDENTITY FULL;
ALTER TABLE customers REPLICA IDENTITY FULL;
```

Without this, only the primary key will be available in DELETE events.

### 2. Handle Update Pairs Correctly

When consuming Feldera format, remember that updates come as **two consecutive events**:

```python
# Python example - processing Feldera events
for line in stream:
    event = json.loads(line)
    
    if 'insert' in event:
        # Add record to view
        view.add(event['insert'])
    elif 'delete' in event:
        # Remove record from view
        view.remove(event['delete'])
```

### 3. Maintain Event Order

The delete event **always comes before** the insert event in an update. Maintain this order for correctness.

### 4. Use Publications for Filtering

Instead of filtering events in your consumer, use PostgreSQL publications:

```sql
-- Only capture order-related tables
CREATE PUBLICATION order_changes FOR TABLE orders, order_items, order_status_log;
```

### 5. Handle NULL Values

NULL values are preserved in the JSON output:

```json
{"insert":{"order_id":"1002","customer":"Bob","discount":null,"status":"pending"}}
```

Your consumer should handle NULL appropriately based on your business logic.

## Integration Patterns

### With Feldera Streaming SQL

The Feldera format is designed for direct integration with Feldera pipelines:

```sql
-- Feldera SQL pipeline
CREATE TABLE orders (
    order_id BIGINT PRIMARY KEY,
    customer VARCHAR,
    amount DECIMAL,
    status VARCHAR
);

-- Create real-time view
CREATE VIEW order_summary AS
SELECT status, COUNT(*) as count, SUM(amount) as total
FROM orders
GROUP BY status;
```

Pipe the output:
```bash
pgoutput-stream --format feldera ... | feldera-ingest --table orders
```

### With Stream Processing

Process the event stream with any tool that handles JSON:

```bash
# Filter only inserts
pgoutput-stream --format feldera | jq 'select(.insert != null)'

# Extract specific fields
pgoutput-stream --format feldera | jq '.insert.customer // .delete.customer'

# Count events by type
pgoutput-stream --format feldera | jq -r 'keys[0]' | sort | uniq -c
```

### With Custom Consumers

```javascript
// Node.js example
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });

const view = new Map();

rl.on('line', (line) => {
  const event = JSON.parse(line);
  
  if (event.insert) {
    // Add or update in view
    view.set(event.insert.order_id, event.insert);
    console.log(`Added order ${event.insert.order_id}`);
  } else if (event.delete) {
    // Remove from view
    view.delete(event.delete.order_id);
    console.log(`Removed order ${event.delete.order_id}`);
  }
});
```

## Differences from Other Formats

### vs. Debezium

| Feature | Feldera | Debezium |
|---------|---------|----------|
| Update encoding | delete + insert | Single event with before/after |
| Envelope | Minimal (`insert`/`delete`) | Rich (op, source, ts_ms) |
| Metadata | None | Extensive (connector, db, schema, table, LSN, timestamp) |
| Use case | Streaming SQL, view maintenance | General CDC, data integration |

### vs. Raw JSON

| Feature | Feldera | Raw JSON |
|---------|---------|----------|
| Operation type | Explicit wrapper | Implicit (Change enum) |
| Updates | Two events | Single Update variant |
| Streaming systems | Optimized | Requires parsing |

## Event Ordering

Events maintain the order of PostgreSQL's logical replication stream:

1. Events from the same transaction are ordered
2. UPDATE always produces delete before insert
3. Cross-transaction ordering follows commit order

## Testing

Test files are available in the `examples/` directory:

- `test_feldera.sql` - SQL script to generate various Feldera events

Run the test:

```bash
# Terminal 1: Start the tool
pgoutput-stream --connection "..." --slot test_slot --publication my_pub --format feldera

# Terminal 2: Execute test operations
psql -U postgres -d mydb -f examples/test_feldera.sql
```

Expected output for an UPDATE:
```json
{"delete":{"order_id":"1","customer":"Alice","amount":"150.00","status":"pending"}}
{"insert":{"order_id":"1","customer":"Alice","amount":"155.00","status":"completed"}}
```

## Troubleshooting

### No Events Appearing

1. Verify the publication includes your tables:
   ```sql
   SELECT * FROM pg_publication_tables WHERE pubname = 'my_publication';
   ```

2. Check that operations are actually happening:
   ```sql
   SELECT * FROM pg_stat_replication;
   ```

3. Ensure REPLICA IDENTITY is set (for UPDATE/DELETE):
   ```sql
   SELECT relname, relreplident FROM pg_class WHERE relname = 'orders';
   ```

### Missing Old Values in Updates

If the delete event in updates shows incomplete data:

```sql
-- Set REPLICA IDENTITY FULL
ALTER TABLE your_table REPLICA IDENTITY FULL;
```

### Updates Not Producing Two Events

Verify you're using the Feldera format:
```bash
pgoutput-stream ... --format feldera  # Correct
pgoutput-stream ... --format json     # Wrong - shows Update variant
```

### JSON Parsing Errors

The output is newline-delimited JSON (one event per line):

```bash
# Good: Process line by line
while IFS= read -r line; do
  echo "$line" | jq '.insert // .delete'
done

# Bad: Try to parse entire stream as array
cat stream.json | jq '.[]'  # This will fail
```

## Performance Considerations

### Event Volume

Updates produce **twice the events** compared to formats that use a single update event:
- 1 INSERT → 1 event
- 1 UPDATE → 2 events (delete + insert)
- 1 DELETE → 1 event

For high-update workloads, expect ~2x event volume compared to Debezium.

### Network Bandwidth

Each update sends:
- Full old record (delete)
- Full new record (insert)

For large records, consider:
- Using `REPLICA IDENTITY DEFAULT` if old values aren't needed
- Filtering columns at the publication level
- Compressing the stream

### Consumer Design

Design consumers to handle update pairs atomically:

```python
# Buffer approach for atomic updates
buffer = None

for event in stream:
    if 'delete' in event:
        buffer = event['delete']
    elif 'insert' in event and buffer:
        # Process update: buffer = old, event = new
        process_update(old=buffer, new=event['insert'])
        buffer = None
    elif 'insert' in event:
        # Pure insert
        process_insert(event['insert'])
```

## Further Reading

- [Feldera Documentation](https://docs.feldera.com/)
- [Incremental View Maintenance](https://en.wikipedia.org/wiki/Incremental_view_maintenance)
- [PostgreSQL Logical Replication](https://www.postgresql.org/docs/current/logical-replication.html)
- [Stream Processing Concepts](https://www.oreilly.com/library/view/streaming-systems/9781491983867/)
