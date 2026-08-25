//! Project and view templates.
//!
//! Everything written here is ordinary Rust that compiles and runs without
//! `maxx`. The only trace `maxx` leaves is a pair of marker comments around the
//! expression it owns.

use std::io;
use std::path::Path;

/// Creates a runnable GPUI project at `root`.
pub fn create_project(root: &Path, name: &str) -> io::Result<()> {
    // Never write over an existing crate: `src/ui/mod.rs` and `src/main.rs`
    // would go with it.
    if root.join("Cargo.toml").exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} contient déjà un Cargo.toml", root.display()),
        ));
    }
    std::fs::create_dir_all(root.join("src/ui"))?;
    std::fs::create_dir_all(root.join(".cargo"))?;
    std::fs::write(root.join("Cargo.toml"), cargo_toml(&crate_name(name)))?;
    std::fs::write(root.join(".cargo/config.toml"), cargo_config())?;
    // `maxx.toml` n'y est pas : il dit ce que le projet a pris à maxx, et sert
    // à lui proposer les corrections plus tard. Il se versionne.
    std::fs::write(root.join(".gitignore"), "/target\n/.cargo\n")?;
    std::fs::write(root.join("src/main.rs"), main_rs())?;
    std::fs::write(root.join("src/menus.rs"), menus_rs())?;
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod accueil;\n")?;
    std::fs::write(root.join("src/ui/accueil.rs"), view_rs("Accueil"))?;
    Ok(())
}

/// Adds a view to an existing project and registers it in `src/ui/mod.rs`.
pub fn create_view(root: &Path, module: &str) -> io::Result<()> {
    let type_name = to_type_name(module);
    let file = root.join(format!("src/ui/{module}.rs"));
    if file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} existe déjà", file.display()),
        ));
    }
    std::fs::write(&file, view_rs(&type_name))?;

    // Registered by textual insertion so the rest of `mod.rs` — comments,
    // ordering, anything the developer put there — is untouched.
    let mod_path = root.join("src/ui/mod.rs");
    let mut source = std::fs::read_to_string(&mod_path).unwrap_or_default();
    let line = format!("pub mod {module};\n");
    if !source.contains(&line) {
        if !source.is_empty() && !source.ends_with('\n') {
            source.push('\n');
        }
        source.push_str(&line);
        std::fs::write(&mod_path, source)?;
    }
    Ok(())
}

/// Turns a folder name into a name cargo accepts: lowercase, `_` for anything
/// that is not alphanumeric, and never starting with a digit.
pub fn crate_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.starts_with(|character: char| character.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// `accueil` becomes `Accueil`, `mon_ecran` becomes `MonEcran`.
pub fn to_type_name(module: &str) -> String {
    module
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
# `runtime_shaders` compiles the Metal shaders at startup instead of at build
# time. Xcode 26 ships the Metal toolchain as a separate downloadable
# component, and without this feature the build fails on a missing `metal`
# tool. Remove it only once that component is installed.
gpui = {{ version = "0.2.2", features = ["runtime_shaders"] }}
gpui-component = "0.5.1"

[profile.dev.package."*"]
opt-level = 2
"#
    )
}

/// Points the project at the cache every maxx project shares.
///
/// The path is absolute, so it is machine-local — hence the `.gitignore` entry.
/// Losing it costs a rebuild, nothing more.
fn cargo_config() -> String {
    format!(
        r#"# Écrit par maxx. Tous les projets maxx compilent dans le même
# répertoire : gpui et gpui-component représentent environ 750 crates, et un
# projet qui a son propre `target/` les recompile intégralement. Ce fichier est
# propre à cette machine, d'où son entrée dans .gitignore.
[build]
target-dir = "{}"
"#,
        // Une chaîne TOML de base traite `\` comme un échappement, et
        // `C:\Users\…` n'en contient aucun de valide : le fichier devient
        // illisible et `cargo` refuse de démarrer avant même de compiler.
        crate::run::shared_target_dir()
            .display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

/// The modules maxx knows how to copy into a project, and their versions.
///
/// A version is bumped whenever the template changes. `tests/scaffold.rs`
/// holds the fingerprint of each one and fails when a template moves without
/// its version following — the guard against a fix that never reaches the
/// projects carrying the old copy.
pub const MODULES: &[(&str, u32)] = &[("systeme", 1), ("reglages", 1)];

/// The version of `module`, if maxx knows it.
pub fn module_version(module: &str) -> Option<u32> {
    MODULES
        .iter()
        .find(|(name, _)| *name == module)
        .map(|(_, version)| *version)
}

/// The current text of a module's template.
pub fn module_body(module: &str) -> Option<String> {
    match module {
        "systeme" => Some(system_rs()),
        "reglages" => Some(settings_rs()),
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
        let path = root.join(format!("src/{module}.rs"));
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        if crate::projectfile::fingerprint(&body) == recorded.empreinte {
            outdated.push((*module).to_string());
        }
    }
    outdated
}

/// Replaces a module with maxx's current version.
///
/// Refuses when the file no longer matches what maxx wrote: the developer's
/// edits are not maxx's to discard.
pub fn update_module(root: &Path, module: &str) -> io::Result<()> {
    let (Some(version), Some(body)) = (module_version(module), module_body(module)) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("module inconnu : {module}"),
        ));
    };
    let file = crate::projectfile::load(root);
    let Some(recorded) = file.modules.get(module) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("maxx.toml ne mentionne pas {module}"),
        ));
    };

    let path = root.join(format!("src/{module}.rs"));
    let current = std::fs::read_to_string(&path)?;
    if crate::projectfile::fingerprint(&current) != recorded.empreinte {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("src/{module}.rs a été modifié — maxx ne le remplace pas"),
        ));
    }

    std::fs::write(&path, &body)?;
    crate::projectfile::record(root, module, version, &body)
}

