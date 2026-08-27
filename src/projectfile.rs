//! `maxx.toml`: the project's own file.
//!
//! Versioned with the project, and readable. It carries two things.
//!
//! What the project took from maxx: which modules were copied into it, in which
//! version, and the fingerprint they had on the way out. That is what makes a
//! copy catchable-up. Copied code belongs to the project and owes maxx nothing
//! — that is the promise — but a defect fixed in maxx would otherwise stay
//! stuck on maxx's side. With this file, maxx knows which projects are behind,
//! and the fingerprint tells it whether the developer has touched the file
//! since: what they changed is never replaced.
//!
//! And what the project is: the view its window opens on, and how it is
//! launched. Both were written in stone before — the entry view inside
//! `main.rs`, the launch as a bare `cargo run` — which meant a project that
//! wanted `--release`, a feature, or another first screen had no place to say
//! so. Here is that place. Every key is optional, and a project without them
//! behaves exactly as it did.

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

/// What maxx knows about the project itself.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    /// The module under `src/ui/` whose view the window opens on, without its
    /// extension — `home` for `src/ui/home.rs`.
    ///
    /// A name, not a path: `main.rs` reaches the view through `crate::ui::`,
    /// and a view living anywhere else is not one maxx wrote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}

impl Project {
    /// Whether nothing has been recorded, and the section can stay out of the
    /// file.
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// How maxx launches the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    /// The cargo profile, handed over as `--profile <name>`.
    ///
    /// Absent means cargo's own default, which is `dev` — and `release` is
    /// spelled here as a profile rather than as a flag of its own, because
    /// `--profile` is the form that also carries a profile the project
    /// defined itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// The features to switch on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Whether the crate's default features stay on.
    #[serde(rename = "default-features", skip_serializing_if = "is_on")]
    pub default_features: bool,
    /// What the application itself is given, after `--`.
    ///
    /// Only on a run: `cargo build` has nothing to hand them to.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

impl Default for Run {
    fn default() -> Self {
        Self { profile: None, features: Vec::new(), default_features: true, args: Vec::new() }
    }
}

impl Run {
    /// Whether nothing has been recorded, and the section can stay out of the
    /// file.
    fn is_default(&self) -> bool {
        self == &Self::default()
    }

    /// The whole cargo command line, `subcommand` first.
    ///
    /// The application's own arguments are appended to a run and dropped from
    /// a build: `cargo build -- --verbose` is refused by cargo, and a prewarm
    /// that fails to start is a prewarm nobody sees fail.
    pub fn arguments(&self, subcommand: &str) -> Vec<String> {
        let mut arguments = vec![subcommand.to_string()];
        if let Some(profile) = &self.profile {
            arguments.push("--profile".into());
            arguments.push(profile.clone());
        }
        if !self.default_features {
            arguments.push("--no-default-features".into());
        }
        if !self.features.is_empty() {
            arguments.push("--features".into());
            arguments.push(self.features.join(","));
        }
        if subcommand == "run" && !self.args.is_empty() {
            arguments.push("--".into());
            arguments.extend(self.args.iter().cloned());
        }
        arguments
    }
}

/// Whether a flag is still on, for the fields that are written only once off.
fn is_on(value: &bool) -> bool {
    *value
}

/// The contents of `maxx.toml`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectFile {
    /// What the project is.
    #[serde(skip_serializing_if = "Project::is_default")]
    pub project: Project,
    /// How it is launched.
    #[serde(skip_serializing_if = "Run::is_default")]
    pub run: Run,
    /// The copied modules, by name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, Module>,
}

/// Where the file lives.
pub fn path(root: &Path) -> PathBuf {
    root.join("maxx.toml")
}

/// Reads `maxx.toml`, or answers an empty file.
///
/// An unreadable file is reported and then ignored: maxx has to open the
/// project anyway, and a project whose file has a typo in it still opens.
pub fn load(root: &Path) -> ProjectFile {
    match read(root) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("{error}");
            ProjectFile::default()
        }
    }
}

/// Reads `maxx.toml` for something about to write it back.
///
/// The difference with [`load`] is the whole point: a file that does not parse
/// is an error here, not an empty file. Rewriting from empty would erase every
/// module fingerprint the developer's project holds — and one missing bracket
/// in a hand-written `[run]` is enough to get there — turning a menu item into
/// a silent loss of the very records this file exists for.
fn read(root: &Path) -> std::io::Result<ProjectFile> {
    let source = match std::fs::read_to_string(path(root)) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProjectFile::default());
        }
        Err(error) => return Err(error),
    };
    toml::from_str(&source).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{}: {error}", path(root).display()),
        )
    })
}

/// Writes `maxx.toml`.
pub fn save(root: &Path, file: &ProjectFile) -> std::io::Result<()> {
    let body = toml::to_string_pretty(file)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    std::fs::write(path(root), format!("{}{body}", header()))
}

/// The module the project's window opens on, when the file says.
pub fn entry(root: &Path) -> Option<String> {
    load(root).project.entry
}

/// Records which module the window opens on.
///
/// The file alone: patching `main.rs` so it is true belongs to
/// [`crate::scaffold::set_entry_view`], which calls this once the code is
/// written — a `maxx.toml` claiming an entry the code does not open would be
/// worse than no record at all.
pub fn set_entry(root: &Path, module: &str) -> std::io::Result<()> {
    let mut file = read(root)?;
    file.project.entry = Some(module.to_string());
    save(root, &file)
}

/// The cargo command line this project asks for.
pub fn arguments(root: &Path, subcommand: &str) -> Vec<String> {
    load(root).run.arguments(subcommand)
}

/// Records that a module was copied, with its version and its fingerprint.
pub fn record(root: &Path, module: &str, version: u32, body: &str) -> std::io::Result<()> {
    let mut file = read(root)?;
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
     # It carries the project: the view its window opens on, how it is\n\
     # launched — profile, features, arguments — and what it took from maxx,\n\
     # which modules, in which version, and the fingerprint they had on the\n\
     # way out. That last part is what lets maxx offer a fix later — and never\n\
     # replace a file you have changed since.\n\
     #\n\
     # [project]\n\
     # entry = \"home\"          # the module under src/ui/ the window opens on\n\
     #\n\
     # [run]\n\
     # profile = \"release\"     # cargo run --profile release\n\
     # features = [\"demo\"]\n\
     # default-features = false\n\
     # args = [\"--verbose\"]    # handed to the application, after --\n\n"
        .to_string()
}
