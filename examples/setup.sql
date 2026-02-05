-- Example PostgreSQL setup for testing pgoutput-stream
-- Run this with: psql -U postgres -f examples/setup.sql

-- Create test database
CREATE DATABASE replication_test;

-- Connect to the test database
\c replication_test

-- Create test tables
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(100),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id),
    product VARCHAR(100),
    amount DECIMAL(10, 2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Set REPLICA IDENTITY to FULL to include all columns in UPDATE old_tuple
-- DEFAULT: Only primary key columns are included in old_tuple
-- FULL: All columns are included in old_tuple (required for old values in UPDATEs)
ALTER TABLE users REPLICA IDENTITY FULL;
ALTER TABLE orders REPLICA IDENTITY FULL;

-- Create a publication for logical replication
CREATE PUBLICATION test_publication FOR ALL TABLES;

-- Alternative: Create publication for specific tables
-- CREATE PUBLICATION test_publication FOR TABLE users, orders;

-- Verify publication was created
SELECT * FROM pg_publication;

-- Insert some test data
INSERT INTO users (name, email) VALUES 
    ('Alice', 'alice@example.com'),
    ('Bob', 'bob@example.com'),
    ('Charlie', 'charlie@example.com');

INSERT INTO orders (user_id, product, amount) VALUES
    (1, 'Laptop', 999.99),
    (2, 'Mouse', 29.99),
    (1, 'Keyboard', 79.99);

-- Check that replication slot can be created (optional, the tool will do this)
-- SELECT pg_create_logical_replication_slot('test_slot', 'pgoutput');

-- To view existing replication slots:
SELECT * FROM pg_replication_slots;

-- To drop a replication slot if needed:
-- SELECT pg_drop_replication_slot('test_slot');
