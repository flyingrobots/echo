// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// Fault fixtures only: this suite never claims successful public compilation.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const runner = fileURLToPath(new URL('./list-worklist-publication.ts', import.meta.url));
const digest = (bytes: Buffer) => createHash('sha256').update(bytes).digest('hex');
const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
function killGroup(pid?: number) {
  if (!pid) return;
  try { process.kill(-pid, 'SIGKILL'); }
  catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error; }
}
for (const scenario of ['relative', 'SIGTERM', 'SIGINT', 'output-limit'] as const) {
  test(`public witness rejects or records ${scenario}`, { timeout: 15_000 }, async () => {
    assert.ok(existsSync('/.dockerenv'), 'Run these fixtures in the guarded Docker worker');
    const root = mkdtempSync(join(tmpdir(), 'worklist-failure-'));
    const work = join(root, 'work');
    const compiler = join(root, 'compiler.ts');
    const pidPath = join(work, 'worklist-read-0', 'fixture.pid');
    let childPid: number | undefined;
    try {
      writeFileSync(compiler, `#!${process.execPath}\nimport {writeFileSync} from 'node:fs';\nwriteFileSync('fixture.pid', String(process.pid));\n${scenario === 'output-limit' ? "writeFileSync('oversize.bin', Buffer.alloc(1024*1024+1));" : ''}\nsetInterval(() => {}, 1000);\n`, { mode: 0o755 });
      const source = `use lawpack jedit.text@1 digest "sha256:${'0'.repeat(64)}" as text;\n`;
      function tree(name: string, files: Record<string, string>) {
        const dir = join(root, name); mkdirSync(dir);
        for (const [path, content] of Object.entries(files)) writeFileSync(join(dir, path), content);
        return { root: dir, files: Object.fromEntries(Object.entries(files).map(([path, content]) => [path, digest(Buffer.from(content))])) };
      }
      const seed = tree('seed', {
        'source.edict': source,
        'edict.application.json': JSON.stringify({ target: {}, lawpacks: [] }),
        'edict.lawpack.json': JSON.stringify({ outputDirectory: 'vendor/atom-read', dependencyBundles: [], lawpack: { exports: { types: [{ coordinate: 'jedit.text@1.FactBytes', definition: 'Bytes<max=1048576>' }] }, localResources: [{ name: 'echo-config', value: { programKind: 'compiler-produced-bounded-read/v1' } }] } }),
      });
      const provider = tree('provider', { 'provider-manifest.echo.json': '{}' });
      const input = join(root, 'input.json');
      const inputBytes = Buffer.from(JSON.stringify({
        schema: 'echo.list-worklist-inputs/v1', revision: '0'.repeat(40),
        compiler: { path: scenario === 'relative' ? './compiler.ts' : compiler, sha256: digest(readFileSync(compiler)) },
        compilerSource: { root, files: { 'compiler.ts': digest(readFileSync(compiler)) } },
        pureSeed: seed, readSeed: seed, pureSource: 'source.edict', readSource: 'source.edict', provider, oldProvider: provider,
      }));
      writeFileSync(input, inputBytes);
      const child = spawn(process.execPath, [runner, '--inputs', input, '--sha256', digest(inputBytes), '--work-root', work], { cwd: root, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
      childPid = child.pid;
      let output = '';
      for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => {
        output += bytes.toString();
        if (output.length > 64 * 1024) { killGroup(child.pid); output = output.slice(0, 64 * 1024); }
      });
      const closed = new Promise<number | null>((resolve, reject) => { child.once('error', reject); child.once('close', resolve); });
      const timeout = setTimeout(() => killGroup(child.pid), 10_000);
      let code: number | null;
      try {
        if (scenario === 'SIGTERM' || scenario === 'SIGINT') {
          const deadline = Date.now() + 5_000;
          while (!existsSync(pidPath) && Date.now() < deadline) await sleep(20);
          assert.ok(existsSync(pidPath), 'Fault compiler must start before interruption');
          child.kill(scenario);
        }
        code = await closed;
      } finally { clearTimeout(timeout); }
      assert.equal(code, 1, output);
      if (scenario === 'relative') {
        assert.match(output, /Absolute compiler path required/);
        assert.equal(existsSync(work), false, 'Invalid compiler selection must fail before output creation');
      } else {
        const receipt = join(work, 'evidence.json');
        assert.ok(existsSync(receipt), `${scenario} must retain failure evidence`);
        const bytes = readFileSync(receipt);
        assert.ok(bytes.length <= 1024 * 1024, 'Failure receipt remains bounded');
        const evidence = JSON.parse(bytes.toString());
        assert.equal(evidence.outcome, undefined);
        assert.match(evidence.failure, scenario === 'output-limit' ? /Witness file limit\/nonregular output:.*oversize.bin/ : new RegExp(scenario));
        if (existsSync(pidPath)) {
          const pid = Number(readFileSync(pidPath, 'utf8'));
          const stat = `/proc/${pid}/stat`;
          assert.ok(!existsSync(stat) || /^\d+ \(.+\) Z /.test(readFileSync(stat, 'utf8')), 'Compiler must be terminated');
        }
      }
    } finally {
      killGroup(childPid);
      if (existsSync(pidPath)) killGroup(Number(readFileSync(pidPath, 'utf8')));
      rmSync(root, { recursive: true, force: true });
    }
  });
}
