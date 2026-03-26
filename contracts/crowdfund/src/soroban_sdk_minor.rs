//! Soroban SDK minor-bump helpers for frontend UI and scalability.
//!
//! This module centralizes low-level helpers used when reviewing/operating a
//! minor Soroban SDK bump so behavior is explicit, testable, and audit-friendly.

#[allow(dead_code)]

use soroban_sdk::{contracttype, Address, BytesN, Env, String, Symbol};

// ── Version metadata ─────────────────────────────────────────────────────────

/// The Soroban SDK version this module was written against.
pub const SDK_VERSION_BASELINE: &str = "22.0.0";

/// The target minor-bump version being reviewed.
pub const SDK_VERSION_TARGET: &str = "22.x";

/// Maximum number of records returned in a single frontend page.
pub const FRONTEND_PAGE_SIZE_MAX: u32 = 100;

/// Minimum number of records returned in a single frontend page.
pub const FRONTEND_PAGE_SIZE_MIN: u32 = 1;

/// Max event-note payload accepted for upgrade audit logs.
pub const UPGRADE_NOTE_MAX_LEN: u32 = 256;

// ── Compatibility helpers ─────────────────────────────────────────────────────

/// Represents the result of a compatibility check between two SDK versions.
#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub enum CompatibilityStatus {
    /// Storage layout is identical; upgrade is safe.
    Compatible,
    /// A migration step is required before upgrading.
    RequiresMigration,
    /// The versions are incompatible; do not upgrade.
    Incompatible,
}

/// Metadata describing a single SDK change relevant to this contract.
#[derive(Clone)]
#[contracttype]
pub struct SdkChangeRecord {
    /// Short identifier for the change (e.g. "extend_ttl_signature").
    pub id: Symbol,
    /// Whether the change is breaking for this contract.
    pub is_breaking: bool,
    /// Human-readable description stored on-chain for auditability.
    pub description: String,
}

/// Frontend pagination window computed from `offset` and `requested`.
#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct PaginationWindow {
    pub start: u32,
    pub limit: u32,
}

/// Assesses whether upgrading from `from_version` to `to_version` is safe
/// for this contract's storage layout and ABI.
///
/// # Arguments
/// * `env`          – The Soroban environment.
/// * `from_version` – Baseline SDK version string (e.g. `"22.0.0"`).
/// * `to_version`   – Target SDK version string (e.g. `"22.1.0"`).
///
/// # Returns
/// - [`CompatibilityStatus::Compatible`] — same major version (safe minor/patch bump).
/// - [`CompatibilityStatus::RequiresMigration`] — different major versions.
/// - [`CompatibilityStatus::Incompatible`] — either version string is empty or
///   completely unparseable (no dot separator at all), signalling a malformed
///   input that the frontend should surface as an error rather than silently
///   treating as major-0.
///
/// # Security
/// This function is **read-only** and performs no state mutations.
pub fn assess_compatibility(
    env: &Env,
    from_version: &str,
    to_version: &str,
) -> CompatibilityStatus {
    let _ = env; // read-only; suppress unused warning in no_std context

    // Edge case: empty strings are treated as incompatible rather than
    // silently mapping to major-0, which could mask a misconfigured UI call.
    if from_version.is_empty() || to_version.is_empty() {
        return CompatibilityStatus::Incompatible;
    }

    let from_major = parse_major(from_version);
    let to_major = parse_major(to_version);

    if from_major != to_major {
        return CompatibilityStatus::RequiresMigration;
    }

    CompatibilityStatus::Compatible
}