/// Adds the system module to an existing project and declares it.
///
/// A copied module, not a dependency: a generated project owes nothing to
/// maxx, and this one owes nothing to gpui either — it is plain `std`.
pub fn add_system_module(root: &Path) -> io::Result<()> {
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod systeme;") {
        lines.insert(header_end(&lines), "mod systeme;".into());
    }

    let path = root.join("src/systeme.rs");
    let body = system_rs();
    if !path.exists() {
        std::fs::write(&path, &body)?;
        crate::projectfile::record(root, "systeme", module_version("systeme").unwrap_or(1), &body)?;
    }

    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(&main_path, out)
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
    let kept: Vec<&str> = source
        .lines()
        .filter(|line| {
            let line = line.trim();
            line != declaration && line != format!("pub {declaration}")
        })
        .collect();

    // Deleting a file that was never declared must not rewrite `main.rs` at
    // all: `lines()` and `join` would quietly turn CRLF into LF, which is a
    // whole-file diff for a change that did not happen.
    if kept.len() == source.lines().count() {
        return Ok(());
    }

    let ending = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = kept.join(ending);
    out.push_str(ending);
    std::fs::write(&main_path, out)
}

/// The first line an item may be inserted before.
///
/// An inner doc comment or an inner attribute has to stay ahead of every item,
/// or the crate stops compiling.
fn header_end(lines: &[String]) -> usize {
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

/// The system module of a generated project.
///
/// Only what actually differs from one system to the next *and* is not already
/// in gpui. The clipboard, opening a URL, revealing a file, the file pickers:
/// gpui has all of them (`cx.write_to_clipboard`, `cx.open_url`,
/// `cx.reveal_path`, `cx.open_with_system`, `cx.prompt_for_paths`), and
/// wrapping them would be noise. What is left is where a system puts an
/// application's files, and what it calls its trash.
fn system_rs() -> String {
    r#"//! Ce que chaque système fait à sa façon.
//!
//! Écrit par maxx, à vous ensuite. Rien ici ne dépend de maxx ni de gpui :
//! c'est du `std`, copiable ailleurs tel quel.
//!
//! Ce module ne couvre volontairement pas le presse-papier, l'ouverture d'une
//! URL, la révélation d'un fichier dans le gestionnaire, ni les sélecteurs de
//! fichiers : gpui les fournit déjà — `cx.write_to_clipboard`,
//! `cx.read_from_clipboard`, `cx.open_url`, `cx.reveal_path`,
//! `cx.open_with_system`, `cx.prompt_for_paths`. Les enrober n'apporterait que
//! du bruit.

// Un module qu'on ajoute avant d'en avoir besoin : sans cela, chaque fonction
// pas encore appelée produit un avertissement, et sept avertissements le jour
// où on l'ajoute apprennent à ne plus les lire. À retirer quand tout sert.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Où ranger les réglages : ce que l'utilisateur édite.
///
/// `XDG_CONFIG_HOME` quand il est réglé, `APPDATA` sur Windows,
/// `Library/Application Support` sur macOS, `.config` ailleurs.
pub fn config_dir(application: &str) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var("APPDATA").ok()?).join(application));
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join(application));
        }
    }
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(application)
    } else {
        home.join(".config").join(application)
    })
}

/// Où ranger ce que l'application retient toute seule.
pub fn data_dir(application: &str) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join(application));
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join(application));
        }
    }
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(application)
    } else {
        home.join(".local/share").join(application)
    })
}

/// Où ranger ce qui se reconstruit.
pub fn cache_dir(application: &str) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join(application));
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join(application));
        }
    }
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Caches").join(application)
    } else {
        home.join(".cache").join(application)
    })
}

/// Écrit `body` dans `path`, en passant par un fichier temporaire.
///
/// Une écriture directe interrompue en cours laisse un fichier tronqué, que la
/// lecture suivante prendra pour un fichier abîmé.
pub fn write_atomically(path: &Path, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Le nom, pas l'extension : `reglages.json` et `reglages.toml` écriraient
    // sinon tous deux dans `reglages.tmp` et s'écraseraient l'un l'autre.
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temporary = path.with_file_name(format!("{name}.tmp"));
    std::fs::write(&temporary, body)?;
    if let Err(erreur) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(erreur);
    }
    Ok(())
}

