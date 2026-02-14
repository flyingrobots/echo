// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { describe, it, expect } from "vitest";
import { hashToHex, hexToHash } from "../types/SceneDelta";

describe("hashToHex", () => {
    it("converts Uint8Array to hex string", () => {
        const hash = new Uint8Array(32);
        hash[0] = 0xab;
        hash[1] = 0xcd;
        hash[31] = 0xff;

        const hex = hashToHex(hash);

        expect(hex).toHaveLength(64);
        expect(hex.slice(0, 4)).toBe("abcd");
        expect(hex.slice(-2)).toBe("ff");
    });

    it("handles all zeros", () => {
        const hash = new Uint8Array(32);
        const hex = hashToHex(hash);

        expect(hex).toBe("0".repeat(64));
    });
});

describe("hexToHash", () => {
    it("converts hex string to Uint8Array", () => {
        const hex = "abcd" + "00".repeat(29) + "ff";
        const hash = hexToHash(hex);

        expect(hash).toBeInstanceOf(Uint8Array);
        expect(hash.length).toBe(32);
        expect(hash[0]).toBe(0xab);
        expect(hash[1]).toBe(0xcd);
        expect(hash[31]).toBe(0xff);
    });

    it("throws on invalid length", () => {
        expect(() => hexToHash("abcd")).toThrow("Invalid hash hex length");
    });

    it("roundtrips correctly", () => {
        const original = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            original[i] = i * 8;
        }

        const hex = hashToHex(original);
        const roundtrip = hexToHash(hex);

        expect(roundtrip).toEqual(original);
    });
});
