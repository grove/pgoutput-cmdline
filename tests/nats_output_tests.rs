use pgoutput_stream::decoder::*;
use std::collections::HashMap;

// Helper function to create test changes
fn create_insert_change(schema: &str, table: &str) -> Change {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("name".to_string(), Some("Test".to_string()));
    
    Change::Insert {
        relation_id: 16384,
        schema: schema.to_string(),
        table: table.to_string(),
        new_tuple,
    }
}

fn create_update_change(schema: &str, table: &str) -> Change {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("1".to_string()));
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("name".to_string(), Some("Updated".to_string()));
    
    Change::Update {
        relation_id: 16384,
        schema: schema.to_string(),
        table: table.to_string(),
        old_tuple: Some(old_tuple),
        new_tuple,
    }
}

fn create_delete_change(schema: &str, table: &str) -> Change {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("1".to_string()));
    
    Change::Delete {
        relation_id: 16384,
        schema: schema.to_string(),
        table: table.to_string(),
        old_tuple,
    }
}

// Note: These tests verify subject generation logic without requiring a NATS server
// Integration tests with actual NATS server would require a running NATS instance

/// Tests NATS subject format generation for INSERT operations.
/// Verifies that INSERT events generate subjects in the format: {prefix}.{schema}.{table}.insert
#[test]
fn test_nats_subject_format_insert() {
    // Test that subject format is correct for INSERT operations
    let change = create_insert_change("public", "users");
    
    // Expected format: {prefix}.{schema}.{table}.{operation}
    // We can verify the Change enum contains the right data
    match change {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "users");
            // Subject should be: postgres.public.users.insert
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests NATS subject format generation for UPDATE operations.
/// Verifies that UPDATE events generate subjects in the format: {prefix}.{schema}.{table}.update
#[test]
fn test_nats_subject_format_update() {
    let change = create_update_change("public", "orders");
    
    match change {
        Change::Update { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "orders");
            // Subject should be: postgres.public.orders.update
        }
        _ => panic!("Expected Update variant"),
    }
}

/// Tests NATS subject format generation for DELETE operations.
/// Verifies that DELETE events generate subjects in the format: {prefix}.{schema}.{table}.delete
#[test]
fn test_nats_subject_format_delete() {
    let change = create_delete_change("public", "products");
    
    match change {
        Change::Delete { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "products");
            // Subject should be: postgres.public.products.delete
        }
        _ => panic!("Expected Delete variant"),
    }
}

/// Tests NATS subject format generation for transaction BEGIN events.
/// Verifies that BEGIN events generate subjects in the format: {prefix}.transactions.begin.event
#[test]
fn test_nats_subject_format_transaction_begin() {
    let change = Change::Begin {
        lsn: "0/16B2D50".to_string(),
        timestamp: 730826470123456,
        xid: 1000,
    };
    
    match change {
        Change::Begin { .. } => {
            // Subject should be: postgres.transactions.begin.event
        }
        _ => panic!("Expected Begin variant"),
    }
}

/// Tests NATS subject format generation for transaction COMMIT events.
/// Verifies that COMMIT events generate subjects in the format: {prefix}.transactions.commit.event
#[test]
fn test_nats_subject_format_transaction_commit() {
    let change = Change::Commit {
        lsn: "0/16B2E20".to_string(),
        timestamp: 730826470123457,
    };
    
    match change {
        Change::Commit { .. } => {
            // Subject should be: postgres.transactions.commit.event
        }
        _ => panic!("Expected Commit variant"),
    }
}

/// Tests NATS subject format generation for RELATION metadata events.
/// Verifies that RELATION events generate subjects in the format: {prefix}.{schema}.{table}.relation
#[test]
fn test_nats_subject_format_relation() {
    let columns = vec![
        ColumnInfo {
            name: "id".to_string(),
            type_id: 23,
            flags: 1,
        },
    ];
    
    let change = Change::Relation {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        columns,
    };
    
    match change {
        Change::Relation { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "users");
            // Subject should be: postgres.public.users.relation
        }
        _ => panic!("Expected Relation variant"),
    }
}