/// Déplace `path` vers la corbeille et répond où il a atterri.
///
/// Jamais un effacement : un clic malheureux doit coûter un aller-retour dans
/// le gestionnaire de fichiers, pas la journée.
///
/// Trois corbeilles différentes. `~/.Trash` sur macOS. Sur Linux la
/// spécification freedesktop, avec le `.trashinfo` sans lequel le bureau ne
/// sait pas d'où venait le fichier et ne peut pas le restaurer. Sur Windows,
/// une corbeille à l'application : la vraie ne s'atteint que par l'API du
/// shell, ce qui coûterait une dépendance et du code `unsafe`.
pub fn move_to_trash(path: &Path, application: &str) -> Result<PathBuf, String> {
    let trash = trash_dir(application)?;
    std::fs::create_dir_all(&trash).map_err(|error| error.to_string())?;

    let name = path
        .file_name()
        .ok_or_else(|| String::from("chemin sans nom de fichier"))?
        .to_string_lossy()
        .into_owned();
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_string(), format!(".{extension}")),
        _ => (name.clone(), String::new()),
    };

    // La corbeille contient peut-être déjà un fichier de ce nom.
    let mut target = trash.join(&name);
    let mut index = 1;
    while target.exists() {
        target = trash.join(format!("{stem} {index}{extension}"));
        index += 1;
    }

    // D'un volume à l'autre, `rename` échoue avec EXDEV : il faut alors copier
    // puis effacer. En Rust et non par `mv` ou `cmd /C move` : `move` refuse de
    // porter un dossier d'un disque à l'autre, ce qui est justement le cas qui
    // amène ici sous Windows.
    if std::fs::rename(path, &target).is_err() {
        copier(path, &target).map_err(|erreur| erreur.to_string())?;
        // Seulement une fois la copie entière : effacer d'abord ferait d'une
        // copie ratée une suppression.
        let efface = if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
        efface.map_err(|erreur| erreur.to_string())?;
    }

    write_trashinfo(&target, path);
    Ok(target)
}

/// Copie un fichier, ou un dossier et tout ce qu'il contient.
fn copier(source: &Path, cible: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        std::fs::copy(source, cible)?;
        return Ok(());
    }
    std::fs::create_dir_all(cible)?;
    for entree in std::fs::read_dir(source)? {
        let entree = entree?;
        copier(&entree.path(), &cible.join(entree.file_name()))?;
    }
    Ok(())
}

/// Le dossier où ce système garde les fichiers mis à la corbeille.
fn trash_dir(application: &str) -> Result<PathBuf, String> {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").map_err(|_| String::from("HOME n'est pas défini"))?;
        return Ok(PathBuf::from(home).join(".Trash"));
    }
    if cfg!(target_os = "windows") {
        return data_dir(application)
            .map(|dir| dir.join("corbeille"))
            .ok_or_else(|| String::from("LOCALAPPDATA n'est pas défini"));
    }
    let data = match std::env::var("XDG_DATA_HOME") {
        Ok(data) if !data.is_empty() => PathBuf::from(data),
        _ => {
            let home = std::env::var("HOME").map_err(|_| String::from("HOME n'est pas défini"))?;
            PathBuf::from(home).join(".local/share")
        }
    };
    Ok(data.join("Trash/files"))
}

/// Écrit la fiche dont un bureau Linux a besoin pour proposer « Restaurer ».
fn write_trashinfo(target: &Path, original: &Path) {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        return;
    }
    let Some(files) = target.parent() else { return };
    let Some(trash) = files.parent() else { return };
    let Some(name) = target.file_name() else { return };

    let info = trash.join("info");
    if std::fs::create_dir_all(&info).is_err() {
        return;
    }
    let absolute = std::fs::canonicalize(original.parent().unwrap_or(original))
        .map(|parent| match original.file_name() {
            Some(name) => parent.join(name),
            None => parent,
        })
        .unwrap_or_else(|_| original.to_path_buf());

    // La spécification demande les deux clés, et un chemin encodé : un fichier
    // nommé `100%.rs` serait sinon mal décodé et restauré ailleurs, ou nulle
    // part.
    let body = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        encoder(&absolute.to_string_lossy()),
        date_de_suppression()
    );
    let _ = std::fs::write(
        info.join(format!("{}.trashinfo", name.to_string_lossy())),
        body,
    );
}

/// Encode un chemin comme la spécification de la corbeille le demande.
fn encoder(chemin: &str) -> String {
    let mut sortie = String::with_capacity(chemin.len());
    for octet in chemin.bytes() {
        match octet {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                sortie.push(octet as char)
            }
            _ => sortie.push_str(&format!("%{octet:02X}")),
        }
    }
    sortie
}

