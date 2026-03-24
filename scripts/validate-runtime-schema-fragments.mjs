// SPDX-License-Identifier: Apache-2.0
// Copyright (c) James Ross FLYING-ROBOTS <https://github.com/flyingrobots>

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse, visit } from "graphql";

const BUILTIN_TYPES = new Set(["String", "Int", "Float", "Boolean", "ID"]);
const DEFINITION_KIND_NAMES = new Map([
    ["ScalarTypeDefinition", "scalar"],
    ["ObjectTypeDefinition", "type"],
    ["InputObjectTypeDefinition", "input"],
    ["EnumTypeDefinition", "enum"],
    ["UnionTypeDefinition", "union"],
]);
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

function resolvePrettierInvocation() {
    const localPrettier = resolve(
        REPO_ROOT,
        "node_modules",
        ".bin",
        process.platform === "win32" ? "prettier.cmd" : "prettier",
    );
    const candidates = [
        { command: "npx", prefix: ["prettier"] },
        { command: "pnpm", prefix: ["exec", "prettier"] },
        { command: localPrettier, prefix: [], requireExists: true },
    ];

    for (const candidate of candidates) {
        if (candidate.requireExists && !existsSync(candidate.command)) {
            continue;
        }
        const probe = spawnSync(
            candidate.command,
            [...candidate.prefix, "--version"],
            { encoding: "utf8" },
        );
        if (!probe.error && probe.status === 0) {
            return candidate;
        }
    }

    throw new Error(
        "failed to locate prettier via npx, pnpm exec, or node_modules/.bin",
    );
}

function runPrettierCheck(files) {
    const formattingErrors = [];
    const prettier = resolvePrettierInvocation();

    for (const file of files) {
        const source = readFileSync(file, "utf8");
        const syntheticFilePath = join(
            REPO_ROOT,
            "schemas/runtime",
            basename(file),
        );
        const result = spawnSync(
            prettier.command,
            [
                ...prettier.prefix,
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
                `failed to run prettier for schema validation: ${result.error.message}`,
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

function lineForNode(node) {
    return node?.loc?.startToken?.line ?? 1;
}

function parseDocuments(files) {
    return files.map((file) => {
        const source = readFileSync(file, "utf8");
        try {
            return {
                file,
                document: parse(source, { noLocation: false }),
            };
        } catch (error) {
            if (error instanceof Error && "locations" in error) {
                const line = error.locations?.[0]?.line ?? 1;
                throw new Error(`${file}:${line}: ${error.message}`);
            }
            throw error;
        }
    });
}

function collectDefinitions(documents, definitions, errors) {
    for (const { file, document } of documents) {
        for (const definition of document.definitions) {
            const kind = DEFINITION_KIND_NAMES.get(definition.kind);
            if (!kind || !("name" in definition) || !definition.name) {
                continue;
            }

            const name = definition.name.value;
            const line = lineForNode(definition.name);
            const previous = definitions.get(name);
            if (previous) {
                errors.push(
                    `${file}:${line}: duplicate ${kind} ${name}; already defined at ${previous.file}:${previous.line}`,
                );
                continue;
            }

            definitions.set(name, { kind, file, line });
        }
    }
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

function validateReferences(documents, definitions, errors) {
    for (const { file, document } of documents) {
        visit(document, {
            NamedType(node) {
                validateReference(
                    file,
                    lineForNode(node.name),
                    node.name.value,
                    definitions,
                    errors,
                );
            },
        });
    }
}

function main() {
    const schemaDir = parseArgs(process.argv.slice(2));
    const files = listSchemaFiles(schemaDir);

    runPrettierCheck(files);

    const documents = parseDocuments(files);
    const definitions = new Map();
    const errors = [];

    collectDefinitions(documents, definitions, errors);
    validateReferences(documents, definitions, errors);

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
