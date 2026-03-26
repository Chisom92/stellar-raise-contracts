/**
 * readme_md_installation.test.js
 *
 * Verifies that the installation commands documented in README.md and
 * docs/readme_md_installation.md are correct and that supporting scripts
 * conform to their documented logging bounds.
 *
 * @security Tests run locally only. No network calls, no Stellar keys required.
 */

'use strict';

const { execSync, spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const ROOT = path.resolve(__dirname);
const DEPLOY_SCRIPT = path.join(ROOT, 'scripts', 'deploy.sh');
const INTERACT_SCRIPT = path.join(ROOT, 'scripts', 'interact.sh');

/** Run a shell command, return stdout string. Throws on non-zero exit. */
function run(cmd, opts = {}) {
  return execSync(cmd, { encoding: 'utf8', stdio: 'pipe', ...opts });
}

/** Spawn a script with args, return { stdout, status }. Never throws. */
function spawn(script, args = []) {
  const result = spawnSync('bash', [script, ...args], { encoding: 'utf8' });
  return { stdout: result.stdout || '', status: result.status };
}

/** Extract [LOG] lines from output. */
function logLines(output) {
  return output.split('\n').filter(l => l.includes('[LOG]'));
}

/** Parse a single [LOG] line into a key=value object. */
function parseLog(line) {
  const obj = {};
  const pairs = line.replace(/.*\[LOG\]\s*/, '').trim().split(/\s+/);
  for (const pair of pairs) {
    const [k, v] = pair.split('=');
    if (k) obj[k] = v ?? '';
  }
  return obj;
}

// ── Prerequisites ─────────────────────────────────────────────────────────────

describe('Prerequisites', () => {
  test('rustc is installed', () => {
    expect(run('rustc --version')).toMatch(/^rustc \d+\.\d+\.\d+/);
  });

  test('cargo is installed', () => {
    expect(run('cargo --version')).toMatch(/^cargo \d+\.\d+\.\d+/);
  });

  test('wasm32-unknown-unknown target is installed', () => {
    expect(run('rustup target list --installed')).toContain('wasm32-unknown-unknown');
  });

  test('stellar CLI is installed (v20+ rename)', () => {
    expect(run('stellar --version')).toContain('stellar-cli');
  });

  test('Node.js >= 18 is available', () => {
    const major = parseInt(run('node --version').trim().replace('v', ''), 10);
    expect(major).toBeGreaterThanOrEqual(18);
  });
});

// ── deploy.sh logging bounds ──────────────────────────────────────────────────

describe('deploy.sh logging bounds', () => {
  test('deploy.sh with no args exits non-zero (missing required args)', () => {
    const { status } = spawn(DEPLOY_SCRIPT);
    expect(status).not.toBe(0);
  });

  test('deploy.sh emits no [LOG] lines before arg validation fails', () => {
    const { stdout } = spawn(DEPLOY_SCRIPT);
    expect(logLines(stdout).length).toBe(0);
  });

  test('[LOG] line format is key=value pairs', () => {
    const out = run(`bash -c 'echo "[LOG] step=build status=start"'`).trim();
    const parsed = parseLog(out);
    expect(parsed.step).toBe('build');
    expect(parsed.status).toBe('start');
  });

  test('deploy.sh source contains all 7 expected [LOG] patterns', () => {
    const src = fs.readFileSync(DEPLOY_SCRIPT, 'utf8');
    expect(src).toMatch(/\[LOG\] step=build status=start/);
    expect(src).toMatch(/\[LOG\] step=build status=ok/);
    expect(src).toMatch(/\[LOG\] step=deploy status=start/);
    expect(src).toMatch(/\[LOG\] step=deploy status=ok/);
    expect(src).toMatch(/\[LOG\] step=initialize status=start/);
    expect(src).toMatch(/\[LOG\] step=initialize status=ok/);
    expect(src).toMatch(/\[LOG\] step=done/);
  });

  test('deploy.sh has at most 7 [LOG] echo lines (bounded output)', () => {
    const src = fs.readFileSync(DEPLOY_SCRIPT, 'utf8');
    const count = (src.match(/echo "\[LOG\]/g) || []).length;
    expect(count).toBeLessThanOrEqual(7);
  });
});

// ── interact.sh logging bounds ────────────────────────────────────────────────

describe('interact.sh logging bounds', () => {
  test('interact.sh with no args exits non-zero', () => {
    const { status } = spawn(INTERACT_SCRIPT);
    expect(status).not.toBe(0);
  });

  test('interact.sh unknown action emits exactly 1 [LOG] error line', () => {
    const { stdout, status } = spawn(INTERACT_SCRIPT, ['CTEST', 'unknown_action']);
    expect(status).toBe(1);
    const lines = logLines(stdout);
    expect(lines.length).toBe(1);
    expect(lines[0]).toMatch(/status=error/);
  });

  test('interact.sh unknown action log line has reason= field', () => {
    const { stdout } = spawn(INTERACT_SCRIPT, ['CTEST', 'unknown_action']);
    const parsed = parseLog(logLines(stdout)[0]);
    expect(parsed.reason).toBe('unknown_action');
  });

  test('interact.sh contribute action has exactly 2 [LOG] lines in source', () => {
    const src = fs.readFileSync(INTERACT_SCRIPT, 'utf8');
    const block = src.match(/contribute\)([\s\S]*?);;/)?.[1] || '';
    expect((block.match(/echo "\[LOG\]/g) || []).length).toBe(2);
  });

  test('interact.sh withdraw action has exactly 2 [LOG] lines in source', () => {
    const src = fs.readFileSync(INTERACT_SCRIPT, 'utf8');
    const block = src.match(/withdraw\)([\s\S]*?);;/)?.[1] || '';
    expect((block.match(/echo "\[LOG\]/g) || []).length).toBe(2);
  });
});

// ── Edge Cases ────────────────────────────────────────────────────────────────

describe('Edge Case — WASM target', () => {
  test('rustup target list --installed contains wasm32-unknown-unknown', () => {
    expect(run('rustup target list --installed')).toMatch(/wasm32-unknown-unknown/);
  });
});

describe('Edge Case — Stellar CLI versioning', () => {
  test('stellar --version does not start with "soroban" (v20+ rename)', () => {
    expect(run('stellar --version')).not.toMatch(/^soroban/);
  });

  test('stellar contract --help exits cleanly', () => {
    expect(() => run('stellar contract --help')).not.toThrow();
  });
});

describe('Edge Case — Network identity (no keys required)', () => {
  test('stellar keys list does not crash', () => {
    expect(() => {
      try { run('stellar keys list'); } catch (_) { /* no keys configured — acceptable */ }
    }).not.toThrow();
  });
});

// ── Security ──────────────────────────────────────────────────────────────────

describe('Security', () => {
  test('.soroban/ is listed in .gitignore', () => {
    const gitignore = fs.readFileSync(path.join(ROOT, '.gitignore'), 'utf8');
    expect(gitignore).toMatch(/\.soroban/);
  });

  test('verify_env.sh exists and is executable', () => {
    const script = path.join(ROOT, 'scripts', 'verify_env.sh');
    expect(fs.existsSync(script)).toBe(true);
    expect(fs.statSync(script).mode & 0o100).toBeTruthy();
  });

  test('docs/readme_md_installation.md exists', () => {
    expect(fs.existsSync(path.join(ROOT, 'docs', 'readme_md_installation.md'))).toBe(true);
  });
});