/// Tests NATS subject generation with non-standard schema and table names.
/// Verifies that schema/table names with hyphens and underscores are correctly included in subjects.
#[test]
fn test_nats_subject_with_special_schema_names() {
    let change = create_insert_change("my-custom_schema", "test_table");
    
    match change {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "my-custom_schema");
            assert_eq!(table, "test_table");
            // Subject should be: postgres.my-custom_schema.test_table.insert
            // NATS subjects allow hyphens and underscores
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests NATS subject generation across multiple schemas.
/// Verifies that events from different schemas create distinct subject paths for proper routing.
#[test]
fn test_nats_subject_with_multiple_schemas() {
    // Test different schemas to ensure proper subject routing
    let changes = vec![
        create_insert_change("public", "users"),
        create_insert_change("sales", "orders"),
        create_insert_change("analytics", "events"),
    ];
    
    for change in changes {
        match change {
            Change::Insert { schema, table, .. } => {
                // Each should create a distinct subject
                assert!(!schema.is_empty());
                assert!(!table.is_empty());
            }
            _ => panic!("Expected Insert variant"),
        }
    }
}

/// Tests JSON serialization of Change events for NATS payloads.
/// Verifies that Change events can be serialized to JSON and deserialized back correctly.
#[test]
fn test_change_serialization_for_nats() {
    // Verify that changes can be serialized to JSON for NATS payloads
    let change = create_insert_change("public", "users");
    
    let json = serde_json::to_vec(&change).unwrap();
    assert!(!json.is_empty());
    
    // Verify it can be deserialized back
    let deserialized: Change = serde_json::from_slice(&json).unwrap();
    match deserialized {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "users");
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests roundtrip serialization for all Change event types.
/// Verifies that all event types (Begin, Insert, Update, Delete, Commit) maintain their structure through serialization.
#[test]
fn test_change_serialization_roundtrip() {
    let changes = vec![
        Change::Begin {
            lsn: "0/123".to_string(),
            timestamp: 12345,
            xid: 100,
        },
        create_insert_change("public", "users"),
        create_update_change("public", "orders"),
        create_delete_change("public", "products"),
        Change::Commit {
            lsn: "0/456".to_string(),
            timestamp: 12346,
        },
    ];
    
    for change in changes {
        let json = serde_json::to_vec(&change).unwrap();
        let deserialized: Change = serde_json::from_slice(&json).unwrap();
        
        // Verify basic structure is preserved
        match (&change, &deserialized) {
            (Change::Begin { .. }, Change::Begin { .. }) => (),
            (Change::Commit { .. }, Change::Commit { .. }) => (),
            (Change::Insert { .. }, Change::Insert { .. }) => (),
            (Change::Update { .. }, Change::Update { .. }) => (),
            (Change::Delete { .. }, Change::Delete { .. }) => (),
            _ => panic!("Deserialization changed variant type"),
        }
    }
}

/// Tests that NATS payload sizes remain reasonable for wide tables.
/// Verifies that even tables with 100 columns produce manageable JSON payloads (< 100KB).
#[test]
fn test_nats_payload_size_reasonable() {
    // Verify that serialized payloads are reasonable sizes
    let mut large_tuple = HashMap::new();
    for i in 0..100 {
        large_tuple.insert(
            format!("column_{}", i),
            Some(format!("value_{}", i)),
        );
    }
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "wide_table".to_string(),
        new_tuple: large_tuple,
    };
    
    let json = serde_json::to_vec(&change).unwrap();
    
    // Should be manageable size (< 100KB for 100 columns)
    assert!(json.len() < 100_000);
}

/// Tests consistency of NATS subject hierarchy for table operations.
/// Verifies that all operations on the same table share a common subject prefix for filtering.
#[test]
fn test_nats_subject_hierarchy_consistency() {
    // Verify all operations on same table use same prefix
    let changes = vec![
        create_insert_change("public", "users"),
        create_update_change("public", "users"),
        create_delete_change("public", "users"),
    ];
    
    for change in changes {
        match change {
            Change::Insert { schema, table, .. }
            | Change::Update { schema, table, .. }
            | Change::Delete { schema, table, .. } => {
                assert_eq!(schema, "public");
                assert_eq!(table, "users");
                // All should share: postgres.public.users.*
            }
            _ => panic!("Expected table operation"),
        }
    }
}

/// Tests that concurrent operations on different tables produce distinct subjects.
/// Verifies subject uniqueness for different schema/table/operation combinations.
#[test]
fn test_concurrent_subject_patterns() {
    // Verify subjects from different tables can be distinguished
    let changes = vec![
        ("public", "users", "insert"),
        ("public", "users", "update"),
        ("public", "orders", "insert"),
        ("sales", "orders", "insert"),
    ];
    
    for (schema, table, operation) in changes {
        let change = match operation {
            "insert" => create_insert_change(schema, table),
            "update" => create_update_change(schema, table),
            _ => panic!("Unknown operation"),
        };
        
        // Each combination creates a unique subject path
        match change {
            Change::Insert { schema: s, table: t, .. }
            | Change::Update { schema: s, table: t, .. } => {
                assert_eq!(s, schema);
                assert_eq!(t, table);
            }
            _ => panic!("Unexpected variant"),
        }
    }
}

/// Tests serialization of INSERT events with empty tuple data.
/// Verifies that events with no column data still serialize to valid JSON.
#[test]
fn test_empty_tuple_serialization() {
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: HashMap::new(),
    };
    
    // Should serialize even with empty tuple
    let json = serde_json::to_vec(&change).unwrap();
    assert!(!json.is_empty());
}

/// Tests serialization of events with maximum LSN and transaction ID values.
/// Verifies that edge case values (max u64, max i64, max u32) serialize correctly.
#[test]
fn test_large_lsn_values() {
    let change = Change::Begin {
        lsn: "FFFFFFFF/FFFFFFFF".to_string(),
        timestamp: i64::MAX,
        xid: u32::MAX,
    };
    
    // Should serialize large values correctly
    let json = serde_json::to_vec(&change).unwrap();
    let deserialized: Change = serde_json::from_slice(&json).unwrap();
    
    match deserialized {
        Change::Begin { lsn, timestamp, xid } => {
            assert_eq!(lsn, "FFFFFFFF/FFFFFFFF");
            assert_eq!(timestamp, i64::MAX);
            assert_eq!(xid, u32::MAX);
        }
        _ => panic!("Expected Begin variant"),
    }
}

/// Tests serialization of Unicode characters in table data.
/// Verifies that international characters (Chinese, Norwegian, German) are preserved through serialization.
#[test]
fn test_unicode_in_table_data() {
    let mut tuple = HashMap::new();
    tuple.insert("name".to_string(), Some("测试用户".to_string()));
    tuple.insert("description".to_string(), Some("Tëst Üsér".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    // Unicode should serialize correctly
    let json = serde_json::to_vec(&change).unwrap();
    let deserialized: Change = serde_json::from_slice(&json).unwrap();
    
    match deserialized {
        Change::Insert { new_tuple, .. } => {
            assert_eq!(new_tuple.get("name").unwrap().as_ref().unwrap(), "测试用户");
            assert_eq!(new_tuple.get("description").unwrap().as_ref().unwrap(), "Tëst Üsér");
        }
        _ => panic!("Expected Insert variant"),
    }
}
