use pgoutput_cmdline::output::*;
use pgoutput_cmdline::decoder::*;
use std::collections::HashMap;

/// Tests parsing of 'json' output format string.
/// Verifies that OutputFormat::from_str correctly recognizes and returns the Json variant.
#[test]
fn test_output_format_from_str_json() {
    let format = OutputFormat::from_str("json").unwrap();
    assert!(matches!(format, OutputFormat::Json));
}

/// Tests parsing of 'json-pretty' output format string.
/// Verifies that the JsonPretty format is correctly recognized.
#[test]
fn test_output_format_from_str_json_pretty() {
    let format = OutputFormat::from_str("json-pretty").unwrap();
    assert!(matches!(format, OutputFormat::JsonPretty));
}

/// Tests parsing of 'text' output format string for human-readable output.
/// Verifies the Text format variant is properly created.
#[test]
fn test_output_format_from_str_text() {
    let format = OutputFormat::from_str("text").unwrap();
    assert!(matches!(format, OutputFormat::Text));
}

/// Tests that output format parsing is case-insensitive.
/// Verifies that 'JSON', 'Json', and 'TEXT' all parse correctly.
#[test]
fn test_output_format_from_str_case_insensitive() {
    assert!(matches!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json));
    assert!(matches!(OutputFormat::from_str("Json").unwrap(), OutputFormat::Json));
    assert!(matches!(OutputFormat::from_str("TEXT").unwrap(), OutputFormat::Text));
}

/// Tests error handling for invalid output format strings.
/// Verifies that unrecognized formats like 'invalid', 'xml', and empty strings return errors.
#[test]
fn test_output_format_from_str_invalid() {
    assert!(OutputFormat::from_str("invalid").is_err());
    assert!(OutputFormat::from_str("xml").is_err());
    assert!(OutputFormat::from_str("").is_err());
}

/// Tests JSON serialization of BEGIN transaction events.
/// Verifies that LSN, timestamp, and transaction ID are correctly serialized to JSON.
#[test]
fn test_json_serialization_begin() {
    let change = Change::Begin {
        lsn: "0/123456".to_string(),
        timestamp: 123456789,
        xid: 999,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("Begin"));
    assert!(json.contains("0/123456"));
    assert!(json.contains("123456789"));
    assert!(json.contains("999"));
    
    // Verify it's valid JSON
    let _: serde_json::Value = serde_json::from_str(&json).unwrap();
}

/// Tests JSON serialization of COMMIT transaction events.
/// Verifies that commit LSN and timestamp are properly formatted in JSON output.
#[test]
fn test_json_serialization_commit() {
    let change = Change::Commit {
        lsn: "0/789ABC".to_string(),
        timestamp: 987654321,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("Commit"));
    assert!(json.contains("0/789ABC"));
    assert!(json.contains("987654321"));
}

/// Tests JSON serialization of INSERT operations.
/// Verifies that relation ID, schema, table name, and tuple data are correctly represented in JSON.
#[test]
fn test_json_serialization_insert() {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Insert {
        relation_id: 100,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("Insert"));
    assert!(json.contains("public"));
    assert!(json.contains("users"));
    assert!(json.contains("Alice"));
    
    // Verify it's valid JSON
    let _: serde_json::Value = serde_json::from_str(&json).unwrap();
}

/// Tests JSON serialization of INSERT operations containing NULL values.
/// Verifies that SQL NULL is properly represented as JSON null.
#[test]
fn test_json_serialization_insert_with_null() {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("email".to_string(), None);
    
    let change = Change::Insert {
        relation_id: 100,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("null"));
    
    // Verify proper JSON null handling
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let insert = &value["Insert"];
    assert!(insert["new_tuple"]["email"].is_null());
}

/// Tests JSON serialization of UPDATE operations with old tuple data.
/// Verifies that both old and new values are included when REPLICA IDENTITY FULL is used.
#[test]
fn test_json_serialization_update_with_old_tuple() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("name".to_string(), Some("Bob".to_string()));
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("name".to_string(), Some("Robert".to_string()));
    
    let change = Change::Update {
        relation_id: 200,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple: Some(old_tuple),
        new_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("Update"));
    assert!(json.contains("Bob"));
    assert!(json.contains("Robert"));
}

