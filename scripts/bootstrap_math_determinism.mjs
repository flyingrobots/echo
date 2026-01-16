import fs from "node:fs";

const OUT_PATH = "testdata/dind/060_math_determinism.eintlog";

// ELOG Header
const MAGIC = Buffer.from("ELOG");
const VERSION = Buffer.alloc(2); VERSION.writeUInt16LE(1);
const FLAGS = Buffer.alloc(2); FLAGS.writeUInt16LE(0);
const RESERVED = Buffer.alloc(8);

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

// OpIDs from crates/echo-dind-tests/src/generated/codecs.rs
const OP_DROP_BALL = 778504871;
const OP_TOGGLE_NAV = 3272403183; 

function makeFrame(opId, args = Buffer.alloc(0)) {
    const opBuf = Buffer.alloc(4);
    opBuf.writeUInt32LE(opId);
    
    const lenBuf = Buffer.alloc(4);
    lenBuf.writeUInt32LE(args.length);
    
    return Buffer.concat([
        Buffer.from("EINT"),
        opBuf,
        lenBuf,
        args
    ]);
}

const frames = [];

// 1. Drop Ball (Initiates Motion/Physics)
frames.push(makeFrame(OP_DROP_BALL));

// 2. Padding steps to allow physics to simulate
// Each step in the harness runs the physics rule once if applicable.
// We'll add 50 steps of "Toggle Nav" (or just dummy ops if we had a NoOp)
// Toggle Nav is safe, it just flips a bit.
for (let i = 0; i < 50; i++) {
    frames.push(makeFrame(OP_TOGGLE_NAV));
}

// Get Schema Hash
const codecs = fs.readFileSync("crates/echo-dind-tests/src/generated/codecs.rs", "utf8");
const match = codecs.match(/SCHEMA_HASH:\s*&str\s*=\s*"([0-9a-f]+)"/);
if (!match) throw new Error("Could not find SCHEMA_HASH");
const schemaHash = match[1];

writeLog(schemaHash, frames);
