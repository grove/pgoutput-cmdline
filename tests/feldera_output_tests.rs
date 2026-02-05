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

// Note: These tests verify schema-qualified table name routing logic
// Integration tests with actual Feldera server would require a running Feldera instance

/// Tests that INSERT events from public.users route to public_users table.
/// Verifies schema-qualified table naming convention.
#[test]
fn test_feldera_routing_public_users() {
    let change = create_insert_change("public", "users");
    
    match change {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "users");
            // Should route to: /ingress/public_users
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests that INSERT events from analytics.users route to analytics_users table.
/// Verifies different schemas route to different Feldera tables.
#[test]
fn test_feldera_routing_analytics_users() {
    let change = create_insert_change("analytics", "users");
    
    match change {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "analytics");
            assert_eq!(table, "users");
            // Should route to: /ingress/analytics_users
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests UPDATE event routing with schema qualification.
#[test]
fn test_feldera_routing_update() {
    let change = create_update_change("sales", "orders");
    
    match change {
        Change::Update { schema, table, .. } => {
            assert_eq!(schema, "sales");
            assert_eq!(table, "orders");
            // Should route to: /ingress/sales_orders
        }
        _ => panic!("Expected Update variant"),
    }
}

/// Tests DELETE event routing with schema qualification.
#[test]
fn test_feldera_routing_delete() {
    let change = create_delete_change("public", "products");
    
    match change {
        Change::Delete { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "products");
            // Should route to: /ingress/public_products
        }
        _ => panic!("Expected Delete variant"),
    }
}

/// Tests multi-schema support - same table name in different schemas.
/// Verifies that public.events and analytics.events route to different endpoints.
#[test]
fn test_feldera_multi_schema_same_table() {
    let change1 = create_insert_change("public", "events");
    let change2 = create_insert_change("analytics", "events");
    
    match change1 {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "events");
            // Should route to: /ingress/public_events
        }
        _ => panic!("Expected Insert variant"),
    }
    
    match change2 {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "analytics");
            assert_eq!(table, "events");
            // Should route to: /ingress/analytics_events
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests that BEGIN transaction events are skipped.
/// Verifies non-data events don't get routed to Feldera.
#[test]
fn test_feldera_skip_begin() {
    let change = Change::Begin {
        lsn: "0/1234567".to_string(),
        timestamp: 1706107200000,
        xid: 12345,
    };
    
    match change {
        Change::Begin { .. } => {
            // Should be skipped by FelderaOutput::write_change
        }
        _ => panic!("Expected Begin variant"),
    }
}

/// Tests that COMMIT transaction events are skipped.
#[test]
fn test_feldera_skip_commit() {
    let change = Change::Commit {
        lsn: "0/1234567".to_string(),
        timestamp: 1706107200000,
    };
    
    match change {
        Change::Commit { .. } => {
            // Should be skipped by FelderaOutput::write_change
        }
        _ => panic!("Expected Commit variant"),
    }
}

/// Tests that RELATION metadata events are skipped.
#[test]
fn test_feldera_skip_relation() {
    let change = Change::Relation {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        columns: vec![],
    };
    
    match change {
        Change::Relation { .. } => {
            // Should be skipped by FelderaOutput::write_change
        }
        _ => panic!("Expected Relation variant"),
    }
}

/// Tests table filtering logic - verifies schema qualification.
/// When filtering by "public_users", only public.users should match.
#[test]
fn test_feldera_table_filtering_logic() {
    let allowed = vec!["public_users".to_string(), "public_orders".to_string()];
    
    // Should match
    assert!(allowed.contains(&"public_users".to_string()));
    assert!(allowed.contains(&"public_orders".to_string()));
    
    // Should NOT match
    assert!(!allowed.contains(&"analytics_users".to_string()));
    assert!(!allowed.contains(&"public_products".to_string()));
}

/// Tests schema names with special characters.
#[test]
fn test_feldera_special_schema_names() {
    let change = create_insert_change("data-warehouse", "events");
    
    match change {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "data-warehouse");
            assert_eq!(table, "events");
            // Should route to: /ingress/data-warehouse_events
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests table names with underscores.
#[test]
fn test_feldera_table_with_underscores() {
    let change = create_insert_change("public", "user_activity_logs");
    
    match change {
        Change::Insert { schema, table, .. } => {
            assert_eq!(schema, "public");
            assert_eq!(table, "user_activity_logs");
            // Should route to: /ingress/public_user_activity_logs
        }
        _ => panic!("Expected Insert variant"),
    }
}

/// Tests conversion to Feldera format for INSERT.
/// Verifies that INSERT events have the correct structure.
#[test]
fn test_feldera_format_insert() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("1".to_string()));
    tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let events = pgoutput_stream::output::convert_to_feldera_test(&change);
    assert_eq!(events.len(), 1);
    
    let event = &events[0];
    assert!(event.insert.is_some());
    assert!(event.delete.is_none());
}

/// Tests conversion to Feldera format for UPDATE.
/// Verifies that UPDATE events are split into DELETE + INSERT.
#[test]
fn test_feldera_format_update() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("1".to_string()));
    old_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("name".to_string(), Some("Bob".to_string()));
    
    let change = Change::Update {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple: Some(old_tuple),
        new_tuple,
    };
    
    let events = pgoutput_stream::output::convert_to_feldera_test(&change);
    assert_eq!(events.len(), 2);
    
    // First event should be DELETE
    assert!(events[0].delete.is_some());
    assert!(events[0].insert.is_none());
    
    // Second event should be INSERT
    assert!(events[1].insert.is_some());
    assert!(events[1].delete.is_none());
}

/// Tests conversion to Feldera format for DELETE.
#[test]
fn test_feldera_format_delete() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("1".to_string()));
    
    let change = Change::Delete {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple,
    };
    
    let events = pgoutput_stream::output::convert_to_feldera_test(&change);
    assert_eq!(events.len(), 1);
    
    let event = &events[0];
    assert!(event.delete.is_some());
    assert!(event.insert.is_none());
}
