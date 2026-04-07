const { PoClient } = require('./index.js');

async function main() {
  console.log("🚀 Starting Protocol Orzatty Node.js Binding Test\n");

  console.log("   [Server&Client] Initializing connection...");
  const [server, client] = await Promise.all([
    PoClient.bind(5544),
    // Give bind a tiny fraction of a second to start listening before connect
    new Promise(res => setTimeout(res, 200)).then(() => PoClient.connect("127.0.0.1:5544"))
  ]);
  
  console.log(`   [Server] Node ID: ${server.nodeId}`);
  console.log(`   [Client] Node ID: ${client.nodeId}`);

  // 3. Client sends a message
  const msg = "Hello from Node.js land over E2EE UDP!";
  console.log(`\n   [Client] Sending: "${msg}"`);
  await client.send(Buffer.from(msg));

  // 4. Server receives it
  const data = await server.recv();
  console.log(`   [Server] Received: "${data.toString('utf8')}"`);

  // 5. Cleanup
  console.log("\n   Closing connections...");
  await client.close();
  await server.close();
  
  console.log("✅ Node.js Bindings test passed!");
}

main().catch(console.error);