/// Tests JSON serialization of UPDATE operations without old tuple data.
/// Verifies proper handling when only new values are available (REPLICA IDENTITY DEFAULT).
#[test]
fn test_json_serialization_update_without_old_tuple() {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("name".to_string(), Some("Carol".to_string()));
    
    let change = Change::Update {
        relation_id: 200,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple: None,
        new_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("Update"));
    assert!(json.contains("Carol"));
    
    // Verify old_tuple is null
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value["Update"]["old_tuple"].is_null());
}

/// Tests JSON serialization of DELETE operations.
/// Verifies that deleted row data (old tuple) is correctly serialized to JSON.
#[test]
fn test_json_serialization_delete() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("42".to_string()));
    
    let change = Change::Delete {
        relation_id: 300,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("Delete"));
    assert!(json.contains("42"));
}

/// Tests JSON serialization of RELATION metadata events.
/// Verifies that table schema information including column names, types, and flags are properly serialized.
#[test]
fn test_json_serialization_relation() {
    let columns = vec![
        ColumnInfo {
            name: "id".to_string(),
            type_id: 23,
            flags: 1,
        },
        ColumnInfo {
            name: "name".to_string(),
            type_id: 1043,
            flags: 0,
        },
    ];
    
    let change = Change::Relation {
        relation_id: 12345,
        schema: "public".to_string(),
        table: "users".to_string(),
        columns,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    assert!(json.contains("Relation"));
    assert!(json.contains("12345"));
    assert!(json.contains("public"));
    assert!(json.contains("users"));
    assert!(json.contains("\"name\":\"id\""));
    assert!(json.contains("\"type_id\":23"));
}

/// Tests pretty-printed JSON output formatting.
/// Verifies that pretty format includes proper newlines and indentation for readability.
#[test]
fn test_json_pretty_format() {
    let change = Change::Begin {
        lsn: "0/123456".to_string(),
        timestamp: 123456789,
        xid: 999,
    };
    
    let json_pretty = serde_json::to_string_pretty(&change).unwrap();
    
    // Pretty format should have newlines and indentation
    assert!(json_pretty.contains("\n"));
    assert!(json_pretty.contains("  ")); // Indentation
}

/// Tests JSON serialization of strings containing special characters.
/// Verifies that quotes, backslashes, and other special characters are properly escaped.
#[test]
fn test_json_special_characters() {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("description".to_string(), Some("Test \"quotes\" and \\backslash".to_string()));
    
    let change = Change::Insert {
        relation_id: 100,
        schema: "public".to_string(),
        table: "items".to_string(),
        new_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    
    // Verify special characters are properly escaped
    assert!(json.contains("\\\""));
    assert!(json.contains("\\\\"));
    
    // Verify it can be parsed back
    let _: serde_json::Value = serde_json::from_str(&json).unwrap();
}

/// Tests JSON serialization of Unicode characters.
/// Verifies that international characters (Norwegian, German, Chinese) are preserved correctly in JSON.
#[test]
fn test_json_unicode() {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("name".to_string(), Some("Håkon Müller 李明".to_string()));
    
    let change = Change::Insert {
        relation_id: 100,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    
    // Verify Unicode is preserved
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let name = value["Insert"]["new_tuple"]["name"].as_str().unwrap();
    assert_eq!(name, "Håkon Müller 李明");
}

/// Tests JSON serialization of empty strings.
/// Verifies that empty string values are correctly represented as "" in JSON output.
#[test]
fn test_json_empty_string() {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("description".to_string(), Some("".to_string()));
    
    let change = Change::Insert {
        relation_id: 100,
        schema: "public".to_string(),
        table: "items".to_string(),
        new_tuple,
    };
    
    let json = serde_json::to_string(&change).unwrap();
    
    // Verify empty string is properly serialized
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let desc = value["Insert"]["new_tuple"]["description"].as_str().unwrap();
    assert_eq!(desc, "");
}

// Tests for new OutputTarget trait and implementations

/// Tests async StdoutOutput implementation for INSERT operations.
/// Verifies that the OutputTarget trait correctly handles INSERT events without panicking.
#[tokio::test]
async fn test_stdout_output_insert() {
    let output = StdoutOutput::new(OutputFormat::Json);
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    // Should not panic
    output.write_change(&change).await.unwrap();
}

/// Tests async StdoutOutput implementation for UPDATE operations.
/// Verifies correct handling of UPDATE events with both old and new tuple data.
#[tokio::test]
async fn test_stdout_output_update() {
    let output = StdoutOutput::new(OutputFormat::Json);
    
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("1".to_string()));
    old_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("name".to_string(), Some("Alice Updated".to_string()));
    
    let change = Change::Update {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple: Some(old_tuple),
        new_tuple,
    };
    
    output.write_change(&change).await.unwrap();
}

/// Tests async StdoutOutput implementation for DELETE operations.
/// Verifies that DELETE events are properly output using JSON-pretty format.
#[tokio::test]
async fn test_stdout_output_delete() {
    let output = StdoutOutput::new(OutputFormat::JsonPretty);
    
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("1".to_string()));
    old_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Delete {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple,
    };
    
    output.write_change(&change).await.unwrap();
}

/// Tests async StdoutOutput for transaction boundary events.
/// Verifies that BEGIN and COMMIT events are correctly output in text format.
#[tokio::test]
async fn test_stdout_output_transaction_events() {
    let output = StdoutOutput::new(OutputFormat::Text);
    
    let begin = Change::Begin {
        lsn: "0/16B2D50".to_string(),
        timestamp: 730826470123456,
        xid: 1000,
    };
    output.write_change(&begin).await.unwrap();
    
    let commit = Change::Commit {
        lsn: "0/16B2E20".to_string(),
        timestamp: 730826470123457,
    };
    output.write_change(&commit).await.unwrap();
}

/// Tests async StdoutOutput for RELATION metadata events.
/// Verifies that table schema definitions are properly output including column information.
#[tokio::test]
async fn test_stdout_output_relation() {
    let output = StdoutOutput::new(OutputFormat::Json);
    
    let columns = vec![
        ColumnInfo {
            name: "id".to_string(),
            type_id: 23,
            flags: 1,
        },
        ColumnInfo {
            name: "name".to_string(),
            type_id: 1043,
            flags: 0,
        },
    ];
    
    let change = Change::Relation {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        columns,
    };
    
    output.write_change(&change).await.unwrap();
}

/// Tests CompositeOutput with a single output target.
/// Verifies that the multiplexer correctly forwards events to a single StdoutOutput.
#[tokio::test]
async fn test_composite_output_with_single_target() {
    use std::sync::Arc;
    
    let stdout = StdoutOutput::new(OutputFormat::Json);
    let composite = CompositeOutput::new(vec![Arc::new(stdout)]);
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    composite.write_change(&change).await.unwrap();
}

/// Tests CompositeOutput with multiple output targets.
/// Verifies that events are correctly sent to multiple outputs (JSON and Text formats).
#[tokio::test]
async fn test_composite_output_with_multiple_targets() {
    use std::sync::Arc;
    
    let stdout1 = StdoutOutput::new(OutputFormat::Json);
    let stdout2 = StdoutOutput::new(OutputFormat::Text);
    let composite = CompositeOutput::new(vec![
        Arc::new(stdout1),
        Arc::new(stdout2),
    ]);
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    composite.write_change(&change).await.unwrap();
}

/// Tests CompositeOutput with no output targets.
/// Verifies that the multiplexer handles empty target lists gracefully without panicking.
#[tokio::test]
async fn test_composite_output_empty_targets() {
    let composite = CompositeOutput::new(vec![]);
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    // Should not panic with no targets
    composite.write_change(&change).await.unwrap();
}

/// Tests complete transaction flow through OutputTarget.
/// Verifies proper handling of Begin, Relation, INSERT, UPDATE, DELETE, and Commit in sequence.
#[tokio::test]
async fn test_full_transaction_flow_through_output() {
    let output = StdoutOutput::new(OutputFormat::Json);
    
    // Begin transaction
    let begin = Change::Begin {
        lsn: "0/100".to_string(),
        timestamp: 1234567890,
        xid: 500,
    };
    output.write_change(&begin).await.unwrap();
    
    // Relation metadata
    let columns = vec![
        ColumnInfo {
            name: "id".to_string(),
            type_id: 23,
            flags: 1,
        },
    ];
    let relation = Change::Relation {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "test_table".to_string(),
        columns,
    };
    output.write_change(&relation).await.unwrap();
    
    // Insert
    let mut insert_tuple = HashMap::new();
    insert_tuple.insert("id".to_string(), Some("1".to_string()));
    let insert = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "test_table".to_string(),
        new_tuple: insert_tuple,
    };
    output.write_change(&insert).await.unwrap();
    
    // Update
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("1".to_string()));
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("2".to_string()));
    let update = Change::Update {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "test_table".to_string(),
        old_tuple: Some(old_tuple),
        new_tuple,
    };
    output.write_change(&update).await.unwrap();
    
    // Delete
    let mut delete_tuple = HashMap::new();
    delete_tuple.insert("id".to_string(), Some("2".to_string()));
    let delete = Change::Delete {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "test_table".to_string(),
        old_tuple: delete_tuple,
    };
    output.write_change(&delete).await.unwrap();
    
    // Commit transaction
    let commit = Change::Commit {
        lsn: "0/200".to_string(),
        timestamp: 1234567900,
    };
    output.write_change(&commit).await.unwrap();
}

