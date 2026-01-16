import fs from "node:fs";

// Simplified seeded random number generator (Xorshift32)
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

// putKv is 3000000001 (0xB2D05E01)
const OP_PUT_KV = 3000000001;

function generateLog(seed, outPath, schemaHashHex) {
    const rng = new RNG(seed);
    
    // Generate 200 disjoint KV inserts.
    // Key space: "key_{i}" for i in 0..200.
    // Since keys are unique, node IDs will be unique (hash of key).
    // Operations are commutative (set union of disjoint nodes).
    
    const ops = [];
    for (let i = 0; i < 200; i++) {
        const key = `key_${i}`;
        const val = `val_${i}_seed_${seed}`; // Value can vary per seed? No, must be same multiset.
        // Wait, "Same multiset of operations".
        // If I change value per seed, it's NOT the same multiset.
        // I must generate the SAME set of (key, value) pairs, just shuffled.
        const fixedVal = `val_${i}`;
        
        const keyBytes = Buffer.from(key, 'utf8');
        const valBytes = Buffer.from(fixedVal, 'utf8');
        
        // Encode Args: key (String), value (String)
        // String encoding: Len(u32 LE) + Bytes
        let size = 4 + keyBytes.length + 4 + valBytes.length;
        const payload = Buffer.alloc(size);
        let offset = 0;
        
        payload.writeUInt32LE(keyBytes.length, offset); offset += 4;
        keyBytes.copy(payload, offset); offset += keyBytes.length;
        
        payload.writeUInt32LE(valBytes.length, offset); offset += 4;
        valBytes.copy(payload, offset); offset += valBytes.length;
        
        ops.push(packFrame(OP_PUT_KV, payload));
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
const codecs = fs.readFileSync("crates/echo-dind-tests/src/generated/codecs.rs", "utf8");
const match = codecs.match(/pub const SCHEMA_HASH: &str = "([0-9a-fA-F]+)";/);
if (!match) throw new Error("Could not find SCHEMA_HASH in codecs.rs");
const schemaHash = match[1];
if (!/^[0-9a-f]{64}$/i.test(schemaHash)) {
    throw new Error(`Invalid SCHEMA_HASH: expected 64 hex chars, got ${schemaHash.length}`);
}

// Generate 3 seeds
for (let i = 1; i <= 3; i++) {
    const seed = 5000 + i;
    const out = `testdata/dind/051_randomized_convergent_seed000${i}.eintlog`;
    generateLog(seed, out, schemaHash);
}
