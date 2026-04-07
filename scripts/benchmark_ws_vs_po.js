const { PoClient } = require('../crates/po-napi/index.js');
const WebSocket = require('ws');
const { performance } = require('perf_hooks');
const fs = require('fs');

const MSG_COUNT = 50000;
const BATCH_SIZE = 500; // Send 500 messages per batch = 100 batches total
const payload = Buffer.alloc(1024, 'a'); // 1KB message

// ── PO con sendBatch (optimizado) ──────────────────────────────────
async function runPO_Batch() {
    console.log("-> Telemetría: Protocolo Orzatty (sendBatch, 1 encrypt per batch)...");
    const [server, client] = await Promise.all([
        PoClient.bind(5547),
        new Promise(res => setTimeout(res, 200)).then(() => PoClient.connect("127.0.0.1:5547"))
    ]);

    // Receiver: server reads raw frames (each batch is 1 recv)
    let totalBatchesReceived = 0;
    const expectedBatches = MSG_COUNT / BATCH_SIZE;
    const rxPromise = (async () => {
        while (totalBatchesReceived < expectedBatches) {
            await server.recv();
            totalBatchesReceived++;
        }
    })();

    // Sender: send in batches
    const start = performance.now();
    const batch = [];
    for (let i = 0; i < BATCH_SIZE; i++) {
        batch.push(payload);
    }

    const batchCount = MSG_COUNT / BATCH_SIZE;
    for (let b = 0; b < batchCount; b++) {
        await client.sendBatch(batch);
    }

    await rxPromise;
    const end = performance.now();

    await client.close();
    await server.close();

    const timeMs = end - start;
    return {
        name: "Protocol Orzatty (PO) - sendBatch QUIC/E2EE",
        timeMs,
        messages: MSG_COUNT,
        msgPerSec: (MSG_COUNT / (timeMs / 1000))
    };
}

// ── PO individual send (baseline para comparar) ────────────────────
async function runPO_Individual() {
    console.log("-> Telemetría: Protocolo Orzatty (send individual, 1 encrypt per msg)...");
    const [server, client] = await Promise.all([
        PoClient.bind(5548),
        new Promise(res => setTimeout(res, 200)).then(() => PoClient.connect("127.0.0.1:5548"))
    ]);

    let received = 0;
    const rxPromise = (async () => {
        while (received < MSG_COUNT) {
            await server.recv();
            received++;
        }
    })();

    const start = performance.now();
    for (let i = 0; i < MSG_COUNT; i++) {
        await client.send(payload);
    }
    await rxPromise;
    const end = performance.now();

    await client.close();
    await server.close();

    const timeMs = end - start;
    return {
        name: "Protocol Orzatty (PO) - individual send",
        timeMs,
        messages: MSG_COUNT,
        msgPerSec: (MSG_COUNT / (timeMs / 1000))
    };
}

// ── WebSockets ─────────────────────────────────────────────────────
async function runWS() {
    console.log("-> Telemetría: WebSockets (TCP plaintext)...");
    const wss = new WebSocket.Server({ port: 5549 });
    return new Promise((resolve) => {
        let received = 0;
        let start;
        wss.on('connection', ws => {
            ws.on('message', () => {
                received++;
                if (received === MSG_COUNT) {
                    const end = performance.now();
                    const timeMs = end - start;
                    resolve({
                        name: "WebSockets (WS) - TCP plaintext",
                        timeMs,
                        messages: MSG_COUNT,
                        msgPerSec: (MSG_COUNT / (timeMs / 1000))
                    });
                    ws.close();
                    wss.close();
                }
            });
        });

        const client = new WebSocket('ws://127.0.0.1:5549');
        client.on('open', () => {
            start = performance.now();
            for (let i = 0; i < MSG_COUNT; i++) {
                client.send(payload);
            }
        });
    });
}

(async () => {
    console.log(`\n======================================================`);
    console.log(`📡 BENCHMARK FÍSICO: PO vs WebSockets`);
    console.log(`======================================================`);
    console.log(`Carga: ${MSG_COUNT} mensajes de ${(payload.length / 1024).toFixed(2)} KB cada uno.`);
    console.log(`Batch size PO: ${BATCH_SIZE} msgs/batch`);
    console.log(`------------------------------------------------------\n`);

    await new Promise(r => setTimeout(r, 500));
    const poBatch = await runPO_Batch();

    await new Promise(r => setTimeout(r, 500));
    const poIndiv = await runPO_Individual();

    await new Promise(r => setTimeout(r, 500));
    const wsResult = await runWS();

    const results = [poBatch, poIndiv, wsResult];
    fs.writeFileSync('benchmark_results.json', JSON.stringify(results, null, 2));

    const factorBatch = poBatch.msgPerSec / wsResult.msgPerSec;
    const speedup = poBatch.msgPerSec / poIndiv.msgPerSec;

    console.log("\n======================================================");
    console.log("📊 RESULTADOS FINALES DE RENDIMIENTO");
    console.log("======================================================");
    console.log(`🔥 PO sendBatch (E2EE):   ${poBatch.timeMs.toFixed(2)} ms | ${poBatch.msgPerSec.toFixed(0)} msg/sec`);
    console.log(`⚡ PO individual (E2EE):  ${poIndiv.timeMs.toFixed(2)} ms | ${poIndiv.msgPerSec.toFixed(0)} msg/sec`);
    console.log(`🐢 WebSockets (TCP raw):  ${wsResult.timeMs.toFixed(2)} ms | ${wsResult.msgPerSec.toFixed(0)} msg/sec`);
    console.log("------------------------------------------------------");
    console.log(`📈 sendBatch vs individual: ${speedup.toFixed(2)}x mejora`);
    console.log(`📈 PO Batch vs WebSockets:  ${factorBatch.toFixed(2)}x`);
    console.log("======================================================\n");
})();
