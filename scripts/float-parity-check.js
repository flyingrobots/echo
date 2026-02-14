// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Bit-exact implementation of canonicalize_f32 in JavaScript.
 * Mirrors the Rust implementation in crates/echo-scene-port/src/canon.rs.
 * 
 * @param {string} inputStr
 * @returns {number}
 */
function canonicalizeF32(inputStr) {
    const val = parseFloat(inputStr);
    if (!Number.isFinite(val)) {
        throw new Error("NaN/Infinity not allowed in scene data");
    }
    
    // 1. Coerce to f32
    const f32 = Math.fround(val);
    
    // 2. Rust: let scaled = x as f64 * 1_000_000.0;
    const scaled = f32 * 1000000.0;
    
    // 3. Rust: let truncated = (scaled as i64)
    const truncatedBigInt = BigInt(Math.trunc(scaled));
    
    // 4. Rust: (truncated as f32) / 1_000_000.0
    // CRITICAL: The division happens in f32!
    // In JS, division is f64, so we must fround the numerator AND the result.
    const numerator = Math.fround(Number(truncatedBigInt));
    const divisor = Math.fround(1000000.0);
    const result = Math.fround(numerator / divisor);
    
    return result === 0 ? 0.0 : result;
}

const input = process.argv[2];
try {
    const result = canonicalizeF32(input);
    const buf = Buffer.alloc(4);
    buf.writeFloatLE(result);
    process.stdout.write(buf.toString('hex'));
} catch (e) {
    process.stdout.write("ERROR");
}
