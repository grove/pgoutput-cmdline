use anyhow::{anyhow, Result};
use crate::decoder::Change;
use serde_json;
use async_nats::jetstream;
use std::sync::Arc;

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
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "json-pretty" => Ok(OutputFormat::JsonPretty),
            "text" => Ok(OutputFormat::Text),
            _ => Err(anyhow!("Unknown output format: {}. Valid options: json, json-pretty, text", s)),
        }
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
