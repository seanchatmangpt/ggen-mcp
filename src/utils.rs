use chrono::{DateTime, SecondsFormat, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::fs::Metadata;
use std::path::Path;
use std::time::SystemTime;

pub fn system_time_to_datetime(time: SystemTime) -> Option<DateTime<Utc>> {
    Some(DateTime::<Utc>::from(time))
}

pub fn system_time_to_rfc3339(time: SystemTime) -> Option<DateTime<Utc>> {
    system_time_to_datetime(time)
}

fn hash_path_metadata_digest(path: &Path, metadata: &Metadata) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(metadata.len().to_le_bytes());
    if let Ok(modified) = metadata.modified()
        && let Some(dt) = system_time_to_datetime(modified)
    {
        hasher.update(dt.to_rfc3339_opts(SecondsFormat::Micros, true).as_bytes());
    }
    hasher.finalize().into()
}

pub fn hash_path_metadata_full(path: &Path, metadata: &Metadata) -> String {
    let digest = hash_path_metadata_digest(path, metadata);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

const WORKBOOK_ID_TOKEN_LEN: usize = 10;

fn encode_base32_u64_prefix(value: u64, len: usize) -> String {
    let mut out = String::with_capacity(len);
    for i in 0..len {
        let shift = 64 - (i + 1) * 5;
        let idx = ((value >> shift) & 31) as usize;
        out.push(SHORT_ID_ALPHABET[idx] as char);
    }
    out
}

pub fn hash_path_metadata(path: &Path, metadata: &Metadata) -> String {
    let digest = hash_path_metadata_digest(path, metadata);
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    let value = u64::from_be_bytes(bytes);

    format!(
        "wb-{}",
        encode_base32_u64_prefix(value, WORKBOOK_ID_TOKEN_LEN)
    )
}

pub fn column_number_to_name(column: u32) -> String {
    let mut column = column;
    let mut name = String::new();
    while column > 0 {
        let rem = ((column - 1) % 26) as u8;
        name.insert(0, (b'A' + rem) as char);
        column = (column - 1) / 26;
    }
    name
}

pub fn cell_address(column: u32, row: u32) -> String {
    format!("{}{}", column_number_to_name(column), row)
}

pub fn make_short_workbook_id(_slug: &str, canonical_id: &str) -> String {
    canonical_id
        .strip_prefix("wb-")
        .unwrap_or(canonical_id)
        .to_string()
}

pub fn path_to_forward_slashes(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if raw.contains('\\') {
        raw.replace('\\', "/")
    } else {
        raw.into_owned()
    }
}

const SHORT_ID_ALPHABET: &[u8] = b"23456789abcdefghijkmnpqrstuvwxyz";

pub fn make_short_random_id(prefix: &str, len: usize) -> String {
    let mut rng = rand::thread_rng();

    let mut out = String::with_capacity(prefix.len() + if prefix.is_empty() { 0 } else { 1 } + len);
    if !prefix.is_empty() {
        out.push_str(prefix);
        out.push('-');
    }

    for _ in 0..len {
        let idx = rng.gen_range(0..SHORT_ID_ALPHABET.len());
        out.push(SHORT_ID_ALPHABET[idx] as char);
    }

    out
}

// =============================================================================
// Safe Unwrapping Utilities (Poka-Yoke Pattern)
// =============================================================================

use anyhow;

/// Safely get the first element from a slice with a descriptive error message
pub fn safe_first<T>(slice: &[T], context: &str) -> anyhow::Result<&T> {
    slice
        .first()
        .ok_or_else(|| anyhow::anyhow!("Failed to get first element: {}", context))
}

/// Safely get the last element from a slice with a descriptive error message
pub fn safe_last<T>(slice: &[T], context: &str) -> anyhow::Result<&T> {
    slice
        .last()
        .ok_or_else(|| anyhow::anyhow!("Failed to get last element: {}", context))
}

/// Safely get an element at an index with a descriptive error message
pub fn safe_get<T>(slice: &[T], index: usize, context: &str) -> anyhow::Result<&T> {
    slice.get(index).ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to get element at index {}: {} (slice length: {})",
            index,
            context,
            slice.len()
        )
    })
}

/// Unwrap an Option with a meaningful error message
pub fn expect_some<T>(option: Option<T>, message: &str) -> anyhow::Result<T> {
    option.ok_or_else(|| anyhow::anyhow!("Expected Some value: {}", message))
}

/// Check if a collection is empty before processing
pub fn ensure_not_empty<T>(slice: &[T], context: &str) -> anyhow::Result<()> {
    if slice.is_empty() {
        anyhow::bail!("Collection is empty: {}", context)
    }
    Ok(())
}

/// Safely extract a string from a JSON value
pub fn safe_json_str<'a>(
    value: &'a serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<&'a str> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to extract string value for key '{}': {}",
                key,
                context
            )
        })
}

/// Safely extract an array from a JSON value
pub fn safe_json_array<'a>(
    value: &'a serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<&'a Vec<serde_json::Value>> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to extract array value for key '{}': {}",
                key,
                context
            )
        })
}

/// Safely extract an object from a JSON value
pub fn safe_json_object<'a>(
    value: &'a serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<&'a serde_json::Map<String, serde_json::Value>> {
    value
        .get(key)
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to extract object value for key '{}': {}",
                key,
                context
            )
        })
}

/// Safely strip a prefix from a string
pub fn safe_strip_prefix<'a>(s: &'a str, prefix: &str, context: &str) -> anyhow::Result<&'a str> {
    s.strip_prefix(prefix).ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to strip prefix '{}' from string '{}': {}",
            prefix,
            s,
            context
        )
    })
}

/// Safely parse a string to a number
pub fn safe_parse<T: std::str::FromStr>(s: &str, context: &str) -> anyhow::Result<T>
where
    T::Err: std::fmt::Display,
{
    s.parse::<T>()
        .map_err(|e| anyhow::anyhow!("Failed to parse '{}': {} - {}", s, context, e))
}

/// Check if a string is empty and provide context
pub fn ensure_non_empty_str(s: &str, context: &str) -> anyhow::Result<&str> {
    if s.trim().is_empty() {
        anyhow::bail!("String is empty: {}", context)
    }
    Ok(s)
}

/// Safely unwrap with a fallback value and log the issue
pub fn unwrap_or_default_with_warning<T: Default + std::fmt::Debug>(
    option: Option<T>,
    context: &str,
) -> T {
    match option {
        Some(value) => value,
        None => {
            eprintln!("Warning: Using default value for {}", context);
            T::default()
        }
    }
}
