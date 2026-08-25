//! `maxx.toml` : ce qu'un projet a pris à maxx.
//!
//! Versionné avec le projet, et lisible : il dit quels modules maxx y a copiés,
//! dans quelle version, et l'empreinte qu'ils avaient en sortant.
//!
//! C'est ce qui rend une copie rattrapable. Le code copié appartient au projet
//! et ne doit rien à maxx — c'est la promesse — mais un défaut corrigé dans
//! maxx restait jusqu'ici bloqué de son côté. Avec ce fichier, maxx sait quels
//! projets sont en retard, et l'empreinte lui dit si le développeur a touché
//! le fichier depuis : ce qu'il a modifié n'est jamais remplacé.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Ce qu'un module copié laisse comme trace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    /// La version du gabarit dont ce fichier est sorti.
    pub version: u32,
    /// L'empreinte du fichier tel que maxx l'a écrit.
    ///
    /// Sert à répondre à une seule question : le développeur l'a-t-il modifié
    /// depuis ? Elle n'a donc pas à être cryptographique.
    pub empreinte: String,
}

/// Le contenu de `maxx.toml`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectFile {
    /// Les modules copiés, par nom.
    pub modules: BTreeMap<String, Module>,
}

/// Où le fichier vit.
pub fn path(root: &Path) -> PathBuf {
    root.join("maxx.toml")
}

/// Lit `maxx.toml`, ou répond un fichier vide.
///
/// Un fichier illisible est signalé puis ignoré : maxx doit ouvrir le projet
/// quand même, et ne réécrira le fichier que si on lui demande d'ajouter
/// quelque chose.
pub fn load(root: &Path) -> ProjectFile {
    let Ok(source) = std::fs::read_to_string(path(root)) else {
        return ProjectFile::default();
    };
    match toml::from_str(&source) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("maxx.toml illisible : {error}");
            ProjectFile::default()
        }
    }
}

/// Écrit `maxx.toml`.
pub fn save(root: &Path, file: &ProjectFile) -> std::io::Result<()> {
    let body = toml::to_string_pretty(file)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    std::fs::write(path(root), format!("{}{body}", header()))
}

/// Note qu'un module a été copié, avec sa version et son empreinte.
pub fn record(root: &Path, module: &str, version: u32, body: &str) -> std::io::Result<()> {
    let mut file = load(root);
    file.modules.insert(module.to_string(), Module { version, empreinte: fingerprint(body) });
    save(root, &file)
}

/// L'empreinte d'un texte.
///
/// FNV-1a sur 64 bits, écrit à la main : la question posée est « ce fichier
/// a-t-il changé depuis que maxx l'a écrit », pas « quelqu'un a-t-il forgé une
/// collision ». Une dépendance de hachage cryptographique coûterait plus qu'elle
/// n'apporte, et le format du fichier serait plus lourd à lire.
pub fn fingerprint(body: &str) -> String {
    // Les fins de ligne ne comptent pas : un fichier passé par un outil qui les
    // convertit n'a pas été modifié pour autant.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body.bytes().filter(|byte| *byte != b'\r') {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// L'en-tête que maxx écrit en tête du fichier.
fn header() -> String {
    "# Écrit par maxx, et à versionner avec le projet.\n\
     #\n\
     # Il dit ce que ce projet a pris à maxx : quels modules, dans quelle\n\
     # version, et l'empreinte qu'ils avaient en sortant. C'est ce qui permet à\n\
     # maxx de proposer une correction plus tard — et de ne jamais remplacer un\n\
     # fichier que vous avez modifié depuis.\n\n"
        .to_string()
}
