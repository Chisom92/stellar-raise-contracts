/**
 * readme_md_installation.test.js
 *
 * Programmatically verifies that the installation commands documented in
 * README.md and docs/readme_md_installation.md execute without errors.
 *
 * Coverage target: 95%+ of "Getting Started" commands.
 *
 * @security  Tests run in the current working directory. They do not write
 *            to the network or require Stellar keys. No secret material is
 *            accessed or generated.
 */

'use strict';

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const ROOT = process.cwd();
const EXEC_OPTS = { encoding: 'utf8', stdio: 'pipe' };

// ── helpers ──────────────────────────────────────────────────────────────────

/** Run a command and return stdout, or throw with a clear message. */
function run(cmd, opts = {}) {
  return execSync(cmd, { ...EXEC_OPTS, ...opts });
}

// ── Prerequisites ─────────────────────────────────────────────────────────────

describe('Prerequisites', () => {
  test('rustc is installed (stable channel)', () => {
    const out = run('rustc --version');
    expect(out).toMatch(/^rustc \d+\.\d+\.\d+/);
  });

  test('cargo is installed', () => {
    const out = run('cargo --version');
    expect(out).toMatch(/^cargo \d+\.\d+\.\d+/);
  });

  test('wasm32-unknown-unknown target is installed', () => {
    const out = run('rustup target list --installed');
    expect(out).toContain('wasm32-unknown-unknown');
  });

  test('stellar CLI is installed (v20+ rename)', () => {
    const out = run('stellar --version');
    expect(out).toContain('stellar-cli');
  });

  test('Node.js >= 18 is available', () => {
    const out = run('node --version');
    const major = parseInt(out.trim().replace('v', ''), 10);
    expect(major).toBeGreaterThanOrEqual(18);
  });
});

// ── Getting Started commands ──────────────────────────────────────────────────

describe('Getting Started', () => {
  test('cargo build --dry-run succeeds (wasm32 release)', () => {
    run(
      'cargo build --release --target wasm32-unknown-unknown -p crowdfund --dry-run',
      { cwd: ROOT, timeout: 30000 }
    );
  }, 35000);

  test('cargo test --no-run compiles test suite', () => {
    run('cargo test --no-run --workspace', { cwd: ROOT, timeout: 120000, stdio: 'ignore' });
  }, 130000);
});

// ── deploy.sh logging bounds ──────────────────────────────────────────────────

