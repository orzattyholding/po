use futures_util::{SinkExt, StreamExt};
use po_node::Po;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n==============================================");
    println!("🚀 ORZATTY PROTOCOL (PO) vs WEBSOCKET BENCHMARK");
    println!("   PO (Encrypted, UDP) vs WS (Plain, TCP)");
    println!("==============================================\n");

    let num_pings = 5000;
    let payload_size = 10 * 1024 * 1024; // 10 MB

    println!("Starting WebSocket Server on 0.0.0.0:8080...");
    tokio::spawn(async move {
        run_ws_server("0.0.0.0:8080").await.unwrap();
    });

    println!("Starting PO Server on 0.0.0.0:4434...");
    tokio::spawn(async move {
        run_po_server(4434).await.unwrap();
    });

    // Wait for servers to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // ═══════════════════════════════════════════════════════
    // TEST 1: CONNECTION SETUP TIME
    // ═══════════════════════════════════════════════════════
    println!("----------------------------------------------");
    println!("🧪 TEST 1: Connection Setup Time");

    // WS
    let ws_start = Instant::now();
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut ws_stream, _) = tokio_tungstenite::client_async("ws://127.0.0.1:8080", stream).await?;
    let ws_setup_time = ws_start.elapsed();
    println!("   WebSocket Setup Time:  {:?}", ws_setup_time);

    // PO
    let po_start = Instant::now();
    let mut po_stream = Po::connect("127.0.0.1:4434").await?;
    let po_setup_time = po_start.elapsed();
    println!(
        "   PO Setup Time:         {:?} (incluye E2EE handshake)",
        po_setup_time
    );

    let setup_ratio = ws_setup_time.as_secs_f64() / po_setup_time.as_secs_f64();
    if po_setup_time < ws_setup_time {
        println!(
            "   ✅ PO {:.1}x más rápido (con cifrado incluido)",
            setup_ratio
        );
    } else {
        println!(
            "   ⚠️  WS {:.1}x más rápido (pero sin cifrado)",
            1.0 / setup_ratio
        );
    }

    // ═══════════════════════════════════════════════════════
    // TEST 2: PING / PONG LATENCY
    // ═══════════════════════════════════════════════════════
    println!("----------------------------------------------");
    println!(
        "🧪 TEST 2: Latency Round-Trip Time (RTT) — {} pings",
        num_pings
    );

    let ping_payload = b"PING_PONG_TEST_64_BYTES_DATA_JUST_TO_HAVE_SOME_PAYLOAD_SIZE_1234".to_vec();

    // WS Ping
    let mut ws_ping_time = Duration::default();
    for _ in 0..num_pings {
        let start = Instant::now();
        ws_stream
            .send(Message::Binary(ping_payload.clone()))
            .await?;
        let _ = ws_stream.next().await.unwrap()?;
        ws_ping_time += start.elapsed();
    }
    let ws_avg_latency = ws_ping_time.as_secs_f64() * 1000.0 / (num_pings as f64);
    println!(
        "   WebSocket Avg Latency: {:.3} ms (plaintext)",
        ws_avg_latency
    );

    // PO Ping
    let mut po_ping_time = Duration::default();
    for _ in 0..num_pings {
        let start = Instant::now();
        po_stream.send(&ping_payload).await?;
        let _ = po_stream.recv().await?.unwrap();
        po_ping_time += start.elapsed();
    }
    let po_avg_latency = po_ping_time.as_secs_f64() * 1000.0 / (num_pings as f64);
    println!(
        "   PO Avg Latency:        {:.3} ms (encrypted)",
        po_avg_latency
    );

    let latency_overhead = po_avg_latency / ws_avg_latency;
    println!(
        "   📊 Encryption cost:    {:.1}x ({:.3} ms overhead per RTT)",
        latency_overhead,
        po_avg_latency - ws_avg_latency
    );

    // ═══════════════════════════════════════════════════════
    // TEST 3: THROUGHPUT (BULK TRANSFER, 256KB chunks)
    // ═══════════════════════════════════════════════════════
    let chunk_size = 256 * 1024; // 256KB — larger chunks reduce per-frame overhead

    println!("----------------------------------------------");
    println!(
        "🧪 TEST 3: Throughput ({} MB, {}KB chunks)",
        payload_size / (1024 * 1024),
        chunk_size / 1024
    );

    // Generate deterministic payload
    let mut bulk_data = vec![0u8; payload_size];
    for (i, byte) in bulk_data.iter_mut().enumerate().take(payload_size) {
        *byte = (i % 256) as u8;
    }

    // WS Throughput
    let ws_start = Instant::now();
    ws_stream
        .send(Message::Binary(b"START_BULK_WS".to_vec()))
        .await?;
    for chunk in bulk_data.chunks(chunk_size) {
        ws_stream.send(Message::Binary(chunk.to_vec())).await?;
    }
    let _ = ws_stream.next().await.unwrap()?;
    let ws_tp_time = ws_start.elapsed();
    let ws_mbps = (payload_size as f64 / 1_048_576.0) / ws_tp_time.as_secs_f64();
    println!(
        "   WebSocket:  {:>9?}  ({:.1} MB/s, plaintext)",
        ws_tp_time, ws_mbps
    );

    // PO Throughput
    let po_start = Instant::now();
    po_stream.send(b"START_BULK_PO").await?;
    for chunk in bulk_data.chunks(chunk_size) {
        po_stream.send(chunk).await?;
    }
    let _ = po_stream.recv().await?.unwrap();
    let po_tp_time = po_start.elapsed();
    let po_mbps = (payload_size as f64 / 1_048_576.0) / po_tp_time.as_secs_f64();
    println!(
        "   PO:         {:>9?}  ({:.1} MB/s, E2EE encrypted)",
        po_tp_time, po_mbps
    );

    let tp_ratio = ws_mbps / po_mbps;
    let num_chunks = payload_size.div_ceil(chunk_size);
    println!(
        "   📊 Throughput ratio:   {:.1}x (WS faster, {} chunks × encrypt+frame)",
        tp_ratio, num_chunks
    );

    // ═══════════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════════
    println!("==============================================");
    println!("📋 SUMMARY");
    println!("==============================================");
    println!("   PO provides full E2EE (Ed25519 + X25519 + ChaCha20)");
    println!("   WebSocket sends plaintext (no encryption)");
    println!("----------------------------------------------");
    println!(
        "   Setup:      PO {:>8?} vs WS {:>8?}",
        po_setup_time, ws_setup_time
    );
    println!(
        "   Latency:    PO {:.3}ms    vs WS {:.3}ms",
        po_avg_latency, ws_avg_latency
    );
    println!(
        "   Throughput: PO {:.1} MB/s  vs WS {:.1} MB/s",
        po_mbps, ws_mbps
    );
    println!("==============================================\n");

    // Clean shutdown
    let _ = ws_stream.close(None).await;
    let _ = po_stream.close().await;

    Ok(())
}

