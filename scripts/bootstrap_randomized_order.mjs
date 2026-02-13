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
const OP_PUT_KV = 3000000001;

function generateLog(seed, outPath, schemaHashHex, type) {
    const rng = new RNG(seed);
    const frames = [];

    const ops = [];
    if (type === "050") {
        // 050: Randomized order, non-convergent (route_push overwrites)
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
    } else if (type === "051") {
        // 051: Randomized order, convergent (disjoint put_kv)
        for (let i = 0; i < 100; i++) {
            const key = `key_${i}`;
            const value = `value_${seed}_${i}`;
            
            // put_kv args: { key: string, value: string }
            const keyBytes = Buffer.from(key, 'utf8');
            const valBytes = Buffer.from(value, 'utf8');
            
            // Layout: key_len (u32) + key + val_len (u32) + val
            const payload = Buffer.alloc(4 + keyBytes.length + 4 + valBytes.length);
            let offset = 0;
            payload.writeUInt32LE(keyBytes.length, offset); offset += 4;
            keyBytes.copy(payload, offset); offset += keyBytes.length;
            payload.writeUInt32LE(valBytes.length, offset); offset += 4;
            valBytes.copy(payload, offset); offset += valBytes.length;
            
            ops.push(packFrame(OP_PUT_KV, payload));
        }
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
const codecs = fs.readFileSync("crates/echo-dind-tests/src/codecs.generated.rs", "utf8");
const match = codecs.match(/pub const SCHEMA_HASH: &str = "([0-9a-fA-F]+)";/);
if (!match) throw new Error("Could not find SCHEMA_HASH in codecs.generated.rs");
const schemaHash = match[1];
if (!/^[0-9a-f]{64}$/i.test(schemaHash)) {
    throw new Error(`Invalid SCHEMA_HASH: expected 64 hex chars, got ${schemaHash.length}`);
}

// Generate 3 seeds for 050 (Invariant A)
for (let i = 1; i <= 3; i++) {
    const seed = 1000 + i;
    const out = `testdata/dind/050_randomized_order_small_seed000${i}.eintlog`;
    generateLog(seed, out, schemaHash, "050");
}

// Generate 3 seeds for 051 (Invariant B)
// Note: To prove convergence, these MUST use the SAME set of operations
// just in different orders. So we use the SAME base seed for ops, 
// but different seeds for SHUFFLING.
const BASE_SEED_OPS = 5151;
for (let i = 1; i <= 3; i++) {
    const shuffleSeed = 2000 + i;
    const out = `testdata/dind/051_randomized_convergent_seed000${i}.eintlog`;
    
    // We need to modify generateLog slightly to take a separate seed for shuffle
    generateLogConvergent(BASE_SEED_OPS, shuffleSeed, out, schemaHash);
}

function generateLogConvergent(opSeed, shuffleSeed, outPath, schemaHashHex) {
    const opRng = new RNG(opSeed);
    const shuffleRng = new RNG(shuffleSeed);
    
    const ops = [];
    // 051: Randomized order, convergent (disjoint put_kv)
    for (let i = 0; i < 100; i++) {
        const key = `key_${i}`;
        const value = `value_convergent_${i}`; // Same values for all logs to ensure convergence
        
        const keyBytes = Buffer.from(key, 'utf8');
        const valBytes = Buffer.from(value, 'utf8');
        
        const payload = Buffer.alloc(4 + keyBytes.length + 4 + valBytes.length);
        let offset = 0;
        payload.writeUInt32LE(keyBytes.length, offset); offset += 4;
        keyBytes.copy(payload, offset); offset += keyBytes.length;
        payload.writeUInt32LE(valBytes.length, offset); offset += 4;
        valBytes.copy(payload, offset); offset += valBytes.length;
        
        ops.push(packFrame(OP_PUT_KV, payload));
    }

    // Shuffle the operations using the shuffleSeed
    shuffleRng.shuffle(ops);
    
    // ELOG Header ... (rest of the logic)
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
