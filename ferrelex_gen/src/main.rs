// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use std::collections::BTreeMap;
use std::fs;

const URL_PROPS: &str = "https://www.unicode.org/Public/latest/ucd/DerivedCoreProperties.txt";
const URL_CATS: &str =
    "https://www.unicode.org/Public/latest/ucd/extracted/DerivedGeneralCategory.txt";
const URL_ALIASES: &str = "https://www.unicode.org/Public/latest/ucd/PropertyValueAliases.txt";

const OUT_PROPS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../ferrelex_core/src/unicode_props.rs"
);
const OUT_CATS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../ferrelex_core/src/unicode_categories.rs"
);

const VERSION_MARKER: &str = "// Unicode-Version: ";
const GENERATOR_MARKER: &str = "// Generator-Version: ";
const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// All Rust keywords (strict + reserved) that cannot be bare function names.
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

fn main() {
    run_generator(
        URL_PROPS,
        OUT_PROPS,
        "DerivedCoreProperties.txt",
        "derived core property",
        parse_property_docs,
    );
    run_categories_generator();
}

// ---------------------------------------------------------------------------
// Fetch helper
// ---------------------------------------------------------------------------

fn fetch(url: &str) -> String {
    ureq::get(url)
        .call()
        .unwrap_or_else(|e| panic!("failed to fetch {url}: {e}"))
        .into_string()
        .expect("failed to read response body")
}

// ---------------------------------------------------------------------------
// Generic generator (DerivedCoreProperties)
// ---------------------------------------------------------------------------

fn run_generator(
    url: &str,
    out_path: &str,
    source_file: &str,
    property_kind: &str,
    parse_docs: fn(&str) -> BTreeMap<String, Vec<String>>,
) {
    println!("--- {url}");
    let content = fetch(url);

    let version =
        extract_file_version(&content).unwrap_or_else(|| panic!("could not find version in {url}"));

    if !needs_update(out_path, &version) {
        return;
    }

    let props = parse_properties(&content);
    let docs = parse_docs(&content);
    let mut generated = generate_rust(&props, &docs, &version, source_file, property_kind);
    generated.push_str(&generate_from_name(props.keys().map(String::as_str)));
    fs::write(out_path, generated).expect("failed to write output file");
    println!("Written to {out_path}");
}

// ---------------------------------------------------------------------------
// Categories generator (DerivedGeneralCategory + PropertyValueAliases)
// ---------------------------------------------------------------------------

fn run_categories_generator() {
    println!("--- {URL_CATS}");
    let cats_content = fetch(URL_CATS);

    let version = extract_file_version(&cats_content)
        .expect("could not find version in DerivedGeneralCategory.txt");

    if !needs_update(OUT_CATS, &version) {
        return;
    }

    let props = parse_properties(&cats_content);
    let cat_docs = parse_category_docs(&cats_content);

    println!("--- {URL_ALIASES}");
    let aliases_content = fetch(URL_ALIASES);
    let groups = parse_category_groups(&aliases_content);
    let alias_docs = parse_alias_docs(&aliases_content);

    let mut generated = generate_rust(
        &props,
        &cat_docs,
        &version,
        "DerivedGeneralCategory.txt",
        "general category",
    );
    generated.push_str(&generate_group_fns(&groups, &alias_docs));
    generated.push_str(&generate_from_name(
        props
            .keys()
            .map(String::as_str)
            .chain(groups.keys().map(String::as_str)),
    ));

    fs::write(OUT_CATS, generated).expect("failed to write unicode_categories.rs");
    println!("Written to {OUT_CATS}");
}

// ---------------------------------------------------------------------------
// Version helpers
// ---------------------------------------------------------------------------

/// Extract the Unicode version from the first line of the raw file.
/// Expected format: `# DerivedCoreProperties-16.0.0.txt`
fn extract_file_version(content: &str) -> Option<String> {
    let first = content.lines().next()?;
    let after_dash = first.rsplit_once('-')?.1;
    let version = after_dash.strip_suffix(".txt")?;
    Some(version.to_owned())
}

