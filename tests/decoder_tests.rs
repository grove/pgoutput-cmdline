use pgoutput_stream::decoder::*;

/// Tests decoding of BEGIN transaction messages from the pgoutput protocol.
/// Verifies that LSN (Log Sequence Number), timestamp, and transaction ID (xid) are correctly parsed.
#[test]
fn test_decode_begin() {
    // BEGIN message format: 'B' + LSN(8) + timestamp(8) + xid(4)
    let mut data = vec![b'B'];
    data.extend_from_slice(&0x0000000001234567u64.to_be_bytes()); // LSN
    data.extend_from_slice(&123456789i64.to_be_bytes());           // timestamp
    data.extend_from_slice(&999u32.to_be_bytes());                 // xid

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Begin { lsn, timestamp, xid }) => {
            assert_eq!(lsn, "0/1234567");
            assert_eq!(timestamp, 123456789);
            assert_eq!(xid, 999);
        }
        _ => panic!("Expected Begin change"),
    }
}

/// Tests decoding of COMMIT transaction messages from the pgoutput protocol.
/// Verifies that commit LSN and timestamp are correctly extracted.
#[test]
fn test_decode_commit() {
    // COMMIT message format: 'C' + flags(1) + LSN(8) + end_lsn(8) + timestamp(8)
    let mut data = vec![b'C', 0]; // 'C' + flags
    data.extend_from_slice(&0x0000000001234567u64.to_be_bytes()); // LSN
    data.extend_from_slice(&0x0000000001234568u64.to_be_bytes()); // end LSN
    data.extend_from_slice(&987654321i64.to_be_bytes());           // timestamp

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Commit { lsn, timestamp }) => {
            assert_eq!(lsn, "0/1234567");
            assert_eq!(timestamp, 987654321);
        }
        _ => panic!("Expected Commit change"),
    }
}

/// Tests decoding of RELATION metadata messages that define table schemas.
/// Verifies extraction of relation ID, schema name, table name, and column definitions
/// including column names, PostgreSQL type IDs, and flags.
#[test]
fn test_decode_relation() {
    // RELATION message format: 'R' + relation_id(4) + schema + table + replica_identity(1) + column_count(2) + columns
    let mut data = vec![b'R'];
    data.extend_from_slice(&12345u32.to_be_bytes()); // relation_id
    data.extend_from_slice(b"public\0");              // schema name
    data.extend_from_slice(b"users\0");               // table name
    data.push(1);                                     // replica_identity
    data.extend_from_slice(&2u16.to_be_bytes());     // column count
    
    // Column 1: id (type_id=23, flags=1)
    data.push(1);                                     // flags
    data.extend_from_slice(b"id\0");                  // column name
    data.extend_from_slice(&23u32.to_be_bytes());    // type_id (integer)
    data.extend_from_slice(&(-1i32).to_be_bytes());  // type_modifier
    
    // Column 2: name (type_id=1043, flags=0)
    data.push(0);                                     // flags
    data.extend_from_slice(b"name\0");                // column name
    data.extend_from_slice(&1043u32.to_be_bytes());  // type_id (varchar)
    data.extend_from_slice(&(-1i32).to_be_bytes());  // type_modifier

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Relation { relation_id, schema, table, columns }) => {
            assert_eq!(relation_id, 12345);
            assert_eq!(schema, "public");
            assert_eq!(table, "users");
            assert_eq!(columns.len(), 2);
            assert_eq!(columns[0].name, "id");
            assert_eq!(columns[0].type_id, 23);
            assert_eq!(columns[0].flags, 1);
            assert_eq!(columns[1].name, "name");
            assert_eq!(columns[1].type_id, 1043);
            assert_eq!(columns[1].flags, 0);
        }
        _ => panic!("Expected Relation change"),
    }
}

/// Tests decoding of INSERT operations from the replication stream.
/// First registers a relation (table schema) then decodes an INSERT message.
/// Verifies that column values are correctly extracted and matched to column names.
#[test]
fn test_decode_insert() {
    // First, register a relation so we can decode the insert
    let mut relation_data = vec![b'R'];
    relation_data.extend_from_slice(&100u32.to_be_bytes());
    relation_data.extend_from_slice(b"public\0");
    relation_data.extend_from_slice(b"test_table\0");
    relation_data.push(1);
    relation_data.extend_from_slice(&2u16.to_be_bytes());
    relation_data.push(1);
    relation_data.extend_from_slice(b"id\0");
    relation_data.extend_from_slice(&23u32.to_be_bytes());
    relation_data.extend_from_slice(&(-1i32).to_be_bytes());
    relation_data.push(0);
    relation_data.extend_from_slice(b"name\0");
    relation_data.extend_from_slice(&1043u32.to_be_bytes());
    relation_data.extend_from_slice(&(-1i32).to_be_bytes());
    
    decode_pgoutput_message(&relation_data).unwrap();

    // INSERT message format: 'I' + relation_id(4) + 'N' + tuple_data
    let mut data = vec![b'I'];
    data.extend_from_slice(&100u32.to_be_bytes()); // relation_id
    data.push(b'N');                                // new tuple indicator
    data.extend_from_slice(&2u16.to_be_bytes());   // column count
    
    // Column 1: id = "1"
    data.push(b't');                                // text data type
    data.extend_from_slice(&1u32.to_be_bytes());   // length
    data.push(b'1');                                // value
    
    // Column 2: name = "Alice"
    data.push(b't');                                // text data type
    data.extend_from_slice(&5u32.to_be_bytes());   // length
    data.extend_from_slice(b"Alice");               // value

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Insert { relation_id, schema, table, new_tuple }) => {
            assert_eq!(relation_id, 100);
            assert_eq!(schema, "public");
            assert_eq!(table, "test_table");
            assert_eq!(new_tuple.get("id"), Some(&Some("1".to_string())));
            assert_eq!(new_tuple.get("name"), Some(&Some("Alice".to_string())));
        }
        _ => panic!("Expected Insert change"),
    }
}