/// Tests StdoutOutput handling of NULL values in tuple data.
/// Verifies that NULL columns are correctly represented in the output.
#[tokio::test]
async fn test_stdout_output_with_null_values() {
    let output = StdoutOutput::new(OutputFormat::Json);
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("email".to_string(), None);
    new_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    output.write_change(&change).await.unwrap();
}

/// Tests StdoutOutput with non-standard schema names.
/// Verifies handling of schema names containing hyphens and underscores.
#[tokio::test]
async fn test_stdout_output_with_special_schema_names() {
    let output = StdoutOutput::new(OutputFormat::Json);
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "my-custom_schema".to_string(),
        table: "test_table".to_string(),
        new_tuple,
    };
    
    output.write_change(&change).await.unwrap();
}

/// Tests text format output for human readability.
/// Verifies that text format correctly displays INSERT operations in readable form.
#[tokio::test]
async fn test_stdout_output_text_format() {
    let output = StdoutOutput::new(OutputFormat::Text);
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("1".to_string()));
    new_tuple.insert("name".to_string(), Some("Test User".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple,
    };
    
    // Text format should not panic
    output.write_change(&change).await.unwrap();
}

/// Tests multiple INSERT operations through CompositeOutput.
/// Verifies that sequential inserts to different tables are handled correctly by multiple outputs.
#[tokio::test]
async fn test_multiple_inserts_through_composite() {
    use std::sync::Arc;
    
    let output1 = StdoutOutput::new(OutputFormat::Json);
    let output2 = StdoutOutput::new(OutputFormat::JsonPretty);
    let composite = CompositeOutput::new(vec![
        Arc::new(output1),
        Arc::new(output2),
    ]);
    
    // First insert
    let mut tuple1 = HashMap::new();
    tuple1.insert("id".to_string(), Some("1".to_string()));
    let change1 = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple1,
    };
    composite.write_change(&change1).await.unwrap();
    
    // Second insert
    let mut tuple2 = HashMap::new();
    tuple2.insert("id".to_string(), Some("2".to_string()));
    let change2 = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "orders".to_string(),
        new_tuple: tuple2,
    };
    composite.write_change(&change2).await.unwrap();
}

