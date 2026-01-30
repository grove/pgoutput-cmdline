use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    Begin {
        lsn: String,
        timestamp: i64,
        xid: u32,
    },
    Commit {
        lsn: String,
        timestamp: i64,
    },
    Insert {
        relation_id: u32,
        schema: String,
        table: String,
        new_tuple: HashMap<String, Option<String>>,
    },
    Update {
        relation_id: u32,
        schema: String,
        table: String,
        old_tuple: Option<HashMap<String, Option<String>>>,
        new_tuple: HashMap<String, Option<String>>,
    },
    Delete {
        relation_id: u32,
        schema: String,
        table: String,
        old_tuple: HashMap<String, Option<String>>,
    },
    Relation {
        relation_id: u32,
        schema: String,
        table: String,
        columns: Vec<ColumnInfo>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub type_id: u32,
    pub flags: u8,
}

// Thread-safe relation cache
static RELATION_CACHE: Lazy<Mutex<HashMap<u32, (String, String, Vec<ColumnInfo>)>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Get column metadata for a relation from the cache
pub fn get_relation_columns(relation_id: u32) -> Option<Vec<ColumnInfo>> {
    let cache = RELATION_CACHE.lock().unwrap();
    cache.get(&relation_id).map(|(_, _, cols)| cols.clone())
}

pub fn decode_pgoutput_message(data: &[u8]) -> Result<Option<Change>> {
    if data.is_empty() {
        return Ok(None);
    }

    let msg_type = data[0] as char;
    let rest = &data[1..];

    match msg_type {
        'B' => decode_begin(rest),
        'C' => decode_commit(rest),
        'R' => decode_relation(rest),
        'I' => decode_insert(rest),
        'U' => decode_update(rest),
        'D' => decode_delete(rest),
        'O' | 'T' | 'Y' => {
            // Origin, Type, Truncate - not implemented yet
            Ok(None)
        }
        _ => {
            eprintln!("Unknown message type: {}", msg_type);
            Ok(None)
        }
    }
}

fn decode_begin(data: &[u8]) -> Result<Option<Change>> {
    if data.len() < 20 {
        return Err(anyhow!("Invalid BEGIN message length"));
    }

    let lsn = u64::from_be_bytes(data[0..8].try_into()?);
    let timestamp = i64::from_be_bytes(data[8..16].try_into()?);
    let xid = u32::from_be_bytes(data[16..20].try_into()?);

    Ok(Some(Change::Begin {
        lsn: format!("{:X}/{:X}", lsn >> 32, lsn & 0xFFFFFFFF),
        timestamp,
        xid,
    }))
}

fn decode_commit(data: &[u8]) -> Result<Option<Change>> {
    if data.len() < 17 {
        return Err(anyhow!("Invalid COMMIT message length"));
    }

    let _flags = data[0];
    let lsn = u64::from_be_bytes(data[1..9].try_into()?);
    let _end_lsn = u64::from_be_bytes(data[9..17].try_into()?);
    let timestamp = i64::from_be_bytes(data[17..25].try_into()?);

    Ok(Some(Change::Commit {
        lsn: format!("{:X}/{:X}", lsn >> 32, lsn & 0xFFFFFFFF),
        timestamp,
    }))
}

fn decode_relation(data: &[u8]) -> Result<Option<Change>> {
    let mut pos = 0;

    let relation_id = u32::from_be_bytes(data[pos..pos + 4].try_into()?);
    pos += 4;

    let schema = read_string(data, &mut pos)?;
    let table = read_string(data, &mut pos)?;

    let _replica_identity = data[pos];
    pos += 1;

    let column_count = u16::from_be_bytes(data[pos..pos + 2].try_into()?) as usize;
    pos += 2;

    let mut columns = Vec::new();
    for _ in 0..column_count {
        let _flags = data[pos];
        pos += 1;

        let name = read_string(data, &mut pos)?;

        let type_id = u32::from_be_bytes(data[pos..pos + 4].try_into()?);
        pos += 4;

        let _type_modifier = i32::from_be_bytes(data[pos..pos + 4].try_into()?);
        pos += 4;

        columns.push(ColumnInfo {
            name,
            type_id,
            flags: _flags,
        });
    }

    // Cache the relation info
    let mut cache = RELATION_CACHE.lock().unwrap();
    cache.insert(relation_id, (schema.clone(), table.clone(), columns.clone()));
    drop(cache);

    Ok(Some(Change::Relation {
        relation_id,
        schema,
        table,
        columns,
    }))
}

