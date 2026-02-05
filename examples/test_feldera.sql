-- Test Feldera InsertDelete format output
-- Run this with: psql -U postgres -d mydb -f test_feldera.sql
-- While running: pgoutput-stream --connection "..." --slot my_slot --publication my_pub --format feldera

BEGIN;

-- Create test table if not exists
CREATE TABLE IF NOT EXISTS test_orders (
    order_id SERIAL PRIMARY KEY,
    customer_name VARCHAR(100),
    total_amount DECIMAL(10, 2),
    status VARCHAR(50)
);

-- Set REPLICA IDENTITY FULL to capture old values in updates
ALTER TABLE test_orders REPLICA IDENTITY FULL;

-- Ensure publication includes the table
-- (If your publication is FOR ALL TABLES, this is not needed)
-- ALTER PUBLICATION my_publication ADD TABLE test_orders;

COMMIT;

-- Test INSERT - produces single insert event: {"insert": {...}}
INSERT INTO test_orders (customer_name, total_amount, status) 
VALUES ('Alice Johnson', 150.00, 'pending');

-- Wait a moment between operations
SELECT pg_sleep(1);

-- Test UPDATE - produces TWO events: {"delete": {...}} followed by {"insert": {...}}
-- This is how Feldera handles updates for incremental view maintenance
UPDATE test_orders 
SET status = 'completed', total_amount = 155.00 
WHERE customer_name = 'Alice Johnson';

-- Wait a moment between operations
SELECT pg_sleep(1);

-- Test another INSERT
INSERT INTO test_orders (customer_name, total_amount, status) 
VALUES ('Bob Smith', 250.00, 'pending');

-- Wait a moment
SELECT pg_sleep(1);

-- Test UPDATE again - will produce delete + insert
UPDATE test_orders 
SET status = 'shipped' 
WHERE customer_name = 'Bob Smith';

-- Wait a moment
SELECT pg_sleep(1);

-- Test INSERT with NULL value
INSERT INTO test_orders (customer_name, total_amount, status) 
VALUES ('Charlie Brown', NULL, 'pending');

-- Wait a moment
SELECT pg_sleep(1);

-- Test DELETE - produces single delete event: {"delete": {...}}
DELETE FROM test_orders WHERE customer_name = 'Charlie Brown';

-- Wait a moment
SELECT pg_sleep(1);

-- Final cleanup (optional)
-- DELETE FROM test_orders WHERE customer_name IN ('Alice Johnson', 'Bob Smith');

-- Query current state
SELECT 'Current orders:' AS info;
SELECT * FROM test_orders ORDER BY order_id;