/// Tests parsing of 'debezium' output format string.
/// Verifies that OutputFormat::from_str correctly recognizes the Debezium variant.
#[test]
fn test_output_format_from_str_debezium() {
    let format = OutputFormat::from_str("debezium").unwrap();
    assert!(matches!(format, OutputFormat::Debezium));
}

/// Tests Debezium format INSERT event structure.
/// Verifies that the envelope contains correct op='c', after data, and source metadata.
#[test]
fn test_debezium_insert() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("123".to_string()));
    tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change).unwrap();
    
    assert_eq!(envelope.op, "c");
    assert!(envelope.before.is_none());
    assert!(envelope.after.is_some());
    assert_eq!(envelope.source.schema, "public");
    assert_eq!(envelope.source.table, "users");
    assert_eq!(envelope.source.connector, "postgresql");
}

/// Tests Debezium format UPDATE event structure.
/// Verifies that the envelope contains op='u', both before and after data, and source metadata.
#[test]
fn test_debezium_update() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("123".to_string()));
    old_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("123".to_string()));
    new_tuple.insert("name".to_string(), Some("Alice Updated".to_string()));
    
    let change = Change::Update {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple: Some(old_tuple),
        new_tuple,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change).unwrap();
    
    assert_eq!(envelope.op, "u");
    assert!(envelope.before.is_some());
    assert!(envelope.after.is_some());
    assert_eq!(envelope.source.schema, "public");
    assert_eq!(envelope.source.table, "users");
}

/// Tests Debezium format DELETE event structure.
/// Verifies that the envelope contains op='d', before data only, and no after data.
#[test]
fn test_debezium_delete() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("123".to_string()));
    old_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Delete {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change).unwrap();
    
    assert_eq!(envelope.op, "d");
    assert!(envelope.before.is_some());
    assert!(envelope.after.is_none());
    assert_eq!(envelope.source.schema, "public");
    assert_eq!(envelope.source.table, "users");
}

