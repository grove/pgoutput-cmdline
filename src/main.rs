mod replication;
mod decoder;
mod output;

use clap::Parser;
use anyhow::Result;
use std::sync::Arc;
use output::OutputTarget;

#[derive(Parser, Debug)]
#[command(name = "pgoutput-cmdline")]
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

    /// NATS server URL (e.g., "nats://localhost:4222")
    #[arg(long)]
    nats_server: Option<String>,

    /// NATS JetStream stream name
    #[arg(long, default_value = "postgres_replication")]
    nats_stream: String,

    /// NATS subject prefix (e.g., "postgres" will create subjects like "postgres.public.users.insert")
    #[arg(long, default_value = "postgres")]
    nats_subject_prefix: String,
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

    // Build output targets
    let mut targets: Vec<Arc<dyn OutputTarget>> = Vec::new();
    
    // Always add stdout output
    let stdout_output = output::StdoutOutput::new(output::OutputFormat::from_str(&args.format)?);
    targets.push(Arc::new(stdout_output));
    
    // Add NATS output if configured
    if let Some(nats_server) = &args.nats_server {
        eprintln!("Connecting to NATS server: {}", nats_server);
        eprintln!("Stream: {}", args.nats_stream);
        eprintln!("Subject prefix: {}\n", args.nats_subject_prefix);
        
        let nats_output = output::NatsOutput::new(
            nats_server,
            &args.nats_stream,
            args.nats_subject_prefix.clone(),
        ).await?;
        targets.push(Arc::new(nats_output));
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
