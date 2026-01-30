use anyhow::{anyhow, Result};
use crate::decoder::{Change, ColumnInfo};
use serde_json;
use async_nats::jetstream;
use std::sync::Arc;
use std::collections::HashMap;
use reqwest::{Client, header};

/// Convert a tuple (HashMap of string values) to proper JSON types based on column metadata
fn tuple_to_json_with_types(
    tuple: &HashMap<String, Option<String>>,
    columns: &[ColumnInfo],
) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    
    for col in columns {
        if let Some(value_opt) = tuple.get(&col.name) {
            let json_value = match value_opt {
                None => serde_json::Value::Null,
                Some(string_val) => {
                    // Convert based on PostgreSQL type OID
                    // Common PostgreSQL type OIDs:
                    // 16 = bool, 20 = int8, 21 = int2, 23 = int4
                    // 700 = float4, 701 = float8
                    // 1700 = numeric, 25 = text, 1043 = varchar
                    // 1082 = date, 1083 = time, 1114 = timestamp, 1184 = timestamptz
                    match col.type_id {
                        // Boolean types
                        16 => {
                            match string_val.as_str() {
                                "t" | "true" | "1" => serde_json::Value::Bool(true),
                                "f" | "false" | "0" => serde_json::Value::Bool(false),
                                _ => serde_json::Value::String(string_val.clone()),
                            }
                        }
                        // Integer types: int2 (smallint), int4 (integer), int8 (bigint)
                        20 | 21 | 23 => {
                            string_val.parse::<i64>()
                                .map(|n| serde_json::Value::Number(n.into()))
                                .unwrap_or_else(|_| serde_json::Value::String(string_val.clone()))
                        }
                        // Float types: float4, float8
                        700 | 701 => {
                            string_val.parse::<f64>()
                                .ok()
                                .and_then(|f| serde_json::Number::from_f64(f))
                                .map(serde_json::Value::Number)
                                .unwrap_or_else(|| serde_json::Value::String(string_val.clone()))
                        }
                        // Numeric/decimal type
                        1700 => {
                            // Try to parse as integer first, then as float
                            if let Ok(n) = string_val.parse::<i64>() {
                                serde_json::Value::Number(n.into())
                            } else if let Ok(f) = string_val.parse::<f64>() {
                                serde_json::Number::from_f64(f)
                                    .map(serde_json::Value::Number)
                                    .unwrap_or_else(|| serde_json::Value::String(string_val.clone()))
                            } else {
                                serde_json::Value::String(string_val.clone())
                            }
                        }
                        // All other types (text, varchar, timestamp, etc.) remain as strings
                        _ => serde_json::Value::String(string_val.clone()),
                    }
                }
            };
            map.insert(col.name.clone(), json_value);
        }
    }
    
    serde_json::Value::Object(map)
}

/// Trait for output targets that can write replication changes
#[async_trait::async_trait]
pub trait OutputTarget: Send + Sync {
    async fn write_change(&self, change: &Change) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Json,
    JsonPretty,
    Text,
    Debezium,
    Feldera,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "json-pretty" => Ok(OutputFormat::JsonPretty),
            "text" => Ok(OutputFormat::Text),
            "debezium" => Ok(OutputFormat::Debezium),
            "feldera" | "insert-delete" | "insert_delete" => Ok(OutputFormat::Feldera),
            _ => Err(anyhow!("Unknown output format: {}. Valid options: json, json-pretty, text, debezium, feldera", s)),
        }
    }
}

/// Debezium CDC event envelope
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DebeziumEnvelope {
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub source: DebeziumSource,
    pub op: String,
    pub ts_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<DebeziumTransaction>,
}

/// Debezium source metadata
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DebeziumSource {
    pub version: String,
    pub connector: String,
    pub name: String,
    pub ts_ms: i64,
    pub db: String,
    pub schema: String,
    pub table: String,
    pub lsn: String,
}

/// Debezium transaction metadata
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DebeziumTransaction {
    pub id: String,
    pub total_order: i64,
    pub data_collection_order: i64,
}

