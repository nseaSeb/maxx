//! The modules maxx copies into a project, and how a copy is kept up to date.

use std::io;
use std::path::Path;

use super::assets::{BUILD_MARKER, WITH_ASSETS, assets_build_rs, assets_rs};
use super::settings::settings_rs;
use super::system::system_rs;
use super::theme::theme_rs;
use super::window::window_rs;
/// The modules maxx knows how to copy into a project, and their versions.
///
/// A version is bumped whenever the template changes. `tests/scaffold.rs`
/// holds the fingerprint of each one and fails when a template moves without
/// its version following — the guard against a fix that never reaches the
/// projects carrying the old copy.
pub const MODULES: &[(&str, u32)] = &[
    ("system", 1),
    ("settings", 1),
    ("theme", 1),
    ("assets", 1),
    ("window", 1),
    ("components", 1),
];

/// The name each module carried before it was renamed to English.
///
/// A project written by an older maxx has `src/systeme.rs` and `mod systeme;`,
/// which the new names do not match: adding the module again would write a
/// second, near-identical file and declare it alongside the first, leaving the
/// developer to guess which one their code calls.
const LEGACY: &[(&str, &str)] = &[("system", "systeme"), ("settings", "reglages")];

/// The error to answer when `module` is already in the project under its old
/// name.
pub(super) fn legacy_copy(root: &Path, module: &str) -> Option<io::Error> {
    let (_, old) = LEGACY.iter().find(|(current, _)| *current == module)?;
    root.join(format!("src/{old}.rs")).exists().then(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "src/{old}.rs is this module under its former name — rename it to \
                 src/{module}.rs, and its `mod {old};` with it, before adding it again"
            ),
        )
    })
}

/// The version of `module`, if maxx knows it.
pub fn module_version(module: &str) -> Option<u32> {
    MODULES.iter().find(|(name, _)| *name == module).map(|(_, version)| *version)
}

/// The current text of a module's template.
pub fn module_body(module: &str) -> Option<String> {
    match module {
        "system" => Some(system_rs()),
        "settings" => Some(settings_rs()),
        "theme" => Some(theme_rs()),
        "assets" => Some(assets_rs()),
        "window" => Some(window_rs()),
        // A directory rather than one file: the body is every component in
        // turn, which is what its fingerprint is taken over.
        "components" => Some(super::components::module_body()),
        _ => None,
    }
}

/// The modules a project carries in a version older than maxx's, and that it
/// has not modified since.
///
/// A file the developer has touched is never listed: it is theirs now, and
/// maxx has no business replacing it.
pub fn outdated_modules(root: &Path) -> Vec<String> {
    let file = crate::projectfile::load(root);
    let mut outdated = Vec::new();

    for (module, current) in MODULES {
        let Some(recorded) = file.modules.get(*module) else {
            continue;
        };
        if recorded.version >= *current {
            continue;
        }
        let Some(body) = installed_body(root, module) else {
            continue;
        };
        if recorded.holds(&body) {
            outdated.push((*module).to_string());
        }
    }
    outdated
}

/// Whether a module is a directory rather than a single file.
///
/// `components` is a library, so it is `src/components/` with one file per
/// brick. Everything else is one `src/<name>.rs`. Read here rather than tested
/// at each site: three functions asked the question and all three assumed the
/// answer, which is how the library ended up invisible to the update it exists
/// to receive.
pub fn is_directory(module: &str) -> bool {
    module == "components"
}

/// What the project currently holds for this module, in the shape its
/// fingerprint was taken over.
///
/// `None` when it is not there at all. For a directory, the same concatenation
/// [`super::components::module_body`] builds — a library is updated whole or
/// not at all, since two versions of the same idea in one project is worse than
/// an old one.
pub(super) fn installed_body(root: &Path, module: &str) -> Option<String> {
    if is_directory(module) {
        return super::components::installed_body(root);
    }
    std::fs::read_to_string(root.join(format!("src/{module}.rs"))).ok()
}