/// Tests decoding of INSERT operations that contain NULL values.
/// Verifies that NULL indicators in the protocol are correctly identified
/// and represented as None in the resulting tuple data.
#[test]
fn test_decode_insert_with_null() {
    // Register relation
    let mut relation_data = vec![b'R'];
    relation_data.extend_from_slice(&101u32.to_be_bytes());
    relation_data.extend_from_slice(b"public\0");
    relation_data.extend_from_slice(b"nullable_table\0");
    relation_data.push(1);
    relation_data.extend_from_slice(&2u16.to_be_bytes());
    relation_data.push(1);
    relation_data.extend_from_slice(b"id\0");
    relation_data.extend_from_slice(&23u32.to_be_bytes());
    relation_data.extend_from_slice(&(-1i32).to_be_bytes());
    relation_data.push(0);
    relation_data.extend_from_slice(b"email\0");
    relation_data.extend_from_slice(&1043u32.to_be_bytes());
    relation_data.extend_from_slice(&(-1i32).to_be_bytes());
    
    decode_pgoutput_message(&relation_data).unwrap();

    // INSERT with NULL value
    let mut data = vec![b'I'];
    data.extend_from_slice(&101u32.to_be_bytes());
    data.push(b'N');
    data.extend_from_slice(&2u16.to_be_bytes());
    data.push(b't');
    data.extend_from_slice(&1u32.to_be_bytes());
    data.push(b'1');
    data.push(b'n'); // NULL indicator

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Insert { new_tuple, .. }) => {
            assert_eq!(new_tuple.get("id"), Some(&Some("1".to_string())));
            assert_eq!(new_tuple.get("email"), Some(&None));
        }
        _ => panic!("Expected Insert change"),
    }
}

/// Tests decoding of UPDATE operations that include old tuple data.
/// When REPLICA IDENTITY FULL is set, UPDATE messages include both old and new values.
/// Verifies that both old and new tuple data are correctly parsed.
#[test]
fn test_decode_update_with_old_tuple() {
    // Register relation
    let mut relation_data = vec![b'R'];
    relation_data.extend_from_slice(&102u32.to_be_bytes());
    relation_data.extend_from_slice(b"public\0");
    relation_data.extend_from_slice(b"users\0");
    relation_data.push(1);
    relation_data.extend_from_slice(&1u16.to_be_bytes());
    relation_data.push(1);
    relation_data.extend_from_slice(b"name\0");
    relation_data.extend_from_slice(&1043u32.to_be_bytes());
    relation_data.extend_from_slice(&(-1i32).to_be_bytes());
    
    decode_pgoutput_message(&relation_data).unwrap();

    // UPDATE message with old tuple: 'U' + relation_id(4) + 'O' + old_tuple + 'N' + new_tuple
    let mut data = vec![b'U'];
    data.extend_from_slice(&102u32.to_be_bytes());
    data.push(b'O'); // old tuple indicator
    data.extend_from_slice(&1u16.to_be_bytes());
    data.push(b't');
    data.extend_from_slice(&3u32.to_be_bytes());
    data.extend_from_slice(b"Bob");
    data.push(b'N'); // new tuple indicator
    data.extend_from_slice(&1u16.to_be_bytes());
    data.push(b't');
    data.extend_from_slice(&5u32.to_be_bytes());
    data.extend_from_slice(b"Alice");

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Update { old_tuple, new_tuple, .. }) => {
            assert!(old_tuple.is_some());
            let old = old_tuple.unwrap();
            assert_eq!(old.get("name"), Some(&Some("Bob".to_string())));
            assert_eq!(new_tuple.get("name"), Some(&Some("Alice".to_string())));
        }
        _ => panic!("Expected Update change"),
    }
}

