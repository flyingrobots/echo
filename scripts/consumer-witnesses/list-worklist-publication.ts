// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
/** Public Amblicit compilation only; run in the project's guarded Docker worker.
 * Node 22.18+ executes this TypeScript without a dependency installation.
 * Caller pins the compiler binary, verified source snapshot, seeds and providers
 * in a SHA-256-selected input manifest. Build provenance is supplied by the
 * caller's guarded compiler build receipt, not inferred from two hashes.
 * This runner bounds its work tree to 64 MiB / 2048 entries, each file to 1 MiB,
 * compiler stdout/stderr to 64 KiB in aggregate, and each process group to
 * 120 seconds. The outer worker guard still
 * owns aggregate project storage, memory, CPU, and host/VM free-space limits.
 */
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  copyFileSync, existsSync, lstatSync, mkdirSync, openSync, closeSync, writeSync,
  readFileSync, readdirSync, rmSync, writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { aliasSource, coordinate, invalidHelpers, pureSource, readSource } from './list-worklist-sources.ts';

type Tree = { root: string; files: Record<string, string> };
type Inputs = {
  schema: string; revision: string;
  compiler: { path: string; sha256: string };
  compilerSource: Tree; pureSeed: Tree; pureSource: string;
  readSeed: Tree; readSource: string; provider: Tree; oldProvider: Tree;
};
type Build = { exitCode: number | null; diagnostics: Record<string, unknown>[]; artifacts: Record<string, string>; stdoutSha256: string; stderrSha256: string };
const MiB = 1024 * 1024;
const outputs = ['executable-operation-package.cbor', 'verification-report.cbor'];
function requireThat(test: unknown, message: string): asserts test {
  if (!test) throw Error(message);
}
function digest(bytes: Uint8Array): string { return createHash('sha256').update(bytes).digest('hex'); }
function boundedFile(path: string, max = MiB): Buffer {
  const stat = lstatSync(path);
  requireThat(stat.isFile() && stat.size <= max, `Unbounded/nonregular input: ${path}`);
  const bytes = readFileSync(path);
  requireThat(bytes.length <= max, `Input grew above bound: ${path}`);
  return bytes;
}
function relativeName(path: string): string {
  requireThat(path.length > 0 && !isAbsolute(path) && !path.split('/').some(part => part === '..' || part === '.' || !part), 'Unsafe relative input path');
  return path;
}
function verifyTree(tree: Tree) {
  requireThat(isAbsolute(tree.root) && !lstatSync(tree.root).isSymbolicLink(), 'Explicit regular input root required');
  const names = Object.keys(tree.files);
  requireThat(names.length > 0 && names.length <= 16384, 'Invalid source inventory');
  for (const name of names) {
    relativeName(name);
    let parent = tree.root;
    for (const part of name.split('/').slice(0, -1)) {
      parent = join(parent, part);
      requireThat(lstatSync(parent).isDirectory() && !lstatSync(parent).isSymbolicLink(), 'Input parent must be a real directory');
    }
    requireThat(digest(boundedFile(join(tree.root, name), 16 * MiB)) === tree.files[name], `Selected input changed: ${name}`);
  }
}
function copyTree(tree: Tree, target: string) {
  requireThat(Object.keys(tree.files).length <= 256, 'Too many selected application/provider files');
  let size = 0;
  for (const name of Object.keys(tree.files).sort()) {
    const source = join(tree.root, relativeName(name));
    const bytes = boundedFile(source);
    size += bytes.length;
    requireThat(size <= 16 * MiB && digest(bytes) === tree.files[name], 'Selected copy changed/exceeds bound');
    const dest = join(target, name);
    mkdirSync(dirname(dest), { recursive: true });
    writeFileSync(dest, bytes, { flag: 'wx' });
  }
}
function inventory(root: string, hashes = false): Record<string, string> {
  let bytes = 0, entries = 0;
  const files: Record<string, string> = {};
  function walk(path: string, prefix: string) {
    for (const name of readdirSync(path).sort()) {
      const full = join(path, name), relative = prefix + name;
      let stat;
      try { stat = lstatSync(full); } catch (error) {
        // Compiler publication atomically removes its transaction directory.
        if ((error as NodeJS.ErrnoException).code === 'ENOENT') continue;
        throw error;
      }
      requireThat(++entries <= 2048, 'Witness entry limit exceeded');
      if (stat.isDirectory()) {
        try { walk(full, relative + '/'); } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
        }
      } else {
        requireThat(stat.isFile() && stat.size <= MiB, `Witness file limit/nonregular output: ${relative}`);
        bytes += stat.size;
        requireThat(bytes <= 64 * MiB, 'Witness exceeds 64 MiB');
        if (hashes) files[relative] = digest(boundedFile(full));
      }
    }
  }
  walk(root, '');
  return files;
}
const args = process.argv.slice(2);
requireThat(args.length === 6 && args[0] === '--inputs' && args[2] === '--sha256' && args[4] === '--work-root', 'Usage: --inputs PATH --sha256 HEX --work-root NEW_DIRECTORY');
requireThat(existsSync('/.dockerenv'), 'Run in the guarded copied-source Docker worker');
const inputPath = resolve(args[1]), selectedHash = args[3], work = resolve(args[5]);
const inputBytes = boundedFile(inputPath);
requireThat(digest(inputBytes) === selectedHash, 'Input manifest selection mismatch');
const inputs = JSON.parse(inputBytes.toString()) as Inputs;
requireThat(inputs.schema === 'echo.list-worklist-inputs/v1' && /^[a-f0-9]{40}$/.test(inputs.revision), 'Wrong input contract');
const trees = [inputs.compilerSource, inputs.pureSeed, inputs.readSeed, inputs.provider, inputs.oldProvider];
function verifyInputs() {
  requireThat(digest(boundedFile(inputPath)) === selectedHash, 'Input manifest changed');
  requireThat(digest(boundedFile(inputs.compiler.path, 256 * MiB)) === inputs.compiler.sha256, 'Compiler binary changed');
  for (const tree of trees) verifyTree(tree);
}
verifyInputs();
requireThat(!existsSync(work), 'Preserve existing witness evidence; select a new work directory');
mkdirSync(work);
writeFileSync(join(work, 'inputs.json'), inputBytes, { flag: 'wx' });
const evidence: Record<string, any> = {
  schema: 'echo.list-worklist-publication/v1', compilerRevision: inputs.revision,
  compilerBinarySha256: inputs.compiler.sha256, inputManifestSha256: selectedHash,
  results: {},
};
let childGroup: number | undefined;
let compilerLogBytes = 0;
function stopChild() {
  if (!childGroup) return;
  try { process.kill(-childGroup, 'SIGKILL'); }
  catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error; }
}
for (const signal of ['SIGTERM', 'SIGINT'] as const) process.on(signal, () => { stopChild(); process.exit(1); });
function retain() {
  inventory(work);
  writeFileSync(join(work, 'evidence.json'), JSON.stringify(evidence, null, 2) + '\n');
  inventory(work);
}
async function build(root: string, document: 'application' | 'lawpack', label: string): Promise<Build> {
  const logs = join(root, 'logs'), temporary = join(root, 'temporary');
  mkdirSync(logs, { recursive: true }); mkdirSync(temporary, { recursive: true });
  const paths = ['stdout', 'stderr'].map(stream => join(logs, `${label}.${stream}.jsonl`));
  const fds = paths.map(path => openSync(path, 'wx'));
  const counts = [0, 0];
  let failure: Error | undefined;
  const child = spawn(inputs.compiler.path, [], {
    cwd: root, detached: true, stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env, TMPDIR: temporary, TMP: temporary, TEMP: temporary, XDG_CACHE_HOME: join(temporary, 'cache') },
  });
  childGroup = child.pid;
  const started = Date.now();
  const fail = (error: unknown) => { failure ??= error instanceof Error ? error : Error(String(error)); stopChild(); };
  const monitor = setInterval(() => {
    try {
      requireThat(Date.now() - started < 120_000, 'Compiler process group exceeded 120 seconds');
      inventory(work);
    } catch (error) { fail(error); }
  }, 100);
  for (const [index, stream] of [child.stdout, child.stderr].entries()) {
    stream.on('data', (data: Buffer) => {
      try {
        counts[index] += data.length;
        compilerLogBytes += data.length;
        requireThat(compilerLogBytes <= 64 * 1024, 'Aggregate compiler log limit exceeded');
        requireThat(counts[index] <= MiB, 'Compiler stream exceeded 1 MiB');
        writeSync(fds[index], data);
      } catch (error) { fail(error); }
    });
  }
  child.stdin.on('error', fail);
  let exitCode: number | null;
  try {
    const done = new Promise<number | null>((ok, reject) => { child.once('error', reject); child.once('close', ok); });
    child.stdin.end(JSON.stringify({ schema: 'edict.compiler.settings/v1', type: 'compilerSettings', operation: 'build', [document]: `edict.${document}.json` }) + '\n');
    exitCode = await done;
  } finally {
    clearInterval(monitor); stopChild(); childGroup = undefined;
    fds.forEach(fd => closeSync(fd));
  }
  if (failure) throw failure;
  inventory(work);
  const events = paths.flatMap(path => boundedFile(path).toString().split('\n').filter(Boolean).map(line => JSON.parse(line)));
  const statuses = events.filter(event => event.type === 'status');
  requireThat(statuses.length === 1 && statuses[0].exitCode === exitCode, 'Compiler status/exit mismatch');
  const output = join(root, 'application-output');
  return { exitCode, diagnostics: events.filter(event => event.type === 'diagnostic'), artifacts: existsSync(output) ? inventory(output, true) : {}, stdoutSha256: digest(boundedFile(paths[0])), stderrSha256: digest(boundedFile(paths[1])) };
}
function succeeded(result: Build, application = true) {
  requireThat(result.exitCode === 0 && !result.diagnostics.length, `Public compiler refused: ${JSON.stringify(result)}`);
  if (application) requireThat(JSON.stringify(Object.keys(result.artifacts).sort()) === JSON.stringify(outputs), 'Expected exactly one public package/report pair');
}
function refused(result: Build, kind: string) {
  const diagnostics = [...result.diagnostics];
  const last = diagnostics.at(-1);
  if (diagnostics.length === 2 && last?.kind === 'ApplicationCompilationFailed' && last.stage === 'cli' && last.message === 'Edict application did not compile to Core') diagnostics.pop();
  requireThat(result.exitCode === 2 && !Object.keys(result.artifacts).length && diagnostics.length === 1 && diagnostics[0].kind === kind, `Wrong refusal boundary: expected ${kind}, got ${JSON.stringify(result)}`);
  if (kind === 'ProviderLowererRefused') {
    const message = String(diagnostics[0].message);
    requireThat(diagnostics[0].stage === 'cli' && message.includes('UnsupportedSemantics') && message.includes('core.required-capabilities'), `Old provider refused at the wrong boundary: ${JSON.stringify(diagnostics[0])}`);
  }
}
const pureOriginal = boundedFile(join(inputs.pureSeed.root, relativeName(inputs.pureSource))).toString();
const readOriginal = boundedFile(join(inputs.readSeed.root, relativeName(inputs.readSource))).toString();
function importLine(source: string) {
  const matches = source.match(/use lawpack jedit\.text@1 digest "sha256:[a-f0-9]{64}" as text;/g);
  requireThat(matches?.length === 1, 'Expected one pinned authored import'); return matches[0];
}
async function runCase(name: string, source: string, options: { read?: boolean; old?: boolean; nominal?: boolean; refusal?: string; coordinate?: string } = {}) {
  const root = join(work, name);
  mkdirSync(root);
  copyTree(options.read ? inputs.readSeed : inputs.pureSeed, root);
  copyTree(options.old ? inputs.oldProvider : inputs.provider, join(root, 'provider'));
  const applicationPath = join(root, 'edict.application.json');
  const application = JSON.parse(boundedFile(applicationPath).toString());
  application.sources = ['source.edict']; application.outputDirectory = 'application-output';
  application.target.providerPackage = 'provider'; application.coordinate = options.coordinate ?? coordinate;
  for (const bundle of application.lawpacks) for (const path of Object.values(bundle)) requireThat(relativeName(String(path)).startsWith('vendor/'), 'Lawpack input escapes selected vendor');
  writeFileSync(applicationPath, JSON.stringify(application, null, 2) + '\n');
  let lawpackBuild: Build | undefined;
  if (options.read) {
    const definition = join(root, 'edict.lawpack.json');
    const lawpack = JSON.parse(boundedFile(definition).toString());
    requireThat(lawpack.outputDirectory === 'vendor/atom-read' && lawpack.dependencyBundles.length === 0, 'Unexpected read lawpack seed');
    // A bounded five-byte stored atom is enough to exercise the whole read path.
    const types = lawpack.lawpack.exports.types;
    const fact = types.filter((item: any) => item.coordinate === 'jedit.text@1.FactBytes');
    requireThat(fact.length === 1 && fact[0].definition === 'Bytes<max=1048576>', 'Unexpected read byte authority');
    fact[0].definition = 'Bytes<max=8>';
    if (options.nominal) types.push({ coordinate: 'jedit.text@1.WitnessId', definition: 'Nominal<Bytes<exact=32>>' });
    const config = lawpack.lawpack.localResources.filter((item: any) => item.name === 'echo-config');
    requireThat(config.length === 1 && config[0].value.programKind === 'compiler-produced-bounded-read/v1', 'Unexpected read profile');
    config[0].value.maxReadBytes = 8; config[0].value.maxReads = 1;
    writeFileSync(definition, JSON.stringify(lawpack, null, 2) + '\n');
    lawpackBuild = await build(root, 'lawpack', 'lawpack'); succeeded(lawpackBuild, false);
    const pin = boundedFile(join(root, 'vendor/atom-read/manifest.sha256')).toString().trim();
    requireThat(/^sha256:[a-f0-9]{64}$/.test(pin), 'Invalid public lawpack digest');
    source = source.replace(importLine(source), `use lawpack jedit.text@1 digest "${pin}" as text;`);
  }
  writeFileSync(join(root, 'source.edict'), source, { flag: 'wx' });
  const result: Record<string, any> = {
    kind: 'public-compiler-output', sourceSha256: digest(Buffer.from(source)),
    applicationSha256: digest(boundedFile(applicationPath)), lawpackBuild,
    providerManifestSha256: digest(boundedFile(join(root, 'provider/provider-manifest.echo.json'))),
  };
  evidence.results[name] = result;
  result.firstBuild = await build(root, 'application', 'first'); retain();
  if (options.refusal) {
    refused(result.firstBuild, options.refusal); result.refusalBoundary = options.refusal;
  } else {
    succeeded(result.firstBuild);
    for (const name of outputs) copyFileSync(join(root, 'application-output', name), join(root, name));
    // Only this newly generated, inventoried pair is removed; retained originals stay.
    rmSync(join(root, 'application-output'), { recursive: true });
    result.repeatedBuild = await build(root, 'application', 'repeat'); succeeded(result.repeatedBuild);
    requireThat(JSON.stringify(result.firstBuild.artifacts) === JSON.stringify(result.repeatedBuild.artifacts), 'Public compiler repeat changed package/report bytes');
  }
  retain();
  console.log(`PUBLIC_WORKLIST_COMPILE_OK ${name}`);
}
try {
  // Exercise the read route first so a new cross-profile regression fails early.
  for (const count of [0, 1, 3]) await runCase(`worklist-read-${count}`, readSource(importLine(readOriginal), count), { read: true });
  await runCase('old-worklist-read', readSource(importLine(readOriginal), 3), { read: true, old: true, refusal: 'ProviderLowererRefused' });
  await runCase('worklist-read-nominal', readSource(importLine(readOriginal), 3, true), { read: true, nominal: true });
  await runCase('worklist-read-nominal-bare', readSource(importLine(readOriginal), 3, true, true), { read: true, nominal: true, refusal: 'TypeMismatch' });
  await runCase('old-control', pureOriginal, { old: true });
  await runCase('old-worklist-pure', pureSource(importLine(pureOriginal), 3), { old: true, refusal: 'ProviderLowererRefused' });
  for (const count of [0, 1, 2, 3, 5]) await runCase(`worklist-pure-${count}`, pureSource(importLine(pureOriginal), count));
  const renamed = 'consumer.worklist@1';
  await runCase('worklist-renamed', pureSource(importLine(pureOriginal), 3, renamed), { coordinate: renamed });
  await runCase('worklist-alias', aliasSource(importLine(pureOriginal)));
  for (const invalid of invalidHelpers) await runCase(`invalid-${invalid.name}`, pureSource(importLine(pureOriginal), 3) + invalid.source, { refusal: invalid.kind });
  verifyInputs();
  evidence.outcome = 'public-compiler-and-provider-verifier-accepted'; retain();
  console.log(`PUBLIC_WORKLIST_EVIDENCE_SHA256 ${digest(boundedFile(join(work, 'evidence.json')))}`);
} catch (error) {
  delete evidence.outcome; evidence.failure = String(error); retain(); throw error;
} finally { stopChild(); }
