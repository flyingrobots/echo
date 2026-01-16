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
    choice(arr) {
        return arr[this.range(0, arr.length)];
    }
}

const OUT_PATH = "testdata/dind/010_dense_rewrite_seed0001.eintlog";

// ELOG Header
const MAGIC = Buffer.from("ELOG");
const VERSION = Buffer.alloc(2); VERSION.writeUInt16LE(1);
const FLAGS = Buffer.alloc(2); FLAGS.writeUInt16LE(0);
const RESERVED = Buffer.alloc(8);

// Helper to write
function writeLog(schemaHashHex, frames) {
    const fd = fs.openSync(OUT_PATH, "w");
    fs.writeSync(fd, MAGIC);
    fs.writeSync(fd, VERSION);
    fs.writeSync(fd, FLAGS);
    fs.writeSync(fd, Buffer.from(schemaHashHex, "hex"));
    fs.writeSync(fd, RESERVED);
    
    for (const frame of frames) {
        const len = Buffer.alloc(4);
        len.writeUInt32LE(frame.byteLength);
        fs.writeSync(fd, len);
        fs.writeSync(fd, frame);
    }
    fs.closeSync(fd);
    console.log(`Wrote ${OUT_PATH}`);
}

// Op Constants
// EINT(4) + OpID(4) + Len(4) + Payload
function packFrame(opId, payload) {
    const header = Buffer.alloc(12);
    header.write("EINT", 0);
    header.writeUInt32LE(opId, 4);
    header.writeUInt32LE(payload.length, 8);
    return Buffer.concat([header, payload]);
}

// OpIDs from generated codecs (hardcoded here to keep script self-contained and stable)
const OP_SET_THEME = 1822649880;
const OP_ROUTE_PUSH = 2216217860;
const OP_TOGGLE_NAV = 3272403183;
const OP_TOAST = 4255241313;
const OP_DROP_BALL = 778504871;

const rng = new RNG(12345); // Fixed seed
const frames = [];

// Generate 500 ops (mix of trivial state changes to churn the graph)
for (let i = 0; i < 500; i++) {
    const opType = rng.choice(["THEME", "NAV", "ROUTE", "TOAST", "DROP"]);
    
    if (opType === "THEME") {
        const mode = rng.range(0, 3); // 0=LIGHT, 1=DARK, 2=SYSTEM
        const payload = Buffer.alloc(2);
        payload.writeUInt16LE(mode, 0);
        frames.push(packFrame(OP_SET_THEME, payload));
    } else if (opType === "NAV") {
        frames.push(packFrame(OP_TOGGLE_NAV, Buffer.alloc(0)));
    } else if (opType === "ROUTE") {
        const path = `/page/${rng.range(0, 100)}`;
        const strBytes = Buffer.from(path, 'utf8');
        const payload = Buffer.alloc(4 + strBytes.length);
        payload.writeUInt32LE(strBytes.length, 0);
        strBytes.copy(payload, 4);
        frames.push(packFrame(OP_ROUTE_PUSH, payload));
    } else if (opType === "TOAST") {
        const msg = `Message ${rng.range(0, 1000)}`;
        const strBytes = Buffer.from(msg, 'utf8');
        const payload = Buffer.alloc(4 + strBytes.length);
        payload.writeUInt32LE(strBytes.length, 0);
        strBytes.copy(payload, 4);
        frames.push(packFrame(OP_TOAST, payload));
    } else if (opType === "DROP") {
        frames.push(packFrame(OP_DROP_BALL, Buffer.alloc(0)));
    }
}

// Get Schema Hash
const codecs = fs.readFileSync("crates/echo-dind-tests/src/generated/codecs.rs", "utf8");
const match = codecs.match(/pub const SCHEMA_HASH: &str = "([0-9a-f]+)";/);
if (!match) throw new Error("Could not find SCHEMA_HASH in codecs.rs");
const schemaHash = match[1];

writeLog(schemaHash, frames);