/// L'instant de la suppression, dans la forme attendue.
///
/// En UTC là où la spécification demande l'heure locale : `std` n'a pas de
/// fuseau, et prendre une dépendance pour une ligne d'un fichier que personne
/// ne lit à la main serait cher payé.
fn date_de_suppression() -> String {
    let secondes = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|ecoule| ecoule.as_secs())
        .unwrap_or(0);

    let jours = (secondes / 86_400) as i64;
    let heure = secondes % 86_400;
    let (annee, mois, jour) = date_civile(jours);
    format!(
        "{annee:04}-{mois:02}-{jour:02}T{:02}:{:02}:{:02}",
        heure / 3600,
        (heure % 3600) / 60,
        heure % 60
    )
}

/// Convertit un nombre de jours depuis 1970-01-01 en date civile.
///
/// L'algorithme de Howard Hinnant : il décale l'année pour la faire commencer
/// en mars, ce qui met le jour bissextile à la fin et évite d'en faire un cas.
fn date_civile(jours: i64) -> (i64, u32, u32) {
    let decale = jours + 719_468;
    let ere = if decale >= 0 { decale } else { decale - 146_096 } / 146_097;
    let jour_ere = decale - ere * 146_097;
    let annee_ere = (jour_ere - jour_ere / 1460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let annee = annee_ere + ere * 400;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let mois_decale = (5 * jour_annee + 2) / 153;
    let jour = (jour_annee - (153 * mois_decale + 2) / 5 + 1) as u32;
    let mois = if mois_decale < 10 {
        mois_decale + 3
    } else {
        mois_decale - 9
    } as u32;
    (if mois <= 2 { annee + 1 } else { annee }, mois, jour)
}
"#
    .to_string()
}

/// Adds the settings module to an existing project, with what it needs.
///
/// Pulls the system module in with it: the settings need to know where this
/// system puts an application's files, and that is exactly what `systeme.rs`
/// answers. And declares `serde` and `serde_json_lenient`, both already
/// compiled in the tree through gpui, so the build does not grow.
pub fn add_settings_module(root: &Path) -> io::Result<()> {
    add_system_module(root)?;
    add_dependencies(
        root,
        &[
            ("serde", "{ version = \"1\", features = [\"derive\"] }"),
            ("serde_json_lenient", "\"0.2\""),
        ],
    )?;

    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    if !lines.iter().any(|line| line.trim() == "mod reglages;") {
        lines.insert(header_end(&lines), "mod reglages;".into());
    }

    let path = root.join("src/reglages.rs");
    let body = settings_rs();
    if !path.exists() {
        std::fs::write(&path, &body)?;
        crate::projectfile::record(root, "reglages", module_version("reglages").unwrap_or(1), &body)?;
    }

    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(&main_path, out)
}

/// Declares crates in the project's `Cargo.toml`, under `[dependencies]`.
///
/// Textual, like everything else maxx adds: the file is the developer's, and
/// rewriting it from a template would throw away whatever they put in it.
fn add_dependencies(root: &Path, crates: &[(&str, &str)]) -> io::Result<()> {
    let path = root.join("Cargo.toml");
    let source = std::fs::read_to_string(&path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    let Some(section) = lines.iter().position(|line| line.trim() == "[dependencies]") else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cargo.toml : pas de section [dependencies]",
        ));
    };
    // The end of the section, not the end of the file: a `[profile]` block
    // after it must stay after it.
    let end = lines[section + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .map(|offset| section + 1 + offset)
        .unwrap_or(lines.len());

    let mut inserted = 0;
    for (name, requirement) in crates {
        let declared = lines[section + 1..end].iter().any(|line| {
            line.split('=')
                .next()
                .is_some_and(|left| left.trim() == *name)
        });
        if declared {
            continue;
        }
        lines.insert(end + inserted, format!("{name} = {requirement}"));
        inserted += 1;
    }
    if inserted == 0 {
        return Ok(());
    }

    let ending = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = lines.join(ending);
    out.push_str(ending);
    std::fs::write(&path, out)
}

/// The settings module of a generated project.
///
/// The same discipline maxx applies to its own: JSON with comments, only the
/// changed key rewritten, a documented default file. It is a copy, and a copy
/// is a debt — a defect found on one side has to be carried to the other.
fn settings_rs() -> String {
    r##"//! Les réglages de l'application : ce qu'elle retient d'un lancement à l'autre.
//!
//! Écrit par maxx, à vous ensuite. Ajoutez vos champs à `Reglages`, une ligne
//! dans `defauts_documentes`, et c'est tout.
//!
//! Du JSON à commentaires, parce qu'un fichier qu'on invite l'utilisateur à
//! ouvrir doit accepter d'être annoté — et **seule la clé qui change est
//! réécrite**. Vos commentaires et votre mise en forme survivent à un
//! enregistrement, ce qu'une sérialisation de la structure entière ne permet
//! jamais.
//!
//! Le principe de lecture : un fichier absent, partiel ou abîmé n'est jamais
//! pire que pas de fichier. `serde(default)` fait retomber une clé manquante
//! sur son défaut plutôt que d'échouer la lecture entière, et un fichier
//! illisible est signalé puis laissé intact — l'écraser perdrait ce que
//! l'utilisateur était en train d'y écrire.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Ce que l'application retient. Ajoutez vos champs ici.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reglages {
    /// Exemple : à remplacer par les vôtres.
    pub theme_sombre: bool,
    /// Exemple : à remplacer par les vôtres.
    pub taille_du_texte: f32,
}