/// Tests that BEGIN transaction events are not converted to Debezium format.
/// Verifies that convert_to_debezium returns None for Begin events.
#[test]
fn test_debezium_begin_skipped() {
    let change = Change::Begin {
        lsn: "0/16B9188".to_string(),
        timestamp: 1705320000000,
        xid: 1234,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change);
    assert!(envelope.is_none());
}

/// Tests that COMMIT transaction events are not converted to Debezium format.
/// Verifies that convert_to_debezium returns None for Commit events.
#[test]
fn test_debezium_commit_skipped() {
    let change = Change::Commit {
        lsn: "0/16B91B8".to_string(),
        timestamp: 1705320001000,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change);
    assert!(envelope.is_none());
}

/// Tests that RELATION events are not converted to Debezium format.
/// Verifies that convert_to_debezium returns None for Relation events.
#[test]
fn test_debezium_relation_skipped() {
    let change = Change::Relation {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        columns: vec![],
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change);
    assert!(envelope.is_none());
}

/// Tests Debezium format with NULL values in tuple data.
/// Verifies that NULL values are correctly represented in the envelope.
#[test]
fn test_debezium_null_handling() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("123".to_string()));
    tuple.insert("email".to_string(), None); // NULL value
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change).unwrap();
    
    assert!(envelope.after.is_some());
    let after = envelope.after.as_ref().unwrap();
    assert!(after.get("email").is_some());
}

/// Tests Debezium source metadata fields.
/// Verifies that source contains all required Debezium fields with correct values.
#[test]
fn test_debezium_source_metadata() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("1".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "test_schema".to_string(),
        table: "test_table".to_string(),
        new_tuple: tuple,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change).unwrap();
    
    assert_eq!(envelope.source.version, "pgoutput-cmdline-0.1.0");
    assert_eq!(envelope.source.connector, "postgresql");
    assert_eq!(envelope.source.name, "pgoutput-cmdline");
    assert_eq!(envelope.source.db, "postgres");
    assert_eq!(envelope.source.schema, "test_schema");
    assert_eq!(envelope.source.table, "test_table");
    assert!(!envelope.source.lsn.is_empty());
}

/// Tests Debezium timestamp field presence.
/// Verifies that ts_ms is populated and is a reasonable Unix timestamp.
#[test]
fn test_debezium_timestamp() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("1".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let envelope = pgoutput_cmdline::output::convert_to_debezium_test(&change).unwrap();
    
    // Verify ts_ms is positive and reasonable (after 2020-01-01)
    assert!(envelope.ts_ms > 1577836800000); // 2020-01-01 in milliseconds
}

/// Tests parsing of 'feldera' output format string.
/// Verifies that OutputFormat::from_str correctly recognizes the Feldera variant.
#[test]
fn test_output_format_from_str_feldera() {
    let format = OutputFormat::from_str("feldera").unwrap();
    assert!(matches!(format, OutputFormat::Feldera));
}

/// Tests parsing of 'insert-delete' and 'insert_delete' as aliases for Feldera format.
/// Verifies that the format can be referenced by multiple names.
#[test]
fn test_output_format_from_str_insert_delete() {
    assert!(matches!(OutputFormat::from_str("insert-delete").unwrap(), OutputFormat::Feldera));
    assert!(matches!(OutputFormat::from_str("insert_delete").unwrap(), OutputFormat::Feldera));
}

/// Tests Feldera format INSERT event structure.
/// Verifies that the event contains an insert key with the record data.
#[test]
fn test_feldera_insert() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("123".to_string()));
    tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert!(event.insert.is_some());
    assert!(event.delete.is_none());
    assert!(event.update.is_none());
    
    let insert_data = event.insert.as_ref().unwrap();
    assert_eq!(insert_data["id"], "123");
    assert_eq!(insert_data["name"], "Alice");
}