/// Return `true` if the output file is absent, has a different Unicode version,
/// or was produced by a different generator version.
/// Prints a status message either way.
fn needs_update(out_path: &str, version: &str) -> bool {
    if let Ok(existing) = fs::read_to_string(out_path) {
        let existing_unicode = existing
            .lines()
            .find(|l| l.starts_with(VERSION_MARKER))
            .map(|l| l[VERSION_MARKER.len()..].trim().to_owned());
        let existing_gen = existing
            .lines()
            .find(|l| l.starts_with(GENERATOR_MARKER))
            .map(|l| l[GENERATOR_MARKER.len()..].trim().to_owned());

        match (existing_unicode, existing_gen) {
            (Some(ref uv), Some(ref gv)) if uv == version && gv == GENERATOR_VERSION => {
                println!("Already up to date (Unicode {version}, generator {GENERATOR_VERSION}).");
                return false;
            }
            (Some(ref uv), Some(ref gv)) => {
                println!("Updating Unicode {uv}→{version}, generator {gv}→{GENERATOR_VERSION}.");
            }
            _ => {
                println!("Generating Unicode {version} (generator {GENERATOR_VERSION}).");
            }
        }
    } else {
        println!("Generating Unicode {version} (generator {GENERATOR_VERSION}).");
    }
    true
}

// ---------------------------------------------------------------------------
// Doc parsers (one per file format)
// ---------------------------------------------------------------------------

/// Extract doc lines for each property from `DerivedCoreProperties.txt` headers.
///
/// Sections look like:
/// ```text
/// # Derived Property: Math
/// #  Generated from: Sm + Other_Math
/// ```
fn parse_property_docs(content: &str) -> BTreeMap<String, Vec<String>> {
    let mut docs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_prop: Option<String> = None;
    let mut current_doc: Vec<String> = Vec::new();
    let mut in_doc = false;

    for line in content.lines() {
        if let Some(prop) = line.strip_prefix("# Derived Property: ") {
            if let Some(prev) = current_prop.take() {
                docs.insert(prev, std::mem::take(&mut current_doc));
            }
            current_prop = Some(prop.trim().to_owned());
            in_doc = true;
        } else if in_doc {
            if let Some(doc_text) = line.strip_prefix("#  ") {
                current_doc.push(doc_text.trim_end().to_owned());
            } else {
                in_doc = false;
                if let Some(prop) = current_prop.take() {
                    docs.insert(prop, std::mem::take(&mut current_doc));
                }
            }
        }
    }
    if let Some(prop) = current_prop {
        docs.insert(prop, current_doc);
    }
    docs
}

/// Extract docs for each category from `DerivedGeneralCategory.txt` headers.
///
/// Sections look like:
/// ```text
/// # General_Category=Uppercase_Letter
///
/// 0041..005A    ; Lu # ...
/// ```
/// Maps short code (`Lu`) → long name (`Uppercase_Letter`).
fn parse_category_docs(content: &str) -> BTreeMap<String, Vec<String>> {
    let mut docs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_long_name: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();

        if let Some(long_name) = line.strip_prefix("# General_Category=") {
            current_long_name = Some(long_name.trim().to_owned());
            continue;
        }
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        // Data line — associate the pending long name with the short code.
        if let Some(ref long_name) = current_long_name {
            let stripped = line.split('#').next().unwrap().trim();
            if let Some(prop) = stripped.splitn(2, ';').nth(1) {
                let short_code = prop.trim().to_owned();
                if !short_code.is_empty() {
                    docs.entry(short_code)
                        .or_insert_with(|| vec![format!("Long name: {long_name}")]);
                }
            }
        }
    }
    docs
}

/// Parse `gc` entries from `PropertyValueAliases.txt` to extract long names.
/// Returns a map of category code → `["Long name: LongName"]`.
fn parse_alias_docs(content: &str) -> BTreeMap<String, Vec<String>> {
    let mut docs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(';').map(str::trim).collect();
        if parts.len() < 3 || parts[0] != "gc" {
            continue;
        }
        let code = parts[1].to_owned();
        let long_name = parts[2].to_owned();
        docs.insert(code, vec![format!("Long name: {long_name}")]);
    }
    docs
}