/// Convert a Change event to Debezium format
fn convert_to_debezium(change: &Change) -> Option<DebeziumEnvelope> {
    use chrono::Utc;
    let ts_ms = Utc::now().timestamp_millis();
    
    match change {
        Change::Insert { schema, table, new_tuple, relation_id } => {
            let after = serde_json::to_value(new_tuple).ok()?;
            Some(DebeziumEnvelope {
                before: None,
                after: Some(after),
                source: DebeziumSource {
                    version: "pgoutput-cmdline-0.1.0".to_string(),
                    connector: "postgresql".to_string(),
                    name: "pgoutput-cmdline".to_string(),
                    ts_ms,
                    db: "postgres".to_string(),
                    schema: schema.clone(),
                    table: table.clone(),
                    lsn: relation_id.to_string(),
                },
                op: "c".to_string(), // c = create/insert
                ts_ms,
                transaction: None,
            })
        }
        Change::Update { schema, table, old_tuple, new_tuple, relation_id } => {
            let before = old_tuple.as_ref().and_then(|t| serde_json::to_value(t).ok());
            let after = serde_json::to_value(new_tuple).ok()?;
            Some(DebeziumEnvelope {
                before,
                after: Some(after),
                source: DebeziumSource {
                    version: "pgoutput-cmdline-0.1.0".to_string(),
                    connector: "postgresql".to_string(),
                    name: "pgoutput-cmdline".to_string(),
                    ts_ms,
                    db: "postgres".to_string(),
                    schema: schema.clone(),
                    table: table.clone(),
                    lsn: relation_id.to_string(),
                },
                op: "u".to_string(), // u = update
                ts_ms,
                transaction: None,
            })
        }
        Change::Delete { schema, table, old_tuple, relation_id } => {
            let before = serde_json::to_value(old_tuple).ok()?;
            Some(DebeziumEnvelope {
                before: Some(before),
                after: None,
                source: DebeziumSource {
                    version: "pgoutput-cmdline-0.1.0".to_string(),
                    connector: "postgresql".to_string(),
                    name: "pgoutput-cmdline".to_string(),
                    ts_ms,
                    db: "postgres".to_string(),
                    schema: schema.clone(),
                    table: table.clone(),
                    lsn: relation_id.to_string(),
                },
                op: "d".to_string(), // d = delete
                ts_ms,
                transaction: None,
            })
        }
        // Begin, Commit, and Relation events are not converted to Debezium format
        _ => None,
    }
}

/// Feldera InsertDelete format event
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FelderaUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<serde_json::Value>,
}

/// Convert a Change event to Feldera InsertDelete format
/// Updates are represented as delete (old) + insert (new) pairs
fn convert_to_feldera(change: &Change) -> Vec<FelderaUpdate> {
    match change {
        Change::Insert { relation_id, new_tuple, .. } => {
            if let Some(columns) = crate::decoder::get_relation_columns(*relation_id) {
                let insert_data = tuple_to_json_with_types(new_tuple, &columns);
                vec![FelderaUpdate {
                    insert: Some(insert_data),
                    delete: None,
                    update: None,
                }]
            } else {
                // Fallback to string conversion if columns not available
                if let Ok(insert_data) = serde_json::to_value(new_tuple) {
                    vec![FelderaUpdate {
                        insert: Some(insert_data),
                        delete: None,
                        update: None,
                    }]
                } else {
                    vec![]
                }
            }
        }
        Change::Update { relation_id, old_tuple, new_tuple, .. } => {
            let columns = crate::decoder::get_relation_columns(*relation_id);
            let mut events = Vec::new();
            
            // First, delete the old state
            if let Some(old) = old_tuple {
                let delete_data = if let Some(ref cols) = columns {
                    tuple_to_json_with_types(old, cols)
                } else {
                    serde_json::to_value(old).unwrap_or(serde_json::Value::Null)
                };
                events.push(FelderaUpdate {
                    insert: None,
                    delete: Some(delete_data),
                    update: None,
                });
            }
            
            // Then, insert the new state
            let insert_data = if let Some(ref cols) = columns {
                tuple_to_json_with_types(new_tuple, cols)
            } else {
                serde_json::to_value(new_tuple).unwrap_or(serde_json::Value::Null)
            };
            events.push(FelderaUpdate {
                insert: Some(insert_data),
                delete: None,
                update: None,
            });
            
            events
        }
        Change::Delete { relation_id, old_tuple, .. } => {
            if let Some(columns) = crate::decoder::get_relation_columns(*relation_id) {
                let delete_data = tuple_to_json_with_types(old_tuple, &columns);
                vec![FelderaUpdate {
                    insert: None,
                    delete: Some(delete_data),
                    update: None,
                }]
            } else {
                // Fallback to string conversion if columns not available
                if let Ok(delete_data) = serde_json::to_value(old_tuple) {
                    vec![FelderaUpdate {
                        insert: None,
                        delete: Some(delete_data),
                        update: None,
                    }]
                } else {
                    vec![]
                }
            }
        }
        // Begin, Commit, and Relation events are not converted to Feldera format
        _ => vec![],
    }
}

/// Stdout output target
pub struct StdoutOutput {
    format: OutputFormat,
}

