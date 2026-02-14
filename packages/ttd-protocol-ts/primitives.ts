// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
/**
 * TTD Primitive Types
 *
 * Base types used throughout the TTD protocol.
 * These are foundational types that should be in the Wesley schema but are
 * defined here to support the generated types.
 */

import { z } from 'zod';

/** 32-byte hash identifier */
export type Hash = Uint8Array;

/** Timestamp in milliseconds since epoch */
export type Timestamp = number;

/** Zod schema for Hash */
export const HashSchema = z.instanceof(Uint8Array);

/** Zod schema for Timestamp */
export const TimestampSchema = z.number();

/**
 * Opaque type for schema identifiers (SHA-256 hashes).
 *
 * Provides type-safe comparison and string representation for schema version matching.
 * Use this instead of raw string comparisons to ensure cross-language consistency.
 */
export class SchemaId {
  /** The internal hash value (64-char hex string) */
  readonly #hash: string;

  private constructor(hash: string) {
    if (!/^[0-9a-f]{64}$/i.test(hash)) {
      throw new Error(`Invalid schema hash: expected 64 hex characters, got "${hash}"`);
    }
    this.#hash = hash.toLowerCase();
  }

  /**
   * Create a SchemaId from a hex string.
   * @param hex - 64-character hex string (SHA-256 hash)
   */
  static fromHex(hex: string): SchemaId {
    return new SchemaId(hex);
  }

  /**
   * Check if this SchemaId equals another.
   * @param other - Another SchemaId to compare
   */
  equals(other: SchemaId): boolean {
    return this.#hash === other.#hash;
  }

  /**
   * Check if this SchemaId matches a hex string.
   * @param hex - 64-character hex string to compare
   */
  matchesHex(hex: string): boolean {
    return this.#hash === hex.toLowerCase();
  }

  /**
   * Get the hex string representation.
   */
  toHex(): string {
    return this.#hash;
  }

  /**
   * String representation for debugging.
   */
  toString(): string {
    return `SchemaId(${this.#hash.slice(0, 8)}...)`;
  }

  /**
   * JSON serialization returns the hex string.
   */
  toJSON(): string {
    return this.#hash;
  }
}