/// Replaces a module with maxx's current version.
///
/// Refuses when the file no longer matches what maxx wrote: the developer's
/// edits are not maxx's to discard.
pub fn update_module(root: &Path, module: &str) -> io::Result<()> {
    let (Some(version), Some(body)) = (module_version(module), module_body(module)) else {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("unknown module: {module}")));
    };
    let file = crate::projectfile::load(root);
    let Some(recorded) = file.modules.get(module) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("maxx.toml does not mention {module}"),
        ));
    };

    let Some(current) = installed_body(root, module) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{module} is not in this project"),
        ));
    };
    if !recorded.holds(&current) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{module} has been modified — maxx does not replace it"),
        ));
    }

    if is_directory(module) {
        super::components::rewrite(root)?;
    } else {
        std::fs::write(root.join(format!("src/{module}.rs")), &body)?;
    }
    // The build script is the other half of the assets module, and `maxx.toml`
    // does not track it: a new `assets.rs` on top of a stale `build.rs` is a
    // project that stops compiling. Rewritten only while it is still maxx's
    // own — a script the developer has taken over is theirs.
    if module == "assets" {
        let build_path = root.join("build.rs");
        if std::fs::read_to_string(&build_path).is_ok_and(|body| body.contains(BUILD_MARKER)) {
            std::fs::write(&build_path, assets_build_rs())?;
        }
    }
    crate::projectfile::record(root, module, version, &body)
}

/// Drops `mod <module>;` from `src/main.rs`.
///
/// Called when a module file goes to the Trash: leaving the declaration behind
/// stops the project from compiling, which is the opposite of what deleting a
/// file is meant to achieve.
pub fn remove_module(root: &Path, module: &str) -> io::Result<()> {
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let declaration = format!("mod {module};");
    let wiring = WIRING.iter().find(|(name, _, _)| *name == module);
    let (statements, fragments) = wiring.map_or((&[][..], &[][..]), |(_, s, f)| (*s, *f));

    let mut changed = false;
    let mut kept: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == declaration
            || trimmed == format!("pub {declaration}")
            || statements.iter().any(|call| trimmed.contains(call))
        {
            changed = true;
            continue;
        }
        let mut line = line.to_string();
        let mut cut = false;
        for fragment in fragments {
            if line.contains(fragment) {
                line = line.replace(fragment, "");
                changed = true;
                cut = true;
            }
        }
        // A fragment rustfmt had given a line of its own leaves an empty one
        // behind, in the middle of a call chain.
        if cut && line.trim().is_empty() {
            continue;
        }
        kept.push(line);
    }

    // Deleting a file that was never declared must not rewrite `main.rs` at
    // all: `lines()` and `join` would quietly turn CRLF into LF, which is a
    // whole-file diff for a change that did not happen.
    if !changed {
        return Ok(());
    }

    std::fs::write(&main_path, joined(&kept, &source))
}

/// What maxx wrote into `main.rs` to reach a module: the whole statements that
/// go when the module goes, and the fragments that are only cut out of the line
/// carrying them.
///
/// Whole lines wherever the wiring allows it, and that is why it has the shape
/// it has: a call written as an argument to another one would leave a hole where
/// a value is expected. Deleting a module has to leave a file that still
/// compiles — `system`, `settings` and `theme` are only declared, these two are
/// called.
const WIRING: &[(&str, &[&str], &[&str])] =
    &[("assets", &[], &[WITH_ASSETS]), ("window", &["window::bounds(", "window::remember("], &[])];

/// `lines` joined the way `source` ends its own.
///
/// `str::lines` drops the `\r` of a CRLF file, so joining with `\n` would turn
/// every line of a `main.rs` into a change nobody asked for — a whole-file diff
/// for two inserted lines, and now on an ordinary view save, since the assets
/// module adds itself.
pub(super) fn joined(lines: &[String], source: &str) -> String {
    let ending = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = lines.join(ending);
    out.push_str(ending);
    out
}

/// The first line an item may be inserted before.
///
/// An inner doc comment or an inner attribute has to stay ahead of every item,
/// or the crate stops compiling.
pub(super) fn header_end(lines: &[String]) -> usize {
    let mut index = 0;
    let mut in_block = false;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if in_block {
            if line.contains("*/") {
                in_block = false;
            }
            index += 1;
            continue;
        }
        // `/*! … */` is an inner doc comment too, and just as fatal to jump
        // over: an item may not precede one.
        if line.starts_with("/*") {
            in_block = !line.contains("*/");
            index += 1;
            continue;
        }
        if line.is_empty() || line.starts_with("//") || line.starts_with("#![") {
            index += 1;
            continue;
        }
        break;
    }
    index
}