impl Default for Reglages {
    fn default() -> Self {
        Self {
            theme_sombre: true,
            taille_du_texte: 14.0,
        }
    }
}

/// Le nom du dossier de l'application, sous le répertoire de configuration.
const APPLICATION: &str = env!("CARGO_PKG_NAME");

/// Où vit le fichier.
pub fn chemin() -> Option<PathBuf> {
    crate::systeme::config_dir(APPLICATION).map(|dossier| dossier.join("reglages.json"))
}

/// Lit les réglages, en tolérant les commentaires et les virgules finales.
pub fn charger() -> Reglages {
    let Some(chemin) = chemin() else {
        return Reglages::default();
    };
    let Ok(source) = std::fs::read_to_string(&chemin) else {
        return Reglages::default();
    };
    match serde_json_lenient::from_str_lenient(&source) {
        Ok(reglages) => reglages,
        Err(erreur) => {
            eprintln!("{} illisible : {erreur}", chemin.display());
            Reglages::default()
        }
    }
}

/// Écrit les réglages en ne touchant que les clés dont la valeur a changé.
pub fn enregistrer(reglages: &Reglages) -> std::io::Result<()> {
    let Some(chemin) = chemin() else {
        return Ok(());
    };
    let source = match std::fs::read_to_string(&chemin) {
        Ok(source) if source.contains('{') => source,
        Ok(_) => defauts_documentes(),
        Err(erreur) if erreur.kind() == std::io::ErrorKind::NotFound => defauts_documentes(),
        // Illisible pour une autre raison — une permission, un incident : écrire
        // remplacerait ce que l'utilisateur a écrit par les défauts, ce qui est
        // le seul résultat pire que ne pas enregistrer.
        Err(erreur) => return Err(erreur),
    };
    crate::systeme::write_atomically(&chemin, &rustiner(&source, reglages))
}

/// Le fichier écrit quand il n'y en a pas : chaque clé, son défaut, et une
/// ligne qui dit ce qu'elle fait. Le fichier est sa propre documentation.
pub fn defauts_documentes() -> String {
    let defauts = Reglages::default();
    format!(
        r#"// Réglages de {APPLICATION}.
//
// Ce fichier est à vous. L'application ne réécrit que la clé qu'elle change :
// vos commentaires et votre mise en forme restent en place. Les commentaires et
// les virgules finales sont acceptés à la lecture.
{{
  // Exemple : à remplacer par les vôtres.
  "theme_sombre": {},

  // Exemple : à remplacer par les vôtres.
  "taille_du_texte": {}
}}
"#,
        defauts.theme_sombre, defauts.taille_du_texte
    )
}

/// Écrit chaque clé de `reglages` dans `source`, en changeant le moins d'octets
/// possible.
pub fn rustiner(source: &str, reglages: &Reglages) -> String {
    let Ok(serde_json_lenient::Value::Object(valeurs)) = serde_json_lenient::to_value(reglages)
    else {
        return source.to_string();
    };

    let mut sortie = source.to_string();
    for (cle, valeur) in valeurs {
        let rendue = valeur.to_string();
        sortie = match remplacer(&sortie, &cle, &rendue) {
            Some(rustinee) => rustinee,
            None => ajouter(&sortie, &cle, &rendue),
        };
    }
    sortie
}

/// Remplace la valeur d'une clé de premier niveau, en gardant tout le reste.
///
/// Répond `None` quand la clé n'y est pas, ce qui dit à l'appelant de
/// l'ajouter.
fn remplacer(source: &str, cle: &str, valeur: &str) -> Option<String> {
    let (membre, _) = parcourir(source, cle);
    let membre = membre?;
    Some(format!(
        "{}{valeur}{}",
        &source[..membre.start],
        &source[membre.end..]
    ))
}

/// Ajoute une clé juste après l'accolade ouvrante.
///
/// Après l'ouvrante et non avant la fermante : la dernière chose d'un objet est
/// très souvent un commentaire, et une virgule ajoutée là se retrouverait
/// commentée — deux membres sans séparateur, fichier invalide.
fn ajouter(source: &str, cle: &str, valeur: &str) -> String {
    let (_, position) = parcourir(source, cle);
    let Some(position) = position else {
        return format!("{{\n  \"{cle}\": {valeur}\n}}\n");
    };

    let octets = source.as_bytes();
    let suivant = sauter_le_vide(octets, position);
    let vide = suivant >= octets.len() || octets[suivant] == b'}';
    let separateur = if vide { "" } else { "," };

    format!(
        "{}\n  \"{cle}\": {valeur}{separateur}{}",
        &source[..position],
        &source[position..]
    )
}