/// Tests decoding of UPDATE operations without old tuple data.
/// When REPLICA IDENTITY DEFAULT is set, UPDATE messages only include new values.
/// Verifies that the decoder handles missing old tuple data correctly.
#[test]
fn test_decode_update_without_old_tuple() {
    // Register relation
    let mut relation_data = vec![b'R'];
    relation_data.extend_from_slice(&103u32.to_be_bytes());
    relation_data.extend_from_slice(b"public\0");
    relation_data.extend_from_slice(b"users\0");
    relation_data.push(1);
    relation_data.extend_from_slice(&1u16.to_be_bytes());
    relation_data.push(1);
    relation_data.extend_from_slice(b"name\0");
    relation_data.extend_from_slice(&1043u32.to_be_bytes());
    relation_data.extend_from_slice(&(-1i32).to_be_bytes());
    
    decode_pgoutput_message(&relation_data).unwrap();

    // UPDATE message without old tuple: 'U' + relation_id(4) + 'N' + new_tuple
    let mut data = vec![b'U'];
    data.extend_from_slice(&103u32.to_be_bytes());
    data.push(b'N'); // new tuple indicator (no old tuple)
    data.extend_from_slice(&1u16.to_be_bytes());
    data.push(b't');
    data.extend_from_slice(&5u32.to_be_bytes());
    data.extend_from_slice(b"Carol");

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Update { old_tuple, new_tuple, .. }) => {
            assert!(old_tuple.is_none());
            assert_eq!(new_tuple.get("name"), Some(&Some("Carol".to_string())));
        }
        _ => panic!("Expected Update change"),
    }
}

/// Tests decoding of DELETE operations from the replication stream.
/// Verifies that the old tuple data (deleted row values) is correctly extracted
/// and associated with the proper relation.
#[test]
fn test_decode_delete() {
    // Register relation
    let mut relation_data = vec![b'R'];
    relation_data.extend_from_slice(&104u32.to_be_bytes());
    relation_data.extend_from_slice(b"public\0");
    relation_data.extend_from_slice(b"users\0");
    relation_data.push(1);
    relation_data.extend_from_slice(&1u16.to_be_bytes());
    relation_data.push(1);
    relation_data.extend_from_slice(b"id\0");
    relation_data.extend_from_slice(&23u32.to_be_bytes());
    relation_data.extend_from_slice(&(-1i32).to_be_bytes());
    
    decode_pgoutput_message(&relation_data).unwrap();

    // DELETE message: 'D' + relation_id(4) + 'K' or 'O' + old_tuple
    let mut data = vec![b'D'];
    data.extend_from_slice(&104u32.to_be_bytes());
    data.push(b'K'); // key/old tuple indicator
    data.extend_from_slice(&1u16.to_be_bytes());
    data.push(b't');
    data.extend_from_slice(&2u32.to_be_bytes());
    data.extend_from_slice(b"42");

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Delete { relation_id, old_tuple, .. }) => {
            assert_eq!(relation_id, 104);
            assert_eq!(old_tuple.get("id"), Some(&Some("42".to_string())));
        }
        _ => panic!("Expected Delete change"),
    }
}

/// Tests decoder behavior with empty message data.
/// Verifies that the decoder gracefully handles empty byte arrays
/// without panicking or producing invalid output.
#[test]
fn test_decode_empty_message() {
    let data = vec![];
    let result = decode_pgoutput_message(&data).unwrap();
    assert!(result.is_none());
}

/// Tests decoder behavior with unknown message type indicators.
/// Verifies that unrecognized message types are handled gracefully
/// without causing errors or crashes.
#[test]
fn test_decode_unknown_message_type() {
    let data = vec![b'X']; // Unknown message type
    let result = decode_pgoutput_message(&data).unwrap();
    assert!(result.is_none());
}

/// Tests decoding of BEGIN messages with maximum LSN values.
/// Ensures the decoder can handle edge cases with very large LSN numbers
/// that use the full 64-bit range.
#[test]
fn test_decode_begin_with_large_lsn() {
    let mut data = vec![b'B'];
    data.extend_from_slice(&0xFFFFFFFFFFFFFFFFu64.to_be_bytes());
    data.extend_from_slice(&9999999999i64.to_be_bytes());
    data.extend_from_slice(&4294967295u32.to_be_bytes()); // max u32

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Begin { lsn, timestamp, xid }) => {
            assert_eq!(lsn, "FFFFFFFF/FFFFFFFF");
            assert_eq!(timestamp, 9999999999);
            assert_eq!(xid, 4294967295);
        }
        _ => panic!("Expected Begin change"),
    }
}

/// Tests decoding of RELATION messages with special characters in identifiers.
/// Verifies that schema and table names containing underscores, hyphens,
/// and other special characters are correctly handled.
#[test]
fn test_decode_relation_with_special_characters() {
    let mut data = vec![b'R'];
    data.extend_from_slice(&999u32.to_be_bytes());
    data.extend_from_slice(b"my_schema\0");
    data.extend_from_slice(b"table_with_underscores\0");
    data.push(1);
    data.extend_from_slice(&1u16.to_be_bytes());
    data.push(0);
    data.extend_from_slice(b"col_name\0");
    data.extend_from_slice(&1043u32.to_be_bytes());
    data.extend_from_slice(&(-1i32).to_be_bytes());

    let result = decode_pgoutput_message(&data).unwrap();
    
    match result {
        Some(Change::Relation { schema, table, .. }) => {
            assert_eq!(schema, "my_schema");
            assert_eq!(table, "table_with_underscores");
        }
        _ => panic!("Expected Relation change"),
    }
}