fn decode_insert(data: &[u8]) -> Result<Option<Change>> {
    let mut pos = 0;

    let relation_id = u32::from_be_bytes(data[pos..pos + 4].try_into()?);
    pos += 4;

    let tuple_type = data[pos] as char;
    pos += 1;

    if tuple_type != 'N' {
        return Err(anyhow!("Expected 'N' (new tuple) in INSERT"));
    }

    let new_tuple = decode_tuple(data, &mut pos, relation_id)?;

    let cache = RELATION_CACHE.lock().unwrap();
    let (schema, table, _) = cache
        .get(&relation_id)
        .ok_or_else(|| anyhow!("Relation {} not found in cache", relation_id))?
        .clone();
    drop(cache);

    Ok(Some(Change::Insert {
        relation_id,
        schema,
        table,
        new_tuple,
    }))
}

fn decode_update(data: &[u8]) -> Result<Option<Change>> {
    let mut pos = 0;

    let relation_id = u32::from_be_bytes(data[pos..pos + 4].try_into()?);
    pos += 4;

    let tuple_type = data[pos] as char;
    pos += 1;

    let old_tuple = if tuple_type == 'K' || tuple_type == 'O' {
        let tuple = decode_tuple(data, &mut pos, relation_id)?;
        let next_type = data[pos] as char;
        pos += 1;
        if next_type != 'N' {
            return Err(anyhow!("Expected 'N' after old tuple in UPDATE"));
        }
        Some(tuple)
    } else if tuple_type == 'N' {
        None
    } else {
        return Err(anyhow!("Unexpected tuple type in UPDATE: {}", tuple_type));
    };

    let new_tuple = decode_tuple(data, &mut pos, relation_id)?;

    let cache = RELATION_CACHE.lock().unwrap();
    let (schema, table, _) = cache
        .get(&relation_id)
        .ok_or_else(|| anyhow!("Relation {} not found in cache", relation_id))?
        .clone();
    drop(cache);

    Ok(Some(Change::Update {
        relation_id,
        schema,
        table,
        old_tuple,
        new_tuple,
    }))
}

fn decode_delete(data: &[u8]) -> Result<Option<Change>> {
    let mut pos = 0;

    let relation_id = u32::from_be_bytes(data[pos..pos + 4].try_into()?);
    pos += 4;

    let tuple_type = data[pos] as char;
    pos += 1;

    if tuple_type != 'K' && tuple_type != 'O' {
        return Err(anyhow!("Expected 'K' or 'O' (old tuple) in DELETE"));
    }

    let old_tuple = decode_tuple(data, &mut pos, relation_id)?;

    let cache = RELATION_CACHE.lock().unwrap();
    let (schema, table, _) = cache
        .get(&relation_id)
        .ok_or_else(|| anyhow!("Relation {} not found in cache", relation_id))?
        .clone();
    drop(cache);

    Ok(Some(Change::Delete {
        relation_id,
        schema,
        table,
        old_tuple,
    }))
}

fn decode_tuple(
    data: &[u8],
    pos: &mut usize,
    relation_id: u32,
) -> Result<HashMap<String, Option<String>>> {
    let column_count = u16::from_be_bytes(data[*pos..*pos + 2].try_into()?) as usize;
    *pos += 2;

    let cache = RELATION_CACHE.lock().unwrap();
    let (_, _, columns) = cache
        .get(&relation_id)
        .ok_or_else(|| anyhow!("Relation {} not found in cache", relation_id))?;
    let columns = columns.clone();
    drop(cache);

    let mut tuple = HashMap::new();

    for i in 0..column_count {
        let column_name = if i < columns.len() {
            columns[i].name.clone()
        } else {
            format!("column_{}", i)
        };

        let tuple_type = data[*pos] as char;
        *pos += 1;

        let value = match tuple_type {
            'n' => None, // NULL
            'u' => None, // UNCHANGED (for UPDATE)
            't' => {
                // Text value
                let length = u32::from_be_bytes(data[*pos..*pos + 4].try_into()?) as usize;
                *pos += 4;
                let value = String::from_utf8_lossy(&data[*pos..*pos + length]).to_string();
                *pos += length;
                Some(value)
            }
            _ => {
                return Err(anyhow!("Unknown tuple data type: {}", tuple_type));
            }
        };

        tuple.insert(column_name, value);
    }

    Ok(tuple)
}

fn read_string(data: &[u8], pos: &mut usize) -> Result<String> {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    let s = String::from_utf8_lossy(&data[start..*pos]).to_string();
    *pos += 1; // Skip null terminator
    Ok(s)
}
