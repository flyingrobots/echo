import fs from "node:fs";

const OUT_PATH = "testdata/dind/030_error_determinism.eintlog";

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

const frames = [];

// 1. Valid Op (SetTheme DARK) - Baseline
// OpID: 1822649880 (0x6ca33218)
// Args: 01 00 (u16)
frames.push(Buffer.concat([
    Buffer.from("EINT"),
    Buffer.from([0x18, 0x32, 0xa3, 0x6c]), 
    Buffer.from([0x02, 0x00, 0x00, 0x00]), 
    Buffer.from([0x01, 0x00])
]));

// 2. Invalid Op ID (0xFFFFFFFF)
// Should be ignored/no-op
frames.push(Buffer.concat([
    Buffer.from("EINT"),
    Buffer.from([0xff, 0xff, 0xff, 0xff]), 
    Buffer.from([0x00, 0x00, 0x00, 0x00]), 
    Buffer.alloc(0)
]));

// 3. Valid Op, Wrong Length (Declared 2, provided 1)
// SetTheme
frames.push(Buffer.concat([
    Buffer.from("EINT"),
    Buffer.from([0x18, 0x32, 0xa3, 0x6c]), 
    Buffer.from([0x02, 0x00, 0x00, 0x00]), 
    Buffer.from([0x01]) // Missing one byte
]));

// 4. Malformed Envelope (Missing Magic)
frames.push(Buffer.from("JUNK000000000000"));

// 5. Valid Op, Valid Envelope, Logic Error (e.g. valid payload but rule doesn't match? 
// Actually, in our kernel, if op decodes, it creates an intent node. 
// If no rule matches the intent, it just sits there or gets cleaned up?
// Current `dispatch_intent` adds `sim/inbox/event:SEQ`. 
// If no rule matches, the event stays or is skipped? 
// `step()` iterates events. If no rule applies, it finishes.
// So state WILL change (sequence number increments, event node created). 
// Determinism means it changes the SAME way every time.

// Let's add a "valid but unknown" op that passes envelope check but has no rule.
// OpID: 0x12345678 (Random valid u32)
frames.push(Buffer.concat([
    Buffer.from("EINT"),
    Buffer.from([0x78, 0x56, 0x34, 0x12]), 
    Buffer.from([0x00, 0x00, 0x00, 0x00]), 
    Buffer.alloc(0)
]));

// 6. Valid Op (SetTheme LIGHT) - Recovery check
frames.push(Buffer.concat([
    Buffer.from("EINT"),
    Buffer.from([0x18, 0x32, 0xa3, 0x6c]), 
    Buffer.from([0x02, 0x00, 0x00, 0x00]), 
    Buffer.from([0x00, 0x00])
]));

const codecs = fs.readFileSync("crates/flyingrobots-echo-wasm/src/generated/codecs.rs", "utf8");
const match = codecs.match(/pub const SCHEMA_HASH: &str = "([0-9a-f]+)";/);
if (!match) throw new Error("Could not find SCHEMA_HASH in codecs.rs");
const schemaHash = match[1];

writeLog(schemaHash, frames);
