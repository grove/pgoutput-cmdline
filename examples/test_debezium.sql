-- Test Debezium format output
-- Run this with: psql -U postgres -d mydb -f test_debezium.sql
-- While running: pgoutput-stream --connection "..." --slot my_slot --publication my_pub --format debezium

BEGIN;

-- Create test table if not exists
CREATE TABLE IF NOT EXISTS test_users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100),
    email VARCHAR(100),
    age INTEGER
);

-- Set REPLICA IDENTITY FULL to capture old values
ALTER TABLE test_users REPLICA IDENTITY FULL;

-- Ensure publication includes the table
-- (If your publication is FOR ALL TABLES, this is not needed)
-- ALTER PUBLICATION my_publication ADD TABLE test_users;

COMMIT;

-- Test INSERT - creates a Debezium event with op='c'
INSERT INTO test_users (name, email, age) VALUES ('Alice', 'alice@example.com', 30);

-- Wait a moment between operations
SELECT pg_sleep(1);

-- Test UPDATE - creates a Debezium event with op='u'
UPDATE test_users SET email = 'alice.updated@example.com', age = 31 WHERE name = 'Alice';

-- Wait a moment between operations
SELECT pg_sleep(1);

-- Test INSERT with NULL - verifies NULL handling
INSERT INTO test_users (name, email, age) VALUES ('Bob', NULL, 25);

-- Wait a moment between operations
SELECT pg_sleep(1);

-- Test DELETE - creates a Debezium event with op='d'
DELETE FROM test_users WHERE name = 'Bob';

-- Wait a moment
SELECT pg_sleep(1);

-- Final cleanup (optional)
-- DELETE FROM test_users WHERE name = 'Alice';
