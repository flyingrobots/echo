// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { test, expect } from '@playwright/test'
import { mkdir, mkdtemp, rm, stat, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import * as net from 'node:net'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'

type Cleanup = () => Promise<void>

async function delay(ms: number) {
  await new Promise((resolve) => setTimeout(resolve, ms))
}

// NOTE: This has an inherent TOCTOU race: the port is freed before the gateway
// binds to it, so another process could claim it between close() and spawn().
async function getFreePort(): Promise<number> {
  const server = net.createServer()
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', () => resolve())
  })
  const address = server.address()
  if (!address || typeof address === 'string') {
    server.close()
    throw new Error('failed to allocate a TCP port')
  }
  const port = address.port
  await new Promise<void>((resolve) => server.close(() => resolve()))
  return port
}

async function waitForFile(path: string, timeoutMs: number) {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (existsSync(path)) {
      try {
        const s = await stat(path)
        if (s.isSocket()) return
      } catch {
        // keep polling
      }
    }
    await delay(50)
  }
  throw new Error(`timed out waiting for file: ${path}`)
}

async function waitForHttpOk(url: string, timeoutMs: number) {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(url, { cache: 'no-store' })
      if (res.ok) return
    } catch {
      // keep polling
    }
    await delay(100)
  }
  throw new Error(`timed out waiting for HTTP ok: ${url}`)
}

function spawnLogged(
  label: string,
  cmd: string,
  args: string[],
  env: NodeJS.ProcessEnv,
): { child: ChildProcessWithoutNullStreams; stop: Cleanup; getOutput: () => string } {
  const child = spawn(cmd, args, { env, stdio: 'pipe' })
  let out = ''
  const onData = (chunk: unknown) => {
    out += String(chunk)
    if (out.length > 128_000) out = out.slice(out.length - 128_000)
  }
  child.stdout.on('data', onData)
  child.stderr.on('data', onData)

  const stop: Cleanup = async () => {
    if (child.exitCode != null) return
    child.kill('SIGINT')
    const graceful = await Promise.race([
      new Promise<boolean>((resolve) => child.once('exit', () => resolve(true))),
      delay(1500).then(() => false),
    ])
    if (!graceful && child.exitCode == null) {
      child.kill('SIGKILL')
      await Promise.race([new Promise<void>((resolve) => child.once('exit', () => resolve())), delay(1500)])
    }
  }

  return {
    child,
    stop,
    getOutput: () => `[${label}] ${out}`,
  }
}

