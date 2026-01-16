import fs from "node:fs";
import { execSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = process.env.DIND_ROOT || path.resolve(SCRIPT_DIR, "..");
const MANIFEST_PATH = path.resolve(ROOT_DIR, "testdata/dind/MANIFEST.json");

function loadManifest() {
    return JSON.parse(fs.readFileSync(MANIFEST_PATH, "utf8"));
}

function parseArgs() {
    const args = process.argv.slice(2);
    const config = {
        tags: [],
        excludeTags: [],
        mode: "run", // run | torture
        runs: 20, // for torture
        emitRepro: false
    };

    for (let i = 0; i < args.length; i++) {
        const arg = args[i];
        if (arg === "--tags") {
            config.tags = args[++i].split(",");
        } else if (arg === "--exclude-tags") {
            config.excludeTags = args[++i].split(",");
        } else if (arg === "--mode") {
            config.mode = args[++i];
        } else if (arg === "--runs") {
            config.runs = parseInt(args[++i], 10);
        } else if (arg === "--emit-repro") {
            config.emitRepro = true;
        }
    }
    return config;
}

function matches(scenario, config) {
    if (config.tags.length > 0) {
        if (!config.tags.some(t => scenario.tags.includes(t))) return false;
    }
    if (config.excludeTags.length > 0) {
        if (config.excludeTags.some(t => scenario.tags.includes(t))) return false;
    }
    return true;
}

function main() {
    const manifest = loadManifest();
    const config = parseArgs();
    
    console.log(`DIND SUITE: Mode=${config.mode}, Tags=${config.tags.join(",") || "ALL"}`);
    
    let failedCount = 0;
    const results = [];
    
    for (const scenario of manifest) {
        if (!matches(scenario, config)) continue;
        
        console.log(`\n>>> Running: ${scenario.desc} (${scenario.path})`);
        
        const scenarioPath = path.resolve(ROOT_DIR, "testdata/dind", scenario.path);
        let cmd = `cargo run -p echo-dind-harness --quiet -- ${config.mode}`;
        cmd += ` ${scenarioPath}`;
        
        if (config.mode === "run") {
             // Look for golden
             const golden = path.resolve(
                ROOT_DIR,
                "testdata/dind",
                scenario.path.replace(".eintlog", ".hashes.json")
             );
             if (fs.existsSync(golden)) {
                 cmd += ` --golden ${golden}`;
             }
        } else if (config.mode === "torture") {
            cmd += ` --runs ${config.runs}`;
        }
        
        if (config.emitRepro) {
            const reproDir = path.resolve(
                ROOT_DIR,
                "test-results/dind",
                scenario.path.replace(".eintlog", "")
            );
            // cleanup old repro
             if (fs.existsSync(reproDir)) {
                fs.rmSync(reproDir, { recursive: true, force: true });
            }
            cmd += ` --emit-repro ${reproDir}`;
        }

        const start = performance.now();
        let passed = false;
        try {
            execSync(cmd, { stdio: "inherit" });
            passed = true;
        } catch (e) {
            console.error(`!!! FAILED: ${scenario.desc}`);
            failedCount++;
        }
        const duration = performance.now() - start;
        
        results.push({
            scenario: scenario.path,
            desc: scenario.desc,
            duration_ms: Math.round(duration),
            passed,
            tags: scenario.tags,
            mode: config.mode
        });
    }
    
    fs.writeFileSync("dind-report.json", JSON.stringify(results, null, 2));
    console.log(`\nWrote dind-report.json`);
    
    if (failedCount > 0) {
        console.error(`\nDIND SUITE FAILED: ${failedCount} scenarios failed.`);
        process.exit(1);
    } else {
        console.log(`\nDIND SUITE PASSED.`);
    }
}

main();