async fn run_ws_server(addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(&addr).await?;
    if let Ok((stream, _)) = listener.accept().await {
        let mut ws_stream = tokio_tungstenite::accept_async(stream).await?;

        let mut bulk_received = 0;
        let target_bulk = 10 * 1024 * 1024;

        while let Some(msg) = ws_stream.next().await {
            let msg = msg?;
            if msg.is_binary() {
                let data = msg.into_data();
                if data.starts_with(b"START_BULK_WS") {
                    bulk_received = 0;
                } else if data.starts_with(b"PING") {
                    ws_stream.send(Message::Binary(data)).await?;
                } else {
                    bulk_received += data.len();
                    if bulk_received >= target_bulk {
                        ws_stream
                            .send(Message::Binary(b"ACK_BULK".to_vec()))
                            .await?;
                        bulk_received = 0;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn run_po_server(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(mut po) = Po::bind(port).await {
        let mut bulk_received = 0;
        let target_bulk = 10 * 1024 * 1024;

        while let Ok(Some((_, data))) = po.recv().await {
            if data.starts_with(b"START_BULK_PO") {
                bulk_received = 0;
            } else if data.starts_with(b"PING") {
                po.send(&data).await?;
            } else {
                bulk_received += data.len();
                if bulk_received >= target_bulk {
                    po.send(b"ACK_BULK").await?;
                    bulk_received = 0;
                }
            }
        }
    }
    Ok(())
}
