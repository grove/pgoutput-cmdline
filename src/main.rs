mod replication;
mod decoder;
mod output;

use clap::Parser;
use anyhow::Result;
use std::sync::Arc;
use output::OutputTarget;

#[derive(Parser, Debug)]
#[command(name = "pgoutput-stream")]
#[command(about = "Stream PostgreSQL logical replication changes to stdout", long_about = None)]
struct Args {
    /// PostgreSQL connection string (e.g., "host=localhost user=postgres dbname=mydb")
    #[arg(short, long)]
    connection: String,

    /// Replication slot name
    #[arg(short, long)]
    slot: String,

    /// Publication name
    #[arg(short, long)]
    publication: String,

    /// Output format: json, json-pretty, text, debezium, or feldera
    #[arg(short, long, default_value = "json")]
    format: String,

    /// Create replication slot if it doesn't exist
    #[arg(long)]
    create_slot: bool,

    /// Starting LSN (Log Sequence Number) to stream from
    #[arg(long)]
    start_lsn: Option<String>,

    /// Output target(s): stdout, nats, feldera (comma-separated for multiple)
    #[arg(short, long, default_value = "stdout")]
    target: String,

    /// NATS server URL (required when target includes 'nats')
    #[arg(long)]
    nats_server: Option<String>,

    /// NATS JetStream stream name
    #[arg(long, default_value = "postgres_replication")]
    nats_stream: String,

    /// NATS subject prefix (e.g., "postgres" will create subjects like "postgres.public.users.insert")
    #[arg(long, default_value = "postgres")]
    nats_subject_prefix: String,

    /// Feldera base URL (required when target includes 'feldera')
    #[arg(long)]
    feldera_url: Option<String>,

    /// Feldera pipeline name (required when target includes 'feldera')
    #[arg(long)]
    feldera_pipeline: Option<String>,

    /// Feldera table names in schema_table format (e.g., 'public_users,analytics_orders').
    /// Optional; if omitted, routes all tables dynamically.
    #[arg(long)]
    feldera_tables: Option<String>,

    /// Feldera API key for authentication (optional)
    #[arg(long)]
    feldera_api_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    eprintln!("Connecting to PostgreSQL...");
    eprintln!("Slot: {}", args.slot);
    eprintln!("Publication: {}", args.publication);
    eprintln!("Output format: {}", args.format);

    // Initialize replication stream
    let mut stream = replication::ReplicationStream::new(
        &args.connection,
        &args.slot,
        &args.publication,
        args.create_slot,
        args.start_lsn,
    )
    .await?;

    eprintln!("Starting replication stream...\n");

    // Build output targets based on --target option
    let mut targets: Vec<Arc<dyn OutputTarget>> = Vec::new();
    let target_list: Vec<&str> = args.target.split(',').map(|s| s.trim()).collect();
    
    eprintln!("Output targets: {}", args.target);
    
    for target in target_list {
        match target {
            "stdout" => {
                let stdout_output = output::StdoutOutput::new(output::OutputFormat::from_str(&args.format)?);
                targets.push(Arc::new(stdout_output));
                eprintln!("  - stdout (format: {})", args.format);
            }
            "nats" => {
                let nats_server = args.nats_server.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("--nats-server is required when target includes 'nats'"))?;
                
                eprintln!("  - NATS JetStream:");
                eprintln!("      Server: {}", nats_server);
                eprintln!("      Stream: {}", args.nats_stream);
                eprintln!("      Subject prefix: {}", args.nats_subject_prefix);
                
                let nats_output = output::NatsOutput::new(
                    nats_server,
                    &args.nats_stream,
                    args.nats_subject_prefix.clone(),
                ).await?;
                targets.push(Arc::new(nats_output));
            }
            "feldera" => {
                let feldera_url = args.feldera_url.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("--feldera-url is required when target includes 'feldera'"))?;
                let feldera_pipeline = args.feldera_pipeline.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("--feldera-pipeline is required when target includes 'feldera'"))?;
                
                // Parse comma-separated table list if provided
                let allowed_tables = args.feldera_tables.as_ref().map(|tables_str| {
                    tables_str
                        .split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect::<Vec<String>>()
                });
                
                eprintln!("  - Feldera HTTP Connector:");
                eprintln!("      URL: {}", feldera_url);
                eprintln!("      Pipeline: {}", feldera_pipeline);
                if let Some(ref tables) = allowed_tables {
                    eprintln!("      Tables: {}", tables.join(", "));
                } else {
                    eprintln!("      Tables: all (dynamic routing)");
                }
                if args.feldera_api_key.is_some() {
                    eprintln!("      API Key: [configured]");
                }
                
                let feldera_output = output::FelderaOutput::new(
                    feldera_url,
                    feldera_pipeline,
                    allowed_tables,
                    args.feldera_api_key.as_deref(),
                ).await?;
                targets.push(Arc::new(feldera_output));
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown target '{}'. Valid targets: stdout, nats, feldera", target));
            }
        }
    }
    
    eprintln!();
    
    if targets.is_empty() {
        return Err(anyhow::anyhow!("At least one output target must be specified"));
    }
    
    // Create composite output
    let output_handler = output::CompositeOutput::new(targets);

    // Set up graceful shutdown
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        eprintln!("\nReceived shutdown signal, stopping...");
        let _ = shutdown_tx.send(()).await;
    });

    // Process replication stream
    loop {
        tokio::select! {
            result = stream.next_message() => {
                match result {
                    Ok(Some(change)) => {
                        output_handler.write_change(&change).await?;
                    }
                    Ok(None) => {
                        // Keep-alive or no data
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error reading replication stream: {}", e);
                        return Err(e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                eprintln!("Shutting down gracefully...");
                break;
            }
        }
    }

    Ok(())
}