/// Parcourt les membres de l'objet de premier niveau.
///
/// Répond l'étendue de la valeur de `cle` si elle y est, et l'endroit où une
/// nouvelle clé peut être insérée.
fn parcourir(source: &str, cle: &str) -> (Option<std::ops::Range<usize>>, Option<usize>) {
    let octets = source.as_bytes();
    // Pas `find('{')` : le fichier commence par un bloc de commentaires, et une
    // accolade écrite dedans ancrerait tout le parcours dans le commentaire.
    let accolade = sauter_le_vide(octets, 0);
    if accolade >= octets.len() || octets[accolade] != b'{' {
        return (None, None);
    }
    let apres = accolade + 1;

    let attendu = format!("\"{cle}\"");
    let mut index = sauter_le_vide(octets, apres);
    while index < octets.len() && octets[index] != b'}' {
        if octets[index] != b'"' {
            break;
        }
        let fin_du_nom = fin_de_chaine(octets, index);
        let trouve = source[index..fin_du_nom] == attendu;

        let deux_points = sauter_le_vide(octets, fin_du_nom);
        if deux_points >= octets.len() || octets[deux_points] != b':' {
            break;
        }
        let debut = sauter_le_vide(octets, deux_points + 1);
        let fin = fin_de_valeur(octets, debut);
        if trouve {
            return (Some(debut..fin), Some(apres));
        }

        index = sauter_le_vide(octets, fin);
        if index < octets.len() && octets[index] == b',' {
            index = sauter_le_vide(octets, index + 1);
        }
    }

    (None, Some(apres))
}