/// Parse `gc` entries from `PropertyValueAliases.txt` to build the group map.
///
/// Group codes are:
/// - Single uppercase letter (`L`, `M`, `N`, `P`, `S`, `Z`, `C`): their members
///   are all two-letter codes that share the same first letter.
/// - `LC` (Cased_Letter): its members are `{Lu, Ll, Lt}`. The file does not
///   list membership explicitly; `LC` = all cased letter sub-categories, which
///   is a stable Unicode definition.
///
/// Returns `group_code → Vec<member_codes>`.
fn parse_category_groups(content: &str) -> BTreeMap<String, Vec<String>> {
    // Collect all gc codes in file order.
    let gc_codes: Vec<String> = content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut parts = line.split(';').map(str::trim);
            if parts.next() == Some("gc") {
                parts.next().map(str::to_owned)
            } else {
                None
            }
        })
        .collect();

    // Leaf codes: two-letter mixed-case codes (Lu, Ll, Mn, Nd, …).
    // Group codes like LC are all-uppercase and excluded here.
    let leaf_codes: Vec<&str> = gc_codes
        .iter()
        .filter(|c| c.len() == 2 && c.chars().any(|ch| ch.is_ascii_lowercase()))
        .map(String::as_str)
        .collect();

    // Single-letter group codes: L, M, N, P, S, Z, C.
    let single_groups: Vec<char> = gc_codes
        .iter()
        .filter(|c| c.len() == 1 && c.chars().all(|ch| ch.is_ascii_uppercase()))
        .filter_map(|c| c.chars().next())
        .collect();

    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Each single-letter group contains all leaf codes with the same first letter.
    for group_char in &single_groups {
        let members: Vec<String> = leaf_codes
            .iter()
            .filter(|c| c.starts_with(*group_char))
            .map(|c| c.to_string())
            .collect();
        if !members.is_empty() {
            groups.insert(group_char.to_string(), members);
        }
    }

    // LC = Lu + Ll + Lt (cased letters — stable Unicode definition).
    let lc_members: Vec<String> = ["Lu", "Ll", "Lt"]
        .iter()
        .filter(|&&m| leaf_codes.contains(&m))
        .map(|m| m.to_string())
        .collect();
    if !lc_members.is_empty() {
        groups.insert("LC".to_owned(), lc_members);
    }

    groups
}