impl StdoutOutput {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }
}

#[async_trait::async_trait]
impl OutputTarget for StdoutOutput {
    async fn write_change(&self, change: &Change) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string(change)?);
            }
            OutputFormat::JsonPretty => {
                println!("{}", serde_json::to_string_pretty(change)?);
            }
            OutputFormat::Text => {
                print_text_format(change);
            }
            OutputFormat::Debezium => {
                // Convert to Debezium format and print only data events (not Begin/Commit/Relation)
                if let Some(debezium_event) = convert_to_debezium(change) {
                    println!("{}", serde_json::to_string(&debezium_event)?);
                }
            }
            OutputFormat::Feldera => {
                // Convert to Feldera InsertDelete format
                // Updates produce two events: delete + insert
                for feldera_event in convert_to_feldera(change) {
                    println!("{}", serde_json::to_string(&feldera_event)?);
                }
            }
        }
        Ok(())
    }
}

/// NATS JetStream output target
pub struct NatsOutput {
    context: jetstream::Context,
    subject_prefix: String,
}

impl NatsOutput {
    pub async fn new(server: &str, stream_name: &str, subject_prefix: String) -> Result<Self> {
        // Connect to NATS server
        let client = async_nats::connect(server).await
            .map_err(|e| anyhow!("Failed to connect to NATS server at {}: {}", server, e))?;
        
        // Create JetStream context
        let jetstream = jetstream::new(client);
        
        // Create or get the stream
        let stream_subjects = format!("{}.*.*.*", subject_prefix);
        match jetstream.get_stream(stream_name).await {
            Ok(_stream) => {
                eprintln!("Using existing NATS stream: {}", stream_name);
                Ok(Self {
                    context: jetstream,
                    subject_prefix,
                })
            }
            Err(_) => {
                // Stream doesn't exist, create it
                eprintln!("Creating NATS stream: {}", stream_name);
                jetstream.create_stream(jetstream::stream::Config {
                    name: stream_name.to_string(),
                    subjects: vec![stream_subjects],
                    max_messages: 1_000_000,
                    max_bytes: 1_000_000_000, // 1GB
                    ..Default::default()
                }).await
                    .map_err(|e| anyhow!("Failed to create NATS stream: {}", e))?;
                
                Ok(Self {
                    context: jetstream,
                    subject_prefix,
                })
            }
        }
    }

    fn get_subject(&self, change: &Change) -> String {
        match change {
            Change::Begin { .. } => format!("{}.transactions.begin.event", self.subject_prefix),
            Change::Commit { .. } => format!("{}.transactions.commit.event", self.subject_prefix),
            Change::Relation { schema, table, .. } => {
                format!("{}.{}.{}.relation", self.subject_prefix, schema, table)
            }
            Change::Insert { schema, table, .. } => {
                format!("{}.{}.{}.insert", self.subject_prefix, schema, table)
            }
            Change::Update { schema, table, .. } => {
                format!("{}.{}.{}.update", self.subject_prefix, schema, table)
            }
            Change::Delete { schema, table, .. } => {
                format!("{}.{}.{}.delete", self.subject_prefix, schema, table)
            }
        }
    }
}

#[async_trait::async_trait]
impl OutputTarget for NatsOutput {
    async fn write_change(&self, change: &Change) -> Result<()> {
        let subject = self.get_subject(change);
        let payload = serde_json::to_vec(change)?;
        
        self.context.publish(subject.clone(), payload.into())
            .await
            .map_err(|e| anyhow!("Failed to publish to NATS subject {}: {}", subject, e))?;
        
        Ok(())
    }
}

/// Feldera HTTP output target
pub struct FelderaOutput {
    client: Client,
    ingress_url: String,
}

impl FelderaOutput {
    pub async fn new(
        base_url: &str,
        pipeline: &str,
        table: &str,
        api_key: Option<&str>,
    ) -> Result<Self> {
        // Build HTTP client with optional authentication
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
        
        if let Some(key) = api_key {
            let auth_value = header::HeaderValue::from_str(&format!("Bearer {}", key))
                .map_err(|e| anyhow!("Invalid API key: {}", e))?;
            headers.insert(header::AUTHORIZATION, auth_value);
        }
        
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
        
        // Build ingress URL with format and update_format parameters
        let base = base_url.trim_end_matches('/');
        let encoded_pipeline = urlencoding::encode(pipeline);
        let encoded_table = urlencoding::encode(table);
        let ingress_url = format!(
            "{}/v0/pipelines/{}/ingress/{}?format=json&update_format=insert_delete&array=true",
            base, encoded_pipeline, encoded_table
        );
        
        Ok(Self {
            client,
            ingress_url,
        })
    }
}

