//! `maxx.toml`: what a project took from maxx.
//!
//! Versioned with the project, and readable: it says which modules maxx copied
//! into it, in which version, and the fingerprint they had on the way out.
//!
//! That is what makes a copy catchable-up. Copied code belongs to the project
//! and owes maxx nothing — that is the promise — but until now a defect fixed
//! in maxx stayed stuck on maxx's side. With this file, maxx knows which
//! projects are behind, and the fingerprint tells it whether the developer has
//! touched the file since: what they changed is never replaced.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The trace a copied module leaves behind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    /// The version of the template this file came out of.
    pub version: u32,
    /// The fingerprint of the file as maxx wrote it.
    ///
    /// It answers one question only: has the developer changed it since? So it
    /// has no need to be cryptographic.
    ///
    /// The alias is what a `maxx.toml` written before the field was renamed
    /// carries. Without it the whole `Module` fails to deserialise, `load`
    /// answers an empty file, and the next `record` writes that emptiness back
    /// — losing what the other modules had recorded.
    #[serde(alias = "empreinte")]
    pub fingerprint: String,
}

/// The contents of `maxx.toml`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectFile {
    /// The copied modules, by name.
    pub modules: BTreeMap<String, Module>,
}

/// Where the file lives.
pub fn path(root: &Path) -> PathBuf {
    root.join("maxx.toml")
}

/// Reads `maxx.toml`, or answers an empty file.
///
/// An unreadable file is reported and then ignored: maxx has to open the
/// project anyway, and will only rewrite the file if asked to add something to
/// it.
pub fn load(root: &Path) -> ProjectFile {
    let Ok(source) = std::fs::read_to_string(path(root)) else {
        return ProjectFile::default();
    };
    match toml::from_str(&source) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("maxx.toml is unreadable: {error}");
            ProjectFile::default()
        }
    }
}

/// Writes `maxx.toml`.
pub fn save(root: &Path, file: &ProjectFile) -> std::io::Result<()> {
    let body = toml::to_string_pretty(file)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    std::fs::write(path(root), format!("{}{body}", header()))
}

/// Records that a module was copied, with its version and its fingerprint.
pub fn record(root: &Path, module: &str, version: u32, body: &str) -> std::io::Result<()> {
    let mut file = load(root);
    file.modules.insert(module.to_string(), Module { version, fingerprint: fingerprint(body) });
    save(root, &file)
}

/// The fingerprint of a text.
///
/// FNV-1a on 64 bits, written by hand: the question asked is “has this file
/// changed since maxx wrote it”, not “has someone forged a collision”. A
/// cryptographic hashing dependency would cost more than it brings, and the
/// file's format would be heavier to read.
pub fn fingerprint(body: &str) -> String {
    // Line endings do not count: a file passed through a tool that converts
    // them has not been modified for all that.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body.bytes().filter(|byte| *byte != b'\r') {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The header maxx writes at the top of the file.
fn header() -> String {
    "# Written by maxx, and to be versioned with the project.\n\
     #\n\
     # It says what this project took from maxx: which modules, in which\n\
     # version, and the fingerprint they had on the way out. That is what lets\n\
     # maxx offer a fix later — and never replace a file you have changed\n\
     # since.\n\n"
        .to_string()
}