/// Saute les espaces et les commentaires.
///
/// Un commentaire n'est pas du JSON, et le parcours doit l'enjamber sans lire
/// comme structure une accolade, un guillemet ou un deux-points écrits dedans.
/// Un seul guillemet impair dans un commentaire laisserait sinon le balayage
/// « dans une chaîne » jusqu'à la fin du fichier.
fn sauter_le_vide(octets: &[u8], mut index: usize) -> usize {
    loop {
        while index < octets.len() && octets[index].is_ascii_whitespace() {
            index += 1;
        }
        if octets[index..].starts_with(b"//") {
            while index < octets.len() && octets[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if octets[index..].starts_with(b"/*") {
            index += 2;
            while index + 1 < octets.len() && !(octets[index] == b'*' && octets[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(octets.len());
            continue;
        }
        return index;
    }
}

/// L'index juste après la chaîne dont le guillemet ouvrant est à `index`.
fn fin_de_chaine(octets: &[u8], index: usize) -> usize {
    let mut index = index + 1;
    while index < octets.len() {
        match octets[index] {
            b'\\' => index += 2,
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    octets.len()
}

/// L'index juste après la valeur JSON qui commence à `index`.
fn fin_de_valeur(octets: &[u8], index: usize) -> usize {
    if index >= octets.len() {
        return octets.len();
    }
    match octets[index] {
        b'"' => fin_de_chaine(octets, index),
        ouvrant @ (b'[' | b'{') => {
            let fermant = if ouvrant == b'[' { b']' } else { b'}' };
            let mut profondeur = 0usize;
            let mut index = index;
            while index < octets.len() {
                index = sauter_le_vide(octets, index);
                if index >= octets.len() {
                    break;
                }
                match octets[index] {
                    b'"' => {
                        index = fin_de_chaine(octets, index);
                        continue;
                    }
                    octet if octet == ouvrant => profondeur += 1,
                    octet if octet == fermant => {
                        profondeur -= 1;
                        if profondeur == 0 {
                            return index + 1;
                        }
                    }
                    _ => {}
                }
                index += 1;
            }
            octets.len()
        }
        // Un nombre, `true`, `false` ou `null` : la valeur s'arrête au premier
        // caractère qui ne peut en faire partie — un séparateur, une espace, ou
        // le début d'un commentaire.
        _ => {
            let mut index = index;
            while index < octets.len() {
                let octet = octets[index];
                if octet.is_ascii_whitespace()
                    || matches!(octet, b',' | b'}' | b']')
                    || octets[index..].starts_with(b"//")
                    || octets[index..].starts_with(b"/*")
                {
                    break;
                }
                index += 1;
            }
            index
        }
    }
}

/// Le chemin d'un fichier, tel quel : pratique pour l'afficher dans un écran de
/// réglages, ou pour l'ouvrir dans l'éditeur de l'utilisateur.
pub fn chemin_affichable() -> String {
    chemin()
        .map(|chemin| chemin.display().to_string())
        .unwrap_or_else(|| "emplacement introuvable sur ce système".into())
}

/// Pour les tests : lit un fichier donné plutôt que celui de l'application.
pub fn charger_depuis(chemin: &Path) -> Reglages {
    let Ok(source) = std::fs::read_to_string(chemin) else {
        return Reglages::default();
    };
    serde_json_lenient::from_str_lenient(&source).unwrap_or_default()
}
"##.to_string()
}

/// The menu bar of a generated project.
///
/// A GPUI application gets no menu bar of its own — not even a Quit — unless it
/// calls `set_menus`, so the template ships a usable one and maxx edits it.
fn menus_rs() -> String {
    r#"use gpui::{
    App, Bounds, Context, Menu, MenuItem, OsAction, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};

actions!(app, [About, Quit, HideApp, HideOthers, ShowAll, Undo, Redo, Cut, Copy, Paste, SelectAll, Minimize]);

/// Wires what the menu entries do.
pub fn register(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| open_about(cx));
    cx.on_action(|_: &Minimize, cx: &mut App| {
        // Deferred: an action handler runs inside the window's own update, and
        // gpui refuses to enter a second one.
        cx.defer(|cx| {
            if let Some(handle) = cx.active_window() {
                let _ = handle.update(cx, |_, window, _| window.minimize_window());
            }
        });
    });
    // maxx:handlers
}

/// The shortcuts the menu entries display.
pub fn key_bindings() -> Vec<gpui::KeyBinding> {
    use gpui::KeyBinding;
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// What the About window shows.
///
/// Name and version are read from Cargo.toml at build time: `[package]` is the
/// one place a version number should live, and `cargo set-version` or a hand
/// edit there is enough to change what this window says.
struct AboutWindow {
    name: SharedString,
    version: SharedString,
}

impl Render for AboutWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .size_full()
            .bg(rgb(0x1e2127))
            .text_color(rgb(0xc8ccd4))
            .child(div().text_2xl().child(self.name.clone()))
            .child(
                div()
                    .text_color(rgb(0x7f8896))
                    .child(format!("version {}", self.version)),
            )
    }
}

/// Opens the About window, or brings it forward when it is already up.
///
/// Plain gpui, no `gpui_component`: a window drawing a component widget has to
/// be rooted in `gpui_component::Root`, and this one does not need it.
///
/// Deferred for the same reason as Minimize above: an action handler runs
/// inside the update of a window, and gpui refuses to enter a second one.
fn open_about(cx: &mut App) {
    cx.defer(open_about_now);
}

fn open_about_now(cx: &mut App) {
    if let Some(existing) = cx
        .windows()
        .into_iter()
        .find(|handle| handle.downcast::<AboutWindow>().is_some())
    {
        let _ = existing.update(cx, |_, window, _| window.activate_window());
        return;
    }

    let bounds = Bounds::centered(None, size(px(320.), px(180.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(SharedString::from("À propos")),
            ..Default::default()
        }),
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };

    cx.open_window(options, |_window, cx| {
        cx.new(|_| AboutWindow {
            name: SharedString::from(env!("CARGO_PKG_NAME")),
            version: SharedString::from(env!("CARGO_PKG_VERSION")),
        })
    })
    .ok();
}

/// The menu bar itself.
pub fn app_menus() -> Vec<Menu> {
    // maxx:begin
    vec![
        Menu {
            name: "app".into(),
            items: vec![
                MenuItem::action("À propos", About),
                MenuItem::separator(),
                MenuItem::action("Masquer", HideApp),
                MenuItem::action("Masquer les autres", HideOthers),
                MenuItem::action("Tout afficher", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quitter", Quit),
            ],
        },
        Menu {
            name: "Édition".into(),
            items: vec![
                MenuItem::os_action("Annuler", Undo, OsAction::Undo),
                MenuItem::os_action("Rétablir", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Couper", Cut, OsAction::Cut),
                MenuItem::os_action("Copier", Copy, OsAction::Copy),
                MenuItem::os_action("Coller", Paste, OsAction::Paste),
                MenuItem::os_action("Tout sélectionner", SelectAll, OsAction::SelectAll),
            ],
        },
        Menu {
            name: "Fenêtre".into(),
            items: vec![MenuItem::action("Réduire", Minimize)],
        },
    ]
    // maxx:end
}
"#
    .to_string()
}

fn main_rs() -> String {
    r#"mod menus;
mod ui;

use gpui::{
    App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size,
};
use gpui_component::Root;

use crate::ui::accueil::Accueil;

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.activate(true);

        menus::register(cx);
        cx.bind_keys(menus::key_bindings());
        cx.set_menus(menus::app_menus());

        let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Accueil::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("la fenêtre doit s'ouvrir");
    });
}
"#
    .to_string()
}

fn view_rs(type_name: &str) -> String {
    format!(
        r#"use gpui::{{Context, Window, prelude::*}};
use gpui_component::label::Label;
use gpui_component::v_flex;

pub struct {type_name} {{}}

impl {type_name} {{
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {{
        Self {{}}
    }}
}}

impl Render for {type_name} {{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {{
        // maxx:begin
        v_flex()
            .gap_2()
            .p_4()
            .child(Label::new("Bienvenue"))
        // maxx:end
    }}
}}
"#
    )
}

/// Gives an existing project a menu bar: writes `src/menus.rs` and wires it
/// into `src/main.rs`.
///
/// Wired by textual insertion, like `create_view`: the project may predate the
/// template entirely, and rewriting its `main.rs` from the template would throw
/// away whatever it does at startup.
pub fn add_menu_bar(root: &Path) -> io::Result<()> {
    // `main.rs` is patched first, and nothing is written until it is known to
    // work: a `src/menus.rs` left behind by a failed wiring would make the next
    // attempt believe the project already has a menu bar.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod menus;") {
        lines.insert(header_end(&lines), "mod menus;".into());
    }

    if !source.contains("menus::app_menus()") {
        // `cx.activate` is what every gpui `main` does first; failing that, the
        // line that opens the closure `run` was given.
        let anchor = lines
            .iter()
            .position(|line| line.contains(".activate("))
            .or_else(|| {
                lines
                    .iter()
                    .position(|line| line.contains(".run(") && line.trim_end().ends_with('{'))
            });
        let Some(anchor) = anchor else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "src/main.rs : ni cx.activate(…) ni Application::new().run(…) — \
                 ajoutez menus::register(cx), cx.bind_keys(menus::key_bindings()) \
                 et cx.set_menus(menus::app_menus()) à la main",
            ));
        };

        // The three calls need the name this `main` gave its application. Both
        // anchors carry it, in different places: `cx.activate(true)` names it
        // as the receiver, `run(|app| {` as the closure's argument. Assuming
        // `cx` would hand a project written as `run(|app| {` three lines naming
        // something that does not exist.
        let Some(app) = application_binding(&lines[anchor]) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "src/main.rs : impossible de lire le nom de l'application dans « {} » — \
                     ajoutez menus::register(…), …bind_keys(menus::key_bindings()) \
                     et …set_menus(menus::app_menus()) à la main",
                    lines[anchor].trim()
                ),
            ));
        };

        let indent: String = lines[anchor]
            .chars()
            .take_while(|character| character.is_whitespace())
            .collect();
        for (offset, call) in [
            format!("menus::register({app});"),
            format!("{app}.bind_keys(menus::key_bindings());"),
            format!("{app}.set_menus(menus::app_menus());"),
        ]
        .iter()
        .enumerate()
        {
            lines.insert(anchor + 1 + offset, format!("{indent}{call}"));
        }
    }

    let menus_path = root.join("src/menus.rs");
    let created = !menus_path.exists();
    if created {
        std::fs::write(&menus_path, menus_rs())?;
    }

    let mut out = lines.join("\n");
    out.push('\n');
    if let Err(error) = std::fs::write(&main_path, out) {
        // A `menus.rs` left behind by a failed wiring would make the next
        // attempt believe the project already has a menu bar, and skip the
        // wiring for good.
        if created {
            let _ = std::fs::remove_file(&menus_path);
        }
        return Err(error);
    }
    Ok(())
}

