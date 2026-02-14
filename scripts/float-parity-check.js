// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Bit-exact implementation of canonicalize_f32 in JavaScript.
 * Mirrors the Rust implementation in crates/echo-scene-port/src/canon.rs.
 * 
 * @param {number} val
 * @returns {number}
 */
function canonicalizeF32(val) {
    if (!Number.isFinite(val)) {
        throw new Error("NaN/Infinity not allowed in scene data");
    }
    
    // Explicit magnitude guard (matches Rust's new 1e12 limit)
    if (Math.abs(val) >= 1000000000000.0) {
        throw new Error("Scene coordinate magnitude exceeds 1e12 limit");
    }

    // 1. Coerce to f32
    const f32 = Math.fround(val);
    
    // 2. Rust: let scaled = x as f64 * 1_000_000.0;
    // JS division/multiplication is f64 by default.
    const scaled = f32 * 1000000.0;
    
    // 3. Rust: let truncated = (scaled as i64)
    // Math.trunc matches 'as i64' (truncates toward zero).
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
if (!input) {
    process.exit(0); // Allow empty input for batch mode if needed, but we expect JSON
}

try {
    const data = JSON.parse(input);
    if (Array.isArray(data)) {
        // Batch mode
        const results = data.map(v => {
            const result = canonicalizeF32(v);
            const buf = Buffer.alloc(4);
            buf.writeFloatLE(result);
            return buf.toString('hex');
        });
        process.stdout.write(JSON.stringify(results));
    } else {
        // Single value mode (legacy support)
        const result = canonicalizeF32(parseFloat(input));
        const buf = Buffer.alloc(4);
        buf.writeFloatLE(result);
        process.stdout.write(buf.toString('hex'));
    }
} catch (e) {
    // If not JSON, try parsing as single float
    try {
        const result = canonicalizeF32(parseFloat(input));
        const buf = Buffer.alloc(4);
        buf.writeFloatLE(result);
        process.stdout.write(buf.toString('hex'));
    } catch (e2) {
        process.stderr.write(e.message);
        process.exit(1);
    }
}
