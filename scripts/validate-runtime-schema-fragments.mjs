// SPDX-License-Identifier: Apache-2.0
// Copyright (c) James Ross FLYING-ROBOTS <https://github.com/flyingrobots>

import { readdirSync, readFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const BUILTIN_TYPES = new Set(["String", "Int", "Float", "Boolean", "ID"]);
const SCRIPT_DIR = fileURLToPath(new URL(".", import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "..");

function usage() {
    console.error(
        "usage: node scripts/validate-runtime-schema-fragments.mjs [--dir <schema-dir>]",
    );
}

function parseArgs(argv) {
    let schemaDir = "schemas/runtime";

    for (let index = 0; index < argv.length; index += 1) {
        const arg = argv[index];
        if (arg === "--dir") {
            const next = argv[index + 1];
            if (!next) {
                usage();
                process.exit(2);
            }
            schemaDir = next;
            index += 1;
            continue;
        }
        if (arg === "--help" || arg === "-h") {
            usage();
            process.exit(0);
        }

        usage();
        console.error(`unexpected argument: ${arg}`);
        process.exit(2);
    }

    return resolve(schemaDir);
}

function listSchemaFiles(schemaDir) {
    const files = readdirSync(schemaDir)
        .filter((entry) => entry.endsWith(".graphql"))
        .sort()
        .map((entry) => join(schemaDir, entry));

    if (files.length === 0) {
        throw new Error(`no runtime schema fragments found under ${schemaDir}`);
    }

    return files;
}

function runPrettierCheck(files) {
    const formattingErrors = [];

    for (const file of files) {
        const source = readFileSync(file, "utf8");
        const syntheticFilePath = join(
            REPO_ROOT,
            "schemas/runtime",
            basename(file),
        );
        const result = spawnSync(
            "npx",
            [
                "prettier",
                "--parser",
                "graphql",
                "--stdin-filepath",
                syntheticFilePath,
            ],
            {
                encoding: "utf8",
                input: source,
            },
        );

        if (result.error) {
            throw new Error(
                `failed to run npx prettier for schema validation: ${result.error.message}`,
            );
        }

        if (result.status !== 0) {
            if (result.stderr) {
                process.stderr.write(result.stderr);
            }
            process.exit(result.status ?? 1);
        }

        if (result.stdout !== source) {
            formattingErrors.push(file);
        }
    }

    if (formattingErrors.length > 0) {
        console.error("runtime schema formatting check failed:");
        for (const file of formattingErrors) {
            console.error(`  - ${file}`);
        }
        process.exit(1);
    }
}

function sanitizeLines(source) {
    const lines = source.split(/\r?\n/u);
    const sanitized = [];
    let inDescription = false;

    for (const rawLine of lines) {
        let line = rawLine;

        if (inDescription) {
            if (line.includes('"""')) {
                inDescription = false;
            }
            sanitized.push("");
            continue;
        }

        const firstTripleQuote = line.indexOf('"""');
        if (firstTripleQuote !== -1) {
            const secondTripleQuote = line.indexOf('"""', firstTripleQuote + 3);
            if (secondTripleQuote === -1) {
                inDescription = true;
            }
            sanitized.push("");
            continue;
        }

        const commentStart = line.indexOf("#");
        if (commentStart !== -1) {
            line = line.slice(0, commentStart);
        }

        sanitized.push(line.trimEnd());
    }

    return sanitized;
}

function collectDefinitions(file, lines, definitions, errors) {
    for (let index = 0; index < lines.length; index += 1) {
        const match = lines[index].match(
            /^\s*(scalar|type|input|enum|union)\s+([A-Za-z_][A-Za-z0-9_]*)\b/u,
        );
        if (!match) {
            continue;
        }

        const kind = match[1];
        const name = match[2];
        const previous = definitions.get(name);
        if (previous) {
            errors.push(
                `${file}:${index + 1}: duplicate ${kind} ${name}; already defined at ${previous.file}:${previous.line}`,
            );
            continue;
        }

        definitions.set(name, { kind, file, line: index + 1 });
    }
}

function extractTypeNames(typeExpression) {
    return typeExpression.match(/[A-Za-z_][A-Za-z0-9_]*/gu) ?? [];
}

function validateReference(file, lineNumber, refName, definitions, errors) {
    if (BUILTIN_TYPES.has(refName)) {
        return;
    }

    if (!definitions.has(refName)) {
        errors.push(
            `${file}:${lineNumber}: missing referenced type ${refName} in runtime schema fragments`,
        );
    }
}

function validateReferences(file, lines, definitions, errors) {
    let bodyKind = null;

    for (let index = 0; index < lines.length; index += 1) {
        const trimmed = lines[index].trim();
        if (trimmed.length === 0) {
            continue;
        }

        if (bodyKind === "type" || bodyKind === "input") {
            if (trimmed.startsWith("}")) {
                bodyKind = null;
                continue;
            }

            const colonIndex = trimmed.indexOf(":");
            if (colonIndex !== -1) {
                const typeExpression = trimmed.slice(colonIndex + 1);
                for (const refName of extractTypeNames(typeExpression)) {
                    validateReference(
                        file,
                        index + 1,
                        refName,
                        definitions,
                        errors,
                    );
                }
            }
            continue;
        }

        if (bodyKind === "enum") {
            if (trimmed.startsWith("}")) {
                bodyKind = null;
            }
            continue;
        }

        if (bodyKind === "union") {
            if (trimmed.startsWith("|")) {
                for (const refName of extractTypeNames(trimmed.slice(1))) {
                    validateReference(
                        file,
                        index + 1,
                        refName,
                        definitions,
                        errors,
                    );
                }
                continue;
            }
            bodyKind = null;
        }

        const match = trimmed.match(
            /^(scalar|type|input|enum|union)\s+([A-Za-z_][A-Za-z0-9_]*)\b/u,
        );
        if (!match) {
            continue;
        }

        const kind = match[1];
        if (kind === "type" || kind === "input" || kind === "enum") {
            if (trimmed.includes("{")) {
                bodyKind = kind;
            }
            continue;
        }

        if (kind === "union") {
            const equalsIndex = trimmed.indexOf("=");
            if (equalsIndex !== -1) {
                for (const refName of extractTypeNames(
                    trimmed.slice(equalsIndex + 1),
                )) {
                    validateReference(
                        file,
                        index + 1,
                        refName,
                        definitions,
                        errors,
                    );
                }
            }
            bodyKind = "union";
        }
    }
}

function main() {
    const schemaDir = parseArgs(process.argv.slice(2));
    const files = listSchemaFiles(schemaDir);

    runPrettierCheck(files);

    const parsedFiles = files.map((file) => ({
        file,
        lines: sanitizeLines(readFileSync(file, "utf8")),
    }));

    const definitions = new Map();
    const errors = [];

    for (const parsed of parsedFiles) {
        collectDefinitions(parsed.file, parsed.lines, definitions, errors);
    }

    for (const parsed of parsedFiles) {
        validateReferences(parsed.file, parsed.lines, definitions, errors);
    }

    if (errors.length > 0) {
        console.error("runtime schema validation failed:");
        for (const error of errors) {
            console.error(`  - ${error}`);
        }
        process.exit(1);
    }

    console.log(
        `runtime schema validation passed for ${files.length} fragment(s) under ${schemaDir}`,
    );
}

try {
    main();
} catch (error) {
    console.error(
        error instanceof Error
            ? error.message
            : "runtime schema validation failed",
    );
    process.exit(1);
}