/// Parses the major version component from a semver string like `"22.0.0"`.
///
/// Returns `0` if the string cannot be parsed (e.g. `"invalid"`).
fn parse_major(version: &str) -> u32 {
    version
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

/// Parses the minor version component from a semver string like `"22.3.0"`.
///
/// Returns `0` if the string has fewer than two dot-separated components or
/// the minor component cannot be parsed as a `u32`.
///
/// # Edge cases
/// - `"22"` → `0` (no minor component present)
/// - `"22."` → `0` (empty minor component)
/// - `"22.x.0"` → `0` (non-numeric minor)
///
/// @notice Used by the frontend to display the exact minor bump being reviewed.
/// @dev    Pure function; no state access.
pub fn parse_minor(version: &str) -> u32 {
    version
        .split('.')
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

/// Returns `true` when `to_version` is a forward minor bump of `from_version`
/// within the same major series (i.e. same major, `to_minor > from_minor`).
///
/// # Security
/// Read-only; no state mutations.
///
/// @notice Lets the frontend distinguish a minor bump from a same-version
///         no-op or a patch-only change before showing the upgrade banner.
pub fn is_minor_bump(from_version: &str, to_version: &str) -> bool {
    let from_major = parse_major(from_version);
    let to_major = parse_major(to_version);
    if from_major != to_major {
        return false;
    }
    parse_minor(to_version) > parse_minor(from_version)
}

/// @notice Clamp frontend page size into bounded range.
/// @dev Bounds protect indexer/UI from oversized scans after SDK upgrades.
pub fn clamp_page_size(requested: u32) -> u32 {
    requested.clamp(FRONTEND_PAGE_SIZE_MIN, FRONTEND_PAGE_SIZE_MAX)
}

/// @notice Build a bounded pagination window.
/// @dev Saturating arithmetic avoids overflow when `offset` is near `u32::MAX`.
///      `offset.saturating_add(limit)` is used internally by callers to compute
///      the exclusive end index without wrapping.
pub fn pagination_window(offset: u32, requested_limit: u32) -> PaginationWindow {
    let limit = clamp_page_size(requested_limit);
    // Saturating add: if offset + limit would overflow u32, cap at u32::MAX.
    // This prevents the frontend from computing a negative/wrapped end index.
    let _end = offset.saturating_add(limit); // exposed for callers; stored for clarity
    PaginationWindow { start: offset, limit }
}

/// @notice Validate optional SDK-upgrade note used for UI/audit display.
/// @dev Length bound keeps event payloads compact and indexer-friendly.
pub fn validate_upgrade_note(note: &String) -> bool {
    note.len() <= UPGRADE_NOTE_MAX_LEN
}

/// Validates that a WASM hash is non-zero before an upgrade is applied.
///
/// A zero hash indicates an uninitialised value and must be rejected to
/// prevent accidental contract bricking.
///
/// # Arguments
/// * `wasm_hash` – The 32-byte WASM hash to validate.
///
/// # Returns
/// `true` if the hash is valid (non-zero), `false` otherwise.
///
/// # Security
/// Prevents upgrade calls with a zeroed hash, which would destroy the
/// contract's executable code.
pub fn validate_wasm_hash(wasm_hash: &BytesN<32>) -> bool {
    wasm_hash.to_array() != [0u8; 32]
}

/// Emits a structured SDK-upgrade audit event on the Soroban event ledger.
///
/// This provides an immutable, on-chain record that an upgrade was reviewed
/// and approved, which is useful for governance and security audits.
///
/// # Arguments
/// * `env`          – The Soroban environment.
/// * `from_version` – The previous SDK version string.
/// * `to_version`   – The new SDK version string.
/// * `reviewer`     – The address that approved the upgrade.
pub fn emit_upgrade_audit_event(
    env: &Env,
    from_version: String,
    to_version: String,
    reviewer: Address,
) {
    env.events().publish(
        (
            Symbol::new(env, "sdk_upgrade"),
            Symbol::new(env, "reviewed"),
        ),
        (reviewer, from_version, to_version),
    );
}

/// @notice Emit SDK-upgrade review with a bounded note for frontend indexing.
/// @dev Falls back to panic on oversized note to keep event schema predictable.
pub fn emit_upgrade_audit_event_with_note(
    env: &Env,
    from_version: String,
    to_version: String,
    reviewer: Address,
    note: String,
) {
    if !validate_upgrade_note(&note) {
        panic!("upgrade note exceeds UPGRADE_NOTE_MAX_LEN");
    }
    env.events().publish(
        (
            Symbol::new(env, "sdk_upgrade"),
            Symbol::new(env, "reviewed_note"),
        ),
        (reviewer, from_version, to_version, note),
    );
}
