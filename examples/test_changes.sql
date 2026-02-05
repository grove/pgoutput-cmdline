-- Test SQL statements to generate replication events
-- Run this while pgoutput-stream is running to see changes

-- Connect to the test database
\c replication_test

-- INSERT events
INSERT INTO users (name, email) VALUES ('David', 'david@example.com');
INSERT INTO users (name, email) VALUES ('Eve', 'eve@example.com');

-- UPDATE events
UPDATE users SET email = 'alice.smith@example.com' WHERE name = 'Alice';



-- More INSERT events
INSERT INTO orders (user_id, product, amount) VALUES (1, 'Monitor', 299.99);
INSERT INTO orders (user_id, product, amount) VALUES (3, 'USB Cable', 9.99);

UPDATE orders SET amount = 279.99 WHERE product = 'Monitor';

-- DELETE events
DELETE FROM orders WHERE amount < 30.00;
DELETE FROM users WHERE name = 'Eve';

-- Multiple changes in a transaction
BEGIN;
INSERT INTO users (name, email) VALUES ('Frank', 'frank@example.com');
INSERT INTO orders (user_id, product, amount) VALUES 
    ((SELECT id FROM users WHERE name = 'Frank'), 'Headphones', 149.99);
UPDATE users SET email = 'frank.updated@example.com' WHERE name = 'Frank';
COMMIT;