#[async_trait::async_trait]
impl OutputTarget for FelderaOutput {
    async fn write_change(&self, change: &Change) -> Result<()> {
        // Convert to Feldera InsertDelete format
        let feldera_events = convert_to_feldera(change);
        
        // Skip non-data events (Begin, Commit, Relation)
        if feldera_events.is_empty() {
            return Ok(());
        }
        
        // When using array=true, Feldera expects ALL events as JSON arrays
        // even single INSERT/DELETE operations
        let payload = serde_json::to_string(&feldera_events)?;
        
        // Send HTTP POST request to Feldera ingress API
        let response = self.client
            .post(&self.ingress_url)
            .body(payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send data to Feldera: {}", e))?;
        
        // Check for successful response
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "<no body>".to_string());
            return Err(anyhow!(
                "Feldera ingress API returned error status {}: {}",
                status,
                error_body
            ));
        }
        
        Ok(())
    }
}

/// Composite output that writes to multiple targets
pub struct CompositeOutput {
    targets: Vec<Arc<dyn OutputTarget>>,
}

impl CompositeOutput {
    pub fn new(targets: Vec<Arc<dyn OutputTarget>>) -> Self {
        Self { targets }
    }
}

#[async_trait::async_trait]
impl OutputTarget for CompositeOutput {
    async fn write_change(&self, change: &Change) -> Result<()> {
        for target in &self.targets {
            target.write_change(change).await?;
        }
        Ok(())
    }
}

// Kept for backward compatibility (currently unused)
#[allow(dead_code)]
pub fn print_change(change: &Change, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(change)?);
        }
        OutputFormat::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(change)?);
        }
        OutputFormat::Text => {
            print_text_format(change);
        }
        OutputFormat::Debezium => {
            if let Some(debezium_event) = convert_to_debezium(change) {
                println!("{}", serde_json::to_string(&debezium_event)?);
            }
        }
        OutputFormat::Feldera => {
            for feldera_event in convert_to_feldera(change) {
                println!("{}", serde_json::to_string(&feldera_event)?);
            }
        }
    }
    Ok(())
}

fn print_text_format(change: &Change) {
    match change {
        Change::Begin { lsn, timestamp, xid } => {
            println!("BEGIN [LSN: {}, XID: {}, Time: {}]", lsn, xid, timestamp);
        }
        Change::Commit { lsn, timestamp } => {
            println!("COMMIT [LSN: {}, Time: {}]", lsn, timestamp);
        }
        Change::Relation { relation_id, schema, table, columns } => {
            println!("RELATION [{}.{} (ID: {})]", schema, table, relation_id);
            println!("  Columns:");
            for col in columns {
                println!("    - {} (type_id: {}, flags: {})", col.name, col.type_id, col.flags);
            }
        }
        Change::Insert { relation_id, schema, table, new_tuple } => {
            println!("INSERT into {}.{} (ID: {})", schema, table, relation_id);
            println!("  New values:");
            for (key, value) in new_tuple {
                match value {
                    Some(v) => println!("    {}: {}", key, v),
                    None => println!("    {}: NULL", key),
                }
            }
        }
        Change::Update { relation_id, schema, table, old_tuple, new_tuple } => {
            println!("UPDATE {}.{} (ID: {})", schema, table, relation_id);
            if let Some(old) = old_tuple {
                println!("  Old values:");
                for (key, value) in old {
                    match value {
                        Some(v) => println!("    {}: {}", key, v),
                        None => println!("    {}: NULL", key),
                    }
                }
            }
            println!("  New values:");
            for (key, value) in new_tuple {
                match value {
                    Some(v) => println!("    {}: {}", key, v),
                    None => println!("    {}: NULL", key),
                }
            }
        }
        Change::Delete { relation_id, schema, table, old_tuple } => {
            println!("DELETE from {}.{} (ID: {})", schema, table, relation_id);
            println!("  Old values:");
            for (key, value) in old_tuple {
                match value {
                    Some(v) => println!("    {}: {}", key, v),
                    None => println!("    {}: NULL", key),
                }
            }
        }
    }
}

/// Public test helper to expose convert_to_debezium for testing
#[doc(hidden)]
pub fn convert_to_debezium_test(change: &Change) -> Option<DebeziumEnvelope> {
    convert_to_debezium(change)
}

/// Public test helper to expose convert_to_feldera for testing
#[doc(hidden)]
pub fn convert_to_feldera_test(change: &Change) -> Vec<FelderaUpdate> {
    convert_to_feldera(change)
}