/// Whether `line` is one of the three calls `add_menu_bar` writes, or the
/// module declaration that goes with them.
fn is_menu_wiring(line: &str) -> bool {
    if line == "mod menus;" || line == "menus::register" {
        return true;
    }
    if let Some(argument) = line
        .strip_prefix("menus::register(")
        .and_then(|rest| rest.strip_suffix(");"))
    {
        return identifier(argument).is_some();
    }
    for call in [
        ".bind_keys(menus::key_bindings());",
        ".set_menus(menus::app_menus());",
    ] {
        if let Some(receiver) = line.strip_suffix(call)
            && identifier(receiver).is_some()
        {
            return true;
        }
    }
    false
}

/// The name `line` gives the application.
///
/// Either the receiver of `.activate(`, or the argument of the closure handed
/// to `run` — `|cx|` as much as `|app: &mut App|`.
fn application_binding(line: &str) -> Option<String> {
    if let Some(dot) = line.find(".activate(") {
        let receiver: String = line[..dot]
            .chars()
            .rev()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        return identifier(&receiver.chars().rev().collect::<String>());
    }

    let start = line.find('|')?;
    let rest = &line[start + 1..];
    let end = rest.find('|')?;
    identifier(rest[..end].split(':').next()?.trim())
}

/// `name` when it can be a Rust binding, nothing otherwise.
fn identifier(name: &str) -> Option<String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        && !name.starts_with(|character: char| character.is_ascii_digit());
    valid.then(|| name.to_string())
}

/// Unwires the menu bar from `src/main.rs`.
///
/// The file `src/menus.rs` is the caller's business — the project panel puts it
/// in the Trash — but leaving `mod menus;` behind would stop the project from
/// compiling.
pub fn remove_menu_bar(root: &Path) -> io::Result<()> {
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    // Matched on shape, not on the exact text: `add_menu_bar` writes these
    // with the name the project gave its application, which is `cx` in the
    // template and anything at all in a hand-written `main.rs`. Filtering
    // literal `cx` lines would leave a call to a module that no longer exists.
    let kept: Vec<&str> = source
        .lines()
        .filter(|line| !is_menu_wiring(line.trim()))
        .collect();
    let mut out = kept.join("\n");
    out.push('\n');
    std::fs::write(&main_path, out)
}