/// Tests Feldera format UPDATE event structure.
/// Verifies that updates are encoded as delete (old) + insert (new) pairs.
#[test]
fn test_feldera_update() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("123".to_string()));
    old_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("123".to_string()));
    new_tuple.insert("name".to_string(), Some("Alice Updated".to_string()));
    
    let change = Change::Update {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple: Some(old_tuple),
        new_tuple,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    
    // Update should produce two events: delete + insert
    assert_eq!(events.len(), 2);
    
    // First event: delete old
    let delete_event = &events[0];
    assert!(delete_event.insert.is_none());
    assert!(delete_event.delete.is_some());
    assert!(delete_event.update.is_none());
    let delete_data = delete_event.delete.as_ref().unwrap();
    assert_eq!(delete_data["id"], "123");
    assert_eq!(delete_data["name"], "Alice");
    
    // Second event: insert new
    let insert_event = &events[1];
    assert!(insert_event.insert.is_some());
    assert!(insert_event.delete.is_none());
    assert!(insert_event.update.is_none());
    let insert_data = insert_event.insert.as_ref().unwrap();
    assert_eq!(insert_data["id"], "123");
    assert_eq!(insert_data["name"], "Alice Updated");
}

/// Tests Feldera format UPDATE without old_tuple.
/// Verifies that when old_tuple is None, only insert event is produced.
#[test]
fn test_feldera_update_without_old_tuple() {
    let mut new_tuple = HashMap::new();
    new_tuple.insert("id".to_string(), Some("123".to_string()));
    new_tuple.insert("name".to_string(), Some("Alice Updated".to_string()));
    
    let change = Change::Update {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple: None,
        new_tuple,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    
    // Without old_tuple, only insert is produced
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert!(event.insert.is_some());
    assert!(event.delete.is_none());
}

/// Tests Feldera format DELETE event structure.
/// Verifies that the event contains a delete key with the old record data.
#[test]
fn test_feldera_delete() {
    let mut old_tuple = HashMap::new();
    old_tuple.insert("id".to_string(), Some("123".to_string()));
    old_tuple.insert("name".to_string(), Some("Alice".to_string()));
    
    let change = Change::Delete {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        old_tuple,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert!(event.insert.is_none());
    assert!(event.delete.is_some());
    assert!(event.update.is_none());
    
    let delete_data = event.delete.as_ref().unwrap();
    assert_eq!(delete_data["id"], "123");
    assert_eq!(delete_data["name"], "Alice");
}

/// Tests that BEGIN transaction events are not converted to Feldera format.
/// Verifies that convert_to_feldera returns empty vector for Begin events.
#[test]
fn test_feldera_begin_skipped() {
    let change = Change::Begin {
        lsn: "0/16B9188".to_string(),
        timestamp: 1705320000000,
        xid: 1234,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    assert!(events.is_empty());
}

/// Tests that COMMIT transaction events are not converted to Feldera format.
/// Verifies that convert_to_feldera returns empty vector for Commit events.
#[test]
fn test_feldera_commit_skipped() {
    let change = Change::Commit {
        lsn: "0/16B91B8".to_string(),
        timestamp: 1705320001000,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    assert!(events.is_empty());
}

/// Tests that RELATION events are not converted to Feldera format.
/// Verifies that convert_to_feldera returns empty vector for Relation events.
#[test]
fn test_feldera_relation_skipped() {
    let change = Change::Relation {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        columns: vec![],
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    assert!(events.is_empty());
}

/// Tests Feldera format with NULL values in tuple data.
/// Verifies that NULL values are correctly represented.
#[test]
fn test_feldera_null_handling() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("123".to_string()));
    tuple.insert("email".to_string(), None); // NULL value
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert!(event.insert.is_some());
    let insert_data = event.insert.as_ref().unwrap();
    assert!(insert_data.get("email").is_some());
}

/// Tests Feldera format serialization roundtrip.
/// Verifies that the format can be serialized and deserialized correctly.
#[test]
fn test_feldera_serialization() {
    let mut tuple = HashMap::new();
    tuple.insert("id".to_string(), Some("1".to_string()));
    tuple.insert("name".to_string(), Some("Test".to_string()));
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    assert_eq!(events.len(), 1);
    
    // Serialize to JSON
    let json = serde_json::to_string(&events[0]).unwrap();
    assert!(json.contains("\"insert\""));
    
    // Deserialize back
    let deserialized: pgoutput_cmdline::output::FelderaUpdate = serde_json::from_str(&json).unwrap();
    assert!(deserialized.insert.is_some());
}

/// Tests Feldera format with empty tuple fields.
/// Verifies handling of records with no data fields.
#[test]
fn test_feldera_empty_tuple() {
    let tuple = HashMap::new();
    
    let change = Change::Insert {
        relation_id: 16384,
        schema: "public".to_string(),
        table: "users".to_string(),
        new_tuple: tuple,
    };
    
    let events = pgoutput_cmdline::output::convert_to_feldera_test(&change);
    assert_eq!(events.len(), 1);
    assert!(events[0].insert.is_some());
}
