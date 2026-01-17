// Bootstrap script for 060_math_determinism test fixture.
//
// This generates an ELOG file that tests fixed-point physics determinism.
// The ball drops from height 400 with initial velocity -5, accelerating due
// to gravity until it reaches ground (y=0) and settles.
//
// IMPORTANT: Each frame MUST have unique content bytes because the engine uses
// content-addressed deduplication. Frames with identical bytes would be treated
// as duplicates and only processed once. We use route_push with unique paths
// (e.g., "/physics/step/0", "/physics/step/1", ...) to ensure each frame is
// unique while the physics simulation runs.

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

// OpIDs from crates/echo-dind-tests/src/codecs.generated.rs
const OP_DROP_BALL = 778504871;
const OP_ROUTE_PUSH = 2216217860;

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

// Encode a route_push args buffer (length-prefixed string)
function encodeRoutePushArgs(path) {
    const pathBytes = Buffer.from(path, "utf8");
    const lenBuf = Buffer.alloc(4);
    lenBuf.writeUInt32LE(pathBytes.length);
    return Buffer.concat([lenBuf, pathBytes]);
}

const frames = [];

// 1. Drop Ball (Initiates Motion/Physics)
frames.push(makeFrame(OP_DROP_BALL));

// 2. Padding steps to allow physics to simulate
// Each step in the harness runs the physics rule once if applicable.
// We use route_push with unique paths to ensure each frame has a unique
// content hash (intents are content-addressed, so identical frames would
// be deduplicated).
for (let i = 0; i < 50; i++) {
    const args = encodeRoutePushArgs(`/physics/step/${i}`);
    frames.push(makeFrame(OP_ROUTE_PUSH, args));
}

// Get Schema Hash
const codecs = fs.readFileSync("crates/echo-dind-tests/src/codecs.generated.rs", "utf8");
const match = codecs.match(/pub const SCHEMA_HASH: &str = "([0-9a-fA-F]+)";/);
if (!match) throw new Error("Could not find SCHEMA_HASH in codecs.generated.rs");
const schemaHash = match[1];
if (!/^[0-9a-f]{64}$/i.test(schemaHash)) {
    throw new Error(`Invalid SCHEMA_HASH: expected 64 hex chars, got ${schemaHash.length}`);
}

writeLog(schemaHash, frames);