/// Generate functions for grouped categories using `CSet::union`.
///
/// Emits one function per group, e.g.:
/// ```rust
/// pub fn l() -> CSet {
///     lu().union(&ll()).union(&lt_()).union(&lm()).union(&lo())
/// }
/// ```
fn generate_group_fns(
    groups: &BTreeMap<String, Vec<String>>,
    alias_docs: &BTreeMap<String, Vec<String>>,
) -> String {
    let mut out = String::new();

    for (group, members) in groups {
        if members.is_empty() {
            continue;
        }

        let fn_name = prop_name_to_fn_name(group);
        let member_fns: Vec<String> = members.iter().map(|m| prop_name_to_fn_name(m)).collect();

        let first = &member_fns[0];
        let union_expr = member_fns[1..].iter().fold(format!("{first}()"), |acc, m| {
            format!("{acc}.union(&{m}())")
        });

        out.push('\n');
        out.push_str(&format!("/// Unicode general category group `{group}`.\n"));
        if let Some(doc_lines) = alias_docs.get(group) {
            if !doc_lines.is_empty() {
                out.push_str("///\n");
                for line in doc_lines {
                    if line.is_empty() {
                        out.push_str("///\n");
                    } else {
                        out.push_str(&format!("/// {line}\n"));
                    }
                }
            }
        }
        out.push_str(&format!("/// Equivalent to: {}\n", members.join(" + ")));
        out.push_str(&format!("pub fn {fn_name}() -> CSet {{\n"));
        out.push_str(&format!("    {union_expr}\n"));
        out.push_str("}\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Shared parsing and generation
// ---------------------------------------------------------------------------

/// Parse code-point ranges grouped by property name.
/// Works for any file following the `XXXX[..YYYY] ; PropName # …` format.
fn parse_properties(content: &str) -> BTreeMap<String, Vec<(u32, u32)>> {
    let mut props: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.split('#').next().unwrap().trim();

        let mut parts = line.splitn(2, ';');
        let Some(range_str) = parts.next().map(str::trim) else {
            continue;
        };
        let Some(prop_name) = parts.next().map(str::trim) else {
            continue;
        };
        if prop_name.is_empty() {
            continue;
        }

        let (lo, hi) = if let Some((lo_str, hi_str)) = range_str.split_once("..") {
            let lo = u32::from_str_radix(lo_str.trim(), 16)
                .unwrap_or_else(|_| panic!("invalid hex '{lo_str}'"));
            let hi = u32::from_str_radix(hi_str.trim(), 16)
                .unwrap_or_else(|_| panic!("invalid hex '{hi_str}'"));
            (lo, hi)
        } else {
            let cp = u32::from_str_radix(range_str, 16)
                .unwrap_or_else(|_| panic!("invalid hex '{range_str}'"));
            (cp, cp)
        };

        props
            .entry(prop_name.to_owned())
            .or_default()
            .push((lo, hi));
    }

    props
}

/// Sort and merge overlapping or adjacent ranges.
/// The result is sorted and strictly non-adjacent, satisfying `CSet::try_from`.
fn merge_ranges(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by_key(|&(lo, _)| lo);
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for (lo, hi) in ranges {
        if let Some(last) = merged.last_mut() {
            if last.1.saturating_add(1) >= lo {
                last.1 = last.1.max(hi);
                continue;
            }
        }
        merged.push((lo, hi));
    }
    merged
}

/// Convert a Unicode property name into a valid Rust function name.
///
/// - Non-ASCII-alphanumeric/underscore characters are replaced with `_`.
/// - The result is lowercased.
/// - Consecutive underscores are collapsed to one.
/// - A leading digit is prefixed with `_`.
/// - Rust keywords are suffixed with `_`.
fn prop_name_to_fn_name(name: &str) -> String {
    let mut fn_name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase();

    if fn_name.starts_with(|c: char| c.is_ascii_digit()) {
        fn_name.insert(0, '_');
    }

    if RUST_KEYWORDS.contains(&fn_name.as_str()) {
        fn_name.push('_');
    }

    fn_name.replace("__", "_")
}

/// Generate a `from_name` lookup function mapping original property names to `CSet`s.
fn generate_from_name<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    out.push_str("\n/// Look up a Unicode property or category by its canonical name.\n");
    out.push_str("pub fn from_name(name: &str) -> Option<CSet> {\n");
    out.push_str("    match name {\n");
    for name in names {
        let fn_name = prop_name_to_fn_name(name);
        out.push_str(&format!("        {name:?} => Some({fn_name}()),\n"));
    }
    out.push_str("        _ => None,\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

/// Generate the full Rust source for a unicode property file.
fn generate_rust(
    props: &BTreeMap<String, Vec<(u32, u32)>>,
    docs: &BTreeMap<String, Vec<String>>,
    version: &str,
    source_file: &str,
    property_kind: &str,
) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "// Generated by ferrelex-gen from Unicode {source_file}.\n"
    ));
    out.push_str(&format!("{VERSION_MARKER}{version}\n"));
    out.push_str(&format!("{GENERATOR_MARKER}{GENERATOR_VERSION}\n"));
    out.push_str("// Do not edit manually — run `cargo run -p ferrelex-gen` to update.\n");
    out.push('\n');
    out.push_str("use crate::cset::CSet;\n");

    for (name, ranges) in props {
        let ranges = merge_ranges(ranges.clone());
        let fn_name = prop_name_to_fn_name(name);

        out.push('\n');
        out.push_str(&format!("/// Unicode {property_kind} `{name}`.\n"));
        if let Some(doc_lines) = docs.get(name) {
            if !doc_lines.is_empty() {
                out.push_str("///\n");
                for line in doc_lines {
                    if line.is_empty() {
                        out.push_str("///\n");
                    } else {
                        out.push_str(&format!("/// {line}\n"));
                    }
                }
            }
        }
        out.push_str(&format!("pub fn {fn_name}() -> CSet {{\n"));
        out.push_str("    CSet::try_from([\n");
        for (lo, hi) in &ranges {
            out.push_str(&format!("        (0x{lo:04X}i32, 0x{hi:04X}i32),\n"));
        }
        out.push_str("    ].as_slice()).expect(\"valid generated Unicode data\")\n");
        out.push_str("}\n");
    }

    out
}
