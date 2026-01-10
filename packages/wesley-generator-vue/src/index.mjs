import path from "node:path";

/**
 * Generate Vue artifacts (ops.ts, schemas.ts, client.ts, useEcho.ts) from Echo Ops IR.
 * @param {object} ir - Echo Ops IR JSON (contains ops[], types[], schema_sha256, codec_id).
 * @param {object} options - { outDir?: string }
 * @returns {{ files: { path: string, content: string }[] }}
 */
export async function generateVue(ir, options = {}) {
  if (!ir || !Array.isArray(ir.ops)) {
    throw new Error("@wesley/generator-vue requires Echo Ops IR with `ops[]`");
  }
  const outDir = options.outDir ?? "src/wesley/generated";
  const files = [];
  files.push({ path: path.join(outDir, "ops.ts"), content: emitOps(ir) });
  files.push({ path: path.join(outDir, "schemas.ts"), content: emitSchemas(ir) });
  files.push({ path: path.join(outDir, "client.ts"), content: emitClient(ir) });
  files.push({ path: path.join(outDir, "useEcho.ts"), content: emitUseEcho() });
  return { files };
}

// ----------------------- emitOps -----------------------

function emitOps(ir) {
  const schemaSha = ir.schema_sha256 ?? "unknown";
  const codecId = ir.codec_id ?? "unknown";
  const regVer = ir.registry_version ?? 0;
  const ops = [...ir.ops].sort((a, b) => {
    const ak = String(a.kind).toUpperCase();
    const bk = String(b.kind).toUpperCase();
    if (ak !== bk) return ak.localeCompare(bk);
    const an = String(a.name);
    const bn = String(b.name);
    if (an !== bn) return an.localeCompare(bn);
    return (a.op_id ?? 0) - (b.op_id ?? 0);
  });

  const lines = [];
  lines.push("// AUTO-GENERATED. DO NOT EDIT.");
  lines.push(`// schema_sha256: ${schemaSha}`);
  lines.push(`// codec_id: ${codecId}`);
  lines.push(`// registry_version: ${regVer}`);
  lines.push("");
  lines.push(`export const SCHEMA_SHA256 = ${JSON.stringify(schemaSha)};`);
  lines.push(`export const CODEC_ID = ${JSON.stringify(codecId)};`);
  lines.push(`export const REGISTRY_VERSION = ${Number(regVer)};`);
  lines.push("");

  for (const op of ops) {
    const kind = String(op.kind).toUpperCase();
    const constName =
      kind === "QUERY"
        ? `QUERY_${toScreamingSnake(op.name)}_ID`
        : `MUT_${toScreamingSnake(op.name)}_ID`;
    lines.push(`export const ${constName} = ${op.op_id} as const;`);
  }

  lines.push("");
  lines.push(`export type OpKind = "QUERY" | "MUTATION";`);
  lines.push(`export type OpDef = { kind: OpKind; name: string; opId: number };`);
  lines.push("");
  lines.push("export const OPS: readonly OpDef[] = [");
  for (const op of ops) {
    const kind = String(op.kind).toUpperCase();
    lines.push(
      `  { kind: ${JSON.stringify(kind)}, name: ${JSON.stringify(op.name)}, opId: ${op.op_id} },`
    );
  }
  lines.push("] as const;");
  lines.push("");
  lines.push("export function findOpId(kind: OpKind, name: string): number | undefined {");
  lines.push("  const hit = OPS.find((o) => o.kind === kind && o.name === name);");
  lines.push("  return hit?.opId;");
  lines.push("}");
  lines.push("");
  return lines.join("\n");
}

// ----------------------- emitSchemas -----------------------