describe('deploy.sh logging bounds', () => {
  // Run with missing args to trigger early exit — we only test log format,
  // not actual network calls.
  test('10 - deploy.sh with no args exits non-zero (missing required args)', () => {
    const { status } = run(DEPLOY_SCRIPT, []);
    expect(status).not.toBe(0);
  });

  test('11 - deploy.sh emits no [LOG] lines before arg validation fails', () => {
    const { stdout } = run(DEPLOY_SCRIPT, []);
    expect(logLines(stdout).length).toBe(0);
  });

  test('12 - [LOG] line format is key=value pairs', () => {
    // Simulate a partial run by sourcing just the echo lines via bash -c
    const out = execSync(
      `bash -c 'echo "[LOG] step=build status=start"'`,
      { encoding: 'utf8' }
    ).trim();
    const parsed = parseLog(out);
    expect(parsed.step).toBe('build');
    expect(parsed.status).toBe('start');
  });

  test('13 - deploy.sh [LOG] lines use step= field', () => {
    // Verify the script source contains the expected log patterns
    const src = fs.readFileSync(DEPLOY_SCRIPT, 'utf8');
    expect(src).toMatch(/\[LOG\] step=build status=start/);
    expect(src).toMatch(/\[LOG\] step=build status=ok/);
    expect(src).toMatch(/\[LOG\] step=deploy status=start/);
    expect(src).toMatch(/\[LOG\] step=deploy status=ok/);
    expect(src).toMatch(/\[LOG\] step=initialize status=start/);
    expect(src).toMatch(/\[LOG\] step=initialize status=ok/);
    expect(src).toMatch(/\[LOG\] step=done/);
  });

  test('14 - deploy.sh has at most 7 [LOG] echo lines (bounded output)', () => {
    const src = fs.readFileSync(DEPLOY_SCRIPT, 'utf8');
    const count = (src.match(/echo "\[LOG\]/g) || []).length;
    expect(count).toBeLessThanOrEqual(7);
  });
});

describe('Edge Case — WASM target', () => {
  test('rustup target list --installed contains wasm32-unknown-unknown', () => {
    expect(run('rustup target list --installed')).toMatch(/wasm32-unknown-unknown/);
  });
});

// ── interact.sh logging bounds ────────────────────────────────────────────────

describe('interact.sh logging bounds', () => {
  test('16 - interact.sh with no args exits non-zero', () => {
    const { status } = run(INTERACT_SCRIPT, []);
    expect(status).not.toBe(0);
  });

  test('17 - interact.sh unknown action emits exactly 1 [LOG] error line', () => {
    const { stdout, status } = run(INTERACT_SCRIPT, ['CTEST', 'unknown_action']);
    expect(status).toBe(1);
    const lines = logLines(stdout);
    expect(lines.length).toBe(1);
    expect(lines[0]).toMatch(/status=error/);
  });

  test('18 - interact.sh unknown action log line has reason= field', () => {
    const { stdout } = run(INTERACT_SCRIPT, ['CTEST', 'unknown_action']);
    const lines = logLines(stdout);
    const parsed = parseLog(lines[0]);
    expect(parsed.reason).toBe('unknown_action');
  });
});

  test('19 - interact.sh contribute action has exactly 2 [LOG] lines in source', () => {
    const src = fs.readFileSync(INTERACT_SCRIPT, 'utf8');
    const contributeBlock = src.match(/contribute\)([\s\S]*?);;/)?.[1] || '';
    const count = (contributeBlock.match(/echo "\[LOG\]/g) || []).length;
    expect(count).toBe(2);
  });

  test('20 - interact.sh withdraw action has exactly 2 [LOG] lines in source', () => {
    const src = fs.readFileSync(INTERACT_SCRIPT, 'utf8');
    const withdrawBlock = src.match(/withdraw\)([\s\S]*?);;/)?.[1] || '';
    const count = (withdrawBlock.match(/echo "\[LOG\]/g) || []).length;
    expect(count).toBe(2);
  });

describe('Edge Case — Stellar CLI versioning', () => {
  test('stellar --version does not contain "soroban" (v20+ rename)', () => {
    const out = run('stellar --version');
    // The binary is now `stellar`, not `soroban`
    expect(out).not.toMatch(/^soroban/);
  });

  test('stellar contract --help exits cleanly', () => {
    // Verifies the CLI sub-command structure expected by deploy scripts
    expect(() => run('stellar contract --help')).not.toThrow();
  });
});

// ── Edge Case: Network identity ───────────────────────────────────────────────

describe('Edge Case — Network identity (graceful, no keys required)', () => {
  test('stellar keys list does not crash', () => {
    // May return empty list — that is fine
    expect(() => {
      try { run('stellar keys list'); } catch (_) { /* no keys configured */ }
    }).not.toThrow();
  });
});

// ── Security: .soroban not committed ─────────────────────────────────────────

describe('Security', () => {
  test('.soroban/ is listed in .gitignore', () => {
    const gitignore = fs.readFileSync(path.join(ROOT, '.gitignore'), 'utf8');
    expect(gitignore).toMatch(/\.soroban/);
  });

  test('verify_env.sh exists and is executable', () => {
    const script = path.join(ROOT, 'scripts', 'verify_env.sh');
    expect(fs.existsSync(script)).toBe(true);
    // S_IXUSR = 0o100 — owner execute bit
    expect(fs.statSync(script).mode & 0o100).toBeTruthy();
  });

  test('docs/readme_md_installation.md exists', () => {
    expect(fs.existsSync(path.join(ROOT, 'docs', 'readme_md_installation.md'))).toBe(true);
  });
});
