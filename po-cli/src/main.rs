use clap::{Parser, Subcommand};
use po_node::Po;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "po",
    about = "Protocol Orzatty — Encrypted P2P communication",
    version,
    author = "Dylan Orzatty <dylan@orzatty.com>"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Listen for incoming connections
    Listen {
        /// Port to listen on
        #[arg(short, long, default_value = "4433")]
        port: u16,
    },

    /// Connect to a remote PO node
    Connect {
        /// Address to connect to (e.g., 192.168.1.5:4433)
        addr: String,
    },

    /// Interactive chat mode (listen + auto-reply)
    Chat {
        /// Port to listen on, or address to connect to
        target: String,
    },

    /// Show your node identity
    Identity,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Listen { port } => {
            println!("🔒 Protocol Orzatty — Listening on port {port}");
            println!("   Waiting for incoming connection...\n");

            let mut po = Po::bind(port).await?;
            println!("✅ Encrypted session established!");
            println!("   Peer: {}\n", po.peer_node_id().unwrap_or_default());

            // Receive loop
            while let Some((_channel, data)) = po.recv().await? {
                let msg = String::from_utf8_lossy(&data);
                println!("📩 {msg}");
            }

            println!("\n🔌 Connection closed.");
        }

        Commands::Connect { addr } => {
            println!("🔒 Protocol Orzatty — Connecting to {addr}");

            let mut po = Po::connect(&addr).await?;
            println!("✅ Encrypted session established!");
            println!("   Peer: {}\n", po.peer_node_id().unwrap_or_default());

            // Interactive send loop
            let stdin = tokio::io::stdin();
            let reader = tokio::io::BufReader::new(stdin);
            use tokio::io::AsyncBufReadExt;
            let mut lines = reader.lines();

            println!("Type messages (Ctrl+C to quit):");
            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() {
                    continue;
                }
                po.send(line.as_bytes()).await?;
                println!("📤 Sent: {line}");
            }

            po.close().await?;
        }

        Commands::Chat { target } => {
            // Determine if target is a port number (listen) or address (connect)
            let is_port = target.parse::<u16>().is_ok();

            let mut po = if is_port {
                let port: u16 = target.parse()?;
                println!("🔒 Protocol Orzatty — Chat mode (listening on port {port})");
                println!("   Waiting for peer...\n");
                Po::bind(port).await?
            } else {
                println!("🔒 Protocol Orzatty — Chat mode (connecting to {target})");
                Po::connect(&target).await?
            };

            println!("✅ Encrypted session established!");
            println!("   Your ID:  {}", po.node_id());
            println!("   Peer ID:  {}", po.peer_node_id().unwrap_or_default());
            println!("\nType messages (Ctrl+C to quit):\n");

            // Spawn receiver task
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

            // Read stdin in a separate task
            tokio::spawn(async move {
                let stdin = tokio::io::stdin();
                let reader = tokio::io::BufReader::new(stdin);
                use tokio::io::AsyncBufReadExt;
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    if tx.send(line).await.is_err() {
                        break;
                    }
                }
            });

            loop {
                tokio::select! {
                    // User typed something
                    Some(line) = rx.recv() => {
                        if !line.is_empty() {
                            po.send(line.as_bytes()).await?;
                        }
                    }
                    // Received from peer
                    result = po.recv() => {
                        match result? {
                            Some((_ch, data)) => {
                                let msg = String::from_utf8_lossy(&data);
                                println!("\r💬 Peer: {msg}");
                            }
                            None => {
                                println!("\n🔌 Peer disconnected.");
                                break;
                            }
                        }
                    }
                }
            }
        }

        Commands::Identity => {
            let identity = po_crypto::identity::Identity::generate();
            println!("🔑 Protocol Orzatty — Node Identity\n");
            println!("   Node ID:    {}", identity.node_id().to_hex());
            println!("   Short ID:   {}", identity.node_id().short());
            println!(
                "   Public Key:  {}",
                hex_encode(&identity.public_key_bytes())
            );
            println!(
                "\n   ℹ️  A new identity is generated each time unless you persist the secret key."
            );
        }
    }

    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