function emitSchemas(ir) {
  const schemaSha = ir.schema_sha256 ?? "unknown";
  const codecId = ir.codec_id ?? "unknown";
  const regVer = ir.registry_version ?? 0;
  const types = ir.types ?? [];
  const ops = ir.ops ?? [];

  const lines = [];
  lines.push("// AUTO-GENERATED. DO NOT EDIT.");
  lines.push(`// schema_sha256: ${schemaSha}`);
  lines.push(`// codec_id: ${codecId}`);
  lines.push(`// registry_version: ${regVer}`);
  lines.push("");
  lines.push('import { z } from "zod";');
  lines.push("");

  const typeMap = new Map(types.map((t) => [t.name, t]));

  // ENUMs
  for (const t of types) {
    if (t.kind !== "ENUM") continue;
    const values = (t.values ?? []).map((v) => JSON.stringify(v)).join(", ");
    lines.push(`export const ${schemaName(t.name)} = z.enum([${values}]);`);
  }
  if (types.some((t) => t.kind === "ENUM")) lines.push("");

  // OBJECTs
  for (const t of types) {
    if (t.kind !== "OBJECT") continue;
    const fields = (t.fields ?? []).map((f) => {
      const schemaExpr = wrapField(f, typeMap);
      return `  ${JSON.stringify(f.name)}: ${schemaExpr},`;
    });
    lines.push(
      `export const ${schemaName(t.name)} = z.object({\n${fields.join("\n")}\n}).strict();`
    );
  }
  if (types.some((t) => t.kind === "OBJECT")) lines.push("");

  // Op var/result schemas
  lines.push("// Operation variable/result schemas");
  for (const op of ops) {
    const varsSchemaName = `${pascal(op.name)}VarsSchema`;
    const resultSchemaName = `${pascal(op.name)}ResultSchema`;
    const args = op.args ?? [];
    const argLines = args.map((a) => {
      const schemaExpr = wrapArg(a, typeMap);
      return `  ${JSON.stringify(a.name)}: ${schemaExpr},`;
    });
    lines.push(`export const ${varsSchemaName} = z.object({\n${argLines.join("\n")}\n}).strict();`);
    if (op.result_type) {
      lines.push(`export const ${resultSchemaName} = ${refType(op.result_type, typeMap)};`);
    } else {
      lines.push(`export const ${resultSchemaName} = z.undefined();`);
    }
    lines.push("");
  }

  return lines.join("\n");
}

function refType(name, typeMap) {
  if (isScalar(name)) return scalarSchema(name);
  if (!typeMap.has(name)) throw new Error(`Unknown type: ${name}`);
  return schemaName(name);
}
function wrapField(f, typeMap) {
  return wrapType(f.type, !!f.list, !!f.required, typeMap);
}
function wrapArg(a, typeMap) {
  return wrapType(a.type, !!a.list, !!a.required, typeMap);
}
function wrapType(typeName, list, required, typeMap) {
  let expr = refType(typeName, typeMap);
  if (list) expr = `z.array(${expr})`;
  if (!required) expr = `${expr}.optional()`;
  return expr;
}
function schemaName(name) {
  return `${name}Schema`;
}
function isScalar(t) {
  return t === "String" || t === "Boolean" || t === "Int" || t === "Float" || t === "ID";
}
function scalarSchema(t) {
  switch (t) {
    case "String":
    case "ID":
      return "z.string()";
    case "Boolean":
      return "z.boolean()";
    case "Int":
      return "z.number().int()";
    case "Float":
      return "z.number()";
    default:
      throw new Error(`Unknown scalar: ${t}`);
  }
}

// ----------------------- emitClient -----------------------