test.describe('Session Dashboard (gateway + hub observer)', () => {
  test('renders and shows observed warp activity', async ({ page }) => {
    test.setTimeout(180_000)

    let root: string
    try {
      root = await mkdtemp('/tmp/echo-e2e-')
    } catch {
      root = await mkdtemp(join(tmpdir(), 'echo-e2e-'))
    }
    const runtimeDir = join(root, 'runtime')
    const homeDir = join(root, 'home')
    await mkdir(runtimeDir, { recursive: true })
    await mkdir(homeDir, { recursive: true })

    const xdgRuntimeDir = runtimeDir
    const socketPath = join(xdgRuntimeDir, 'echo-session.sock')
    const port = await getFreePort()
    const baseUrl = `http://127.0.0.1:${port}`

    const env: NodeJS.ProcessEnv = {
      ...process.env,
      // keep hub config + socket location isolated from the developer machine
      HOME: homeDir,
      XDG_RUNTIME_DIR: xdgRuntimeDir,
      RUST_LOG: process.env.RUST_LOG ?? 'info',
      CARGO_TERM_COLOR: 'never',
    }

    const cleanups: Cleanup[] = []
    const pushCleanup = (fn: Cleanup) => cleanups.unshift(fn)
    const cleanupAll = async () => {
      for (const fn of cleanups) {
        try {
          await fn()
        } catch {
          // best-effort cleanup; keep going
        }
      }
      await rm(root, { recursive: true, force: true })
    }

    let hubGetOutput: (() => string) | null = null
    let gatewayGetOutput: (() => string) | null = null

    try {
      // Build once so we don't fight Cargo locks across multiple long-lived processes.
      {
        const build = spawnLogged(
          'cargo-build',
          'cargo',
          ['build', '-p', 'echo-session-service', '-p', 'echo-session-ws-gateway'],
          env,
        )
        pushCleanup(build.stop)
        const exitCode: number = await new Promise((resolve) => build.child.once('exit', resolve))
        if (exitCode !== 0) {
          throw new Error(`cargo build failed (${exitCode})\n${build.getOutput()}`)
        }
      }

      {
        const buildExample = spawnLogged(
          'cargo-build-example',
          'cargo',
          ['build', '-p', 'echo-session-client', '--example', 'publish_pulse'],
          env,
        )
        pushCleanup(buildExample.stop)
        const exitCode: number = await new Promise((resolve) => buildExample.child.once('exit', resolve))
        if (exitCode !== 0) {
          throw new Error(`cargo build (publish_pulse) failed (${exitCode})\n${buildExample.getOutput()}`)
        }
      }

      const hub = spawnLogged('hub', 'target/debug/echo-session-service', [], env)
      pushCleanup(hub.stop)
      hubGetOutput = hub.getOutput

      await waitForFile(socketPath, 20_000)

      const gateway = spawnLogged(
        'gateway',
        'target/debug/echo-session-ws-gateway',
        ['--listen', `127.0.0.1:${port}`, '--unix-socket', socketPath, '--observe-warp', '1'],
        env,
      )
      pushCleanup(gateway.stop)
      gatewayGetOutput = gateway.getOutput

      await waitForHttpOk(`${baseUrl}/api/metrics`, 20_000)

      await page.goto(`${baseUrl}/dashboard`)

      await expect(page.locator('h1')).toHaveText(/Session Dashboard/i)
      await expect(page.locator('#status-pill')).toHaveText('ok', { timeout: 20_000 })

      // D3 is loaded lazily from the gateway; validate that endpoint works.
      await page.waitForFunction(() => typeof (window as any).d3 !== 'undefined', undefined, { timeout: 20_000 })

      // Ensure the hub observer connects.
      await expect(page.locator('#o-state')).toHaveText('connected', { timeout: 20_000 })

      // Drive traffic: snapshot + gapless diffs into the hub (via UDS), which the gateway observer should see.
      {
        const pulse = spawnLogged(
          'pulse',
          'target/debug/examples/publish_pulse',
          [socketPath, '1', '8', '60'],
          env,
        )
        const exitCode: number = await new Promise((resolve) => pulse.child.once('exit', resolve))
        if (exitCode !== 0) {
          throw new Error(`publish_pulse failed (${exitCode})\n${pulse.getOutput()}`)
        }
      }

      // Poll metrics until we see warp activity reflected.
      let lastMetrics: any = null
      let observed = false
      const start = Date.now()
      while (Date.now() - start < 20_000) {
        const res = await fetch(`${baseUrl}/api/metrics`, { cache: 'no-store' })
        const m: any = await res.json()
        lastMetrics = m
        const w1 = (m.warps || []).find((w: any) => w.warp_id === 1)
        if (w1 && w1.snapshot_count >= 1 && w1.diff_count >= 1 && (w1.last_epoch == null || w1.last_epoch >= 1)) {
          expect(m.hub_observer).toBeTruthy()
          expect(m.hub_observer.enabled).toBe(true)
          expect(m.hub_observer.rx_frames).toBeGreaterThan(0)
          observed = true
          break
        }
        await delay(150)
      }
      if (!observed) {
        throw new Error(`timed out waiting for warp activity in /api/metrics (last=${JSON.stringify(lastMetrics)})`)
      }

      // UI should render the warp row once D3 is present.
      await expect(page.locator('#warp-rows tr')).toHaveCount(1, { timeout: 20_000 })
      await expect(page.locator('#warp-rows tr').first()).toContainText('1')

      // Capture a screenshot for the Playwright report (always), and optionally
      // write it into docs/assets when explicitly requested.
      const screenshotPng = await page.screenshot({ fullPage: true, type: 'png' })
      await test.info().attach('session-dashboard', { body: screenshotPng, contentType: 'image/png' })
      if (process.env.ECHO_CAPTURE_DASHBOARD_SCREENSHOT === '1') {
        const outDir = 'docs/assets/wvp'
        const outPath = `${outDir}/session-dashboard.png`
        await mkdir(outDir, { recursive: true })
        await writeFile(outPath, screenshotPng)
      }
    } catch (err) {
      const extra = `\n\n--- hub output ---\n${hubGetOutput?.() ?? ''}\n\n--- gateway output ---\n${
        gatewayGetOutput?.() ?? ''
      }\n`
      throw new Error(`${String(err)}${extra}`)
    } finally {
      await cleanupAll()
    }
  })
})
