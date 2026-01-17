import fs from "node:fs";
// Fix import path: relative to scripts/ is ../src/wasm/echo_protocol.ts
// But node might not like .ts extension directly without loader. 
// I'll assume I can run this with `bun` or `ts-node`, or simply read the file and eval it since it has no imports itself.
// Actually, `echo_protocol.ts` is pure JS syntax (enums are problematic in raw JS, but the generated file uses `export enum` which is TS).
// Wait, the generated `echo_protocol.ts` uses `export enum`. Node won't run that.
// I should rely on the *generator* to output a JS version or I should parse it.
// OR, I can just use raw bytes for this bootstrap script since I know the spec.
// "EINT" + OpID + Len + Vars.
// SetTheme is OpID 1822649880. Args: u16 mode.
// Mode DARK = 1.
// Payload: 01 00.
// Envelope: "EINT" + 18 32 A3 6C + 02 00 00 00 + 01 00.
// I will just hardcode the bytes for the bootstrap to avoid TS dependency hell in this script.

const OUT_PATH = "testdata/dind/000_smoke_theme.eintlog";

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

// 1. Set Theme DARK
// OpID: 1822649880 (0x6ca33218)
// Args: 01 00 (u16)
const op1 = Buffer.concat([
    Buffer.from("EINT"),
    Buffer.from([0x18, 0x32, 0xa3, 0x6c]), // OpID LE
    Buffer.from([0x02, 0x00, 0x00, 0x00]), // Len LE
    Buffer.from([0x01, 0x00])              // Payload
]);
frames.push(op1);

// 2. Set Theme LIGHT
// Args: 00 00
const op2 = Buffer.concat([
    Buffer.from("EINT"),
    Buffer.from([0x18, 0x32, 0xa3, 0x6c]),
    Buffer.from([0x02, 0x00, 0x00, 0x00]),
    Buffer.from([0x00, 0x00])
]);
frames.push(op2);

const codecs = fs.readFileSync("crates/echo-dind-tests/src/codecs.generated.rs", "utf8");
const match = codecs.match(/SCHEMA_HASH:\s*&str\s*=\s*"([0-9a-fA-F]+)"/);
if (!match) throw new Error("Could not find SCHEMA_HASH");
const schemaHash = match[1];
if (!/^[0-9a-f]{64}$/i.test(schemaHash)) {
    throw new Error(`Invalid SCHEMA_HASH: expected 64 hex chars, got ${schemaHash.length}`);
}

writeLog(schemaHash, frames);