function emitClient(ir) {
  const schemaSha = ir.schema_sha256 ?? "unknown";
  const codecId = ir.codec_id ?? "unknown";
  const ops = ir.ops ?? [];
  const queries = ops.filter((o) => String(o.kind).toUpperCase() === "QUERY");
  const muts = ops.filter((o) => String(o.kind).toUpperCase() === "MUTATION");

  const lines = [];
  lines.push("// AUTO-GENERATED. DO NOT EDIT.");
  lines.push(`// schema_sha256: ${schemaSha}`);
  lines.push(`// codec_id: ${codecId}`);
  lines.push("");
  lines.push('import {');
  lines.push('  CODEC_ID, SCHEMA_SHA256, REGISTRY_VERSION,');
  for (const op of ops) {
    const kind = String(op.kind).toUpperCase();
    const constName =
      kind === "QUERY"
        ? `QUERY_${toScreamingSnake(op.name)}_ID`
        : `MUT_${toScreamingSnake(op.name)}_ID`;
    lines.push(`  ${constName},`);
  }
  lines.push('} from "./ops";');
  lines.push('import {');
  for (const op of ops) {
    const name = pascal(op.name);
    lines.push(`  ${name}VarsSchema,`);
    lines.push(`  ${name}ResultSchema,`);
  }
  lines.push('} from "./schemas";');
  lines.push("");
  lines.push("export type Bytes = Uint8Array;");
  lines.push("");
  lines.push("export interface EchoWasm {");
  lines.push("  dispatch_intent(intentBytes: Bytes): void;");
  lines.push("  step(stepBudget: number): Bytes; // StepResult");
  lines.push("  drain_view_ops(): Bytes; // ViewOp[]");
  lines.push("  get_head(): Bytes; // HeadInfo");
  lines.push("");
  lines.push("  execute_query(queryId: number, varsBytes: Bytes): Bytes;");
  lines.push("  encode_command(opId: number, payload: unknown): Bytes;");
  lines.push("  encode_query_vars(queryId: number, vars: unknown): Bytes;");
  lines.push("");
  lines.push("  get_registry_info?(): Bytes;");
  lines.push("}");
  lines.push("");
  lines.push("export type RegistryInfo = { schema_sha256: string; codec_id: string; registry_version: number };");
  lines.push("");
  lines.push("export class WesleyClient {");
  lines.push("  constructor(private wasm: EchoWasm) {}");
  lines.push("");
  lines.push("  verifyRegistry(decodeRegistryInfo?: (bytes: Bytes) => RegistryInfo) {");
  lines.push("    if (!this.wasm.get_registry_info || !decodeRegistryInfo) return;");
  lines.push("    const info = decodeRegistryInfo(this.wasm.get_registry_info());");
  lines.push("    if (info.schema_sha256 !== SCHEMA_SHA256) throw new Error('Schema hash mismatch');");
  lines.push("    if (info.codec_id !== CODEC_ID) throw new Error('Codec mismatch');");
  lines.push("    if (info.registry_version !== REGISTRY_VERSION) throw new Error('Registry version mismatch');");
  lines.push("  }");
  lines.push("");
  for (const q of queries) {
    const fn = `query${pascal(q.name)}`;
    const constName = `QUERY_${toScreamingSnake(q.name)}_ID`;
    lines.push(`  ${fn}(vars: unknown = {}) {`);
    lines.push(`    const parsed = ${pascal(q.name)}VarsSchema.parse(vars);`);
    lines.push(`    const varsBytes = this.wasm.encode_query_vars(${constName}, parsed);`);
    lines.push(`    const resultBytes = this.wasm.execute_query(${constName}, varsBytes);`);
    lines.push(`    return { bytes: resultBytes, schema: ${pascal(q.name)}ResultSchema };`);
    lines.push("  }");
    lines.push("");
  }
  for (const m of muts) {
    const fn = `dispatch${pascal(m.name)}`;
    const constName = `MUT_${toScreamingSnake(m.name)}_ID`;
    const args = (m.args ?? []).map((a) => safeIdent(a.name)).join(", ");
    const payloadObj =
      (m.args ?? []).length === 0
        ? "{}"
        : `{ ${(m.args ?? []).map((a) => `${safeIdent(a.name)}: ${safeIdent(a.name)}`).join(", ")} }`;
    lines.push(`  ${fn}(${args}) {`);
    lines.push(`    const payload = ${pascal(m.name)}VarsSchema.parse(${payloadObj});`);
    lines.push(`    const bytes = this.wasm.encode_command(${constName}, payload);`);
    lines.push("    this.wasm.dispatch_intent(bytes);");
    lines.push("  }");
    lines.push("");
  }
  lines.push("}");
  lines.push("");
  return lines.join("\n");
}

// ----------------------- emitUseEcho -----------------------

function emitUseEcho() {
  return `// AUTO-GENERATED scaffold. You will likely extend this.\n` +
    `import { WesleyClient } from "./client";\n\n` +
    `export function useEcho(wasm) {\n` +
    `  const client = new WesleyClient(wasm);\n` +
    `  const pump = (budget = 1000) => {\n` +
    `    const res = wasm.step(budget);\n` +
    `    // TODO: decode StepResult and apply ViewOps via wasm.drain_view_ops()\n` +
    `    return res;\n` +
    `  };\n` +
    `  return { client, pump };\n` +
    `}\n`;
}

// ----------------------- helpers -----------------------

function pascal(name) {
  return String(name)
    .replace(/[^A-Za-z0-9]+/g, " ")
    .trim()
    .split(/\s+/)
    .map((w) => w.slice(0, 1).toUpperCase() + w.slice(1))
    .join("");
}

function toScreamingSnake(name) {
  return String(name)
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[^A-Za-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .toUpperCase();
}

function safeIdent(name) {
  const s = String(name).replace(/[^A-Za-z0-9_]/g, "_");
  if (!/^[A-Za-z_]/.test(s)) return `_${s}`;
  return s;
}
