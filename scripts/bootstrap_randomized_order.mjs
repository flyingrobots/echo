import fs from "node:fs";

// Simplified seeded random number generator (Xorshift32) for deterministic scenario generation
function xorshift32(state) {
    let x = state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    return x >>> 0; // unsigned 32-bit integer
}

class RNG {
    constructor(seed) {
        this.state = seed || 1;
    }
    next() {
        this.state = xorshift32(this.state);
        return this.state;
    }
    range(min, max) {
        return min + (this.next() % (max - min));
    }
    // Fisher-Yates shuffle
    shuffle(array) {
        for (let i = array.length - 1; i > 0; i--) {
            const j = this.range(0, i + 1);
            [array[i], array[j]] = [array[j], array[i]];
        }
        return array;
    }
}

// Op Constants
function packFrame(opId, payload) {
    const header = Buffer.alloc(12);
    header.write("EINT", 0);
    header.writeUInt32LE(opId, 4);
    header.writeUInt32LE(payload.length, 8);
    return Buffer.concat([header, payload]);
}

const OP_ROUTE_PUSH = 2216217860;
const OP_TOAST = 4255241313;

function generateLog(seed, outPath, schemaHashHex) {
    const rng = new RNG(seed);
    const frames = [];

    // Set of operations to perform (order-independent set)
    // 100 RoutePush ops with unique paths
    // 100 Toast ops with unique messages
    
    const ops = [];
    for (let i = 0; i < 100; i++) {
        const path = `/node/${i}`;
        const strBytes = Buffer.from(path, 'utf8');
        const payload = Buffer.alloc(4 + strBytes.length);
        payload.writeUInt32LE(strBytes.length, 0);
        strBytes.copy(payload, 4);
        ops.push(packFrame(OP_ROUTE_PUSH, payload));
    }
    for (let i = 0; i < 100; i++) {
        const msg = `Toast ${i}`;
        const strBytes = Buffer.from(msg, 'utf8');
        const payload = Buffer.alloc(4 + strBytes.length);
        payload.writeUInt32LE(strBytes.length, 0);
        strBytes.copy(payload, 4);
        ops.push(packFrame(OP_TOAST, payload));
    }

    // Shuffle the operations
    rng.shuffle(ops);
    
    // ELOG Header
    const MAGIC = Buffer.from("ELOG");
    const VERSION = Buffer.alloc(2); VERSION.writeUInt16LE(1);
    const FLAGS = Buffer.alloc(2); FLAGS.writeUInt16LE(0);
    const RESERVED = Buffer.alloc(8);

    const fd = fs.openSync(outPath, "w");
    fs.writeSync(fd, MAGIC);
    fs.writeSync(fd, VERSION);
    fs.writeSync(fd, FLAGS);
    fs.writeSync(fd, Buffer.from(schemaHashHex, "hex"));
    fs.writeSync(fd, RESERVED);
    
    for (const frame of ops) {
        const len = Buffer.alloc(4);
        len.writeUInt32LE(frame.byteLength);
        fs.writeSync(fd, len);
        fs.writeSync(fd, frame);
    }
    fs.closeSync(fd);
    console.log(`Wrote ${outPath}`);
}

// Main
const codecs = fs.readFileSync("crates/flyingrobots-echo-wasm/src/generated/codecs.rs", "utf8");
const match = codecs.match(/pub const SCHEMA_HASH: &str = "([0-9a-f]+)";/);
if (!match) throw new Error("Could not find SCHEMA_HASH in codecs.rs");
const schemaHash = match[1];

// Generate 3 seeds
for (let i = 1; i <= 3; i++) {
    const seed = 1000 + i;
    const out = `testdata/dind/050_randomized_order_small_seed000${i}.eintlog`;
    generateLog(seed, out, schemaHash);
}
