//! `⌘P`, the quick-open list: what it offers and how it answers.
//!
//! The most used gesture of Zed, and the one maxx had nothing for. What is
//! tested here is the half that has no window: which files the list holds, and
//! which lines a query keeps.

use std::path::PathBuf;

use maxx::palette::matching_labels;
use maxx::project;
use maxx::scaffold::{self, Template};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::var("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join(name);
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn labels(files: &[PathBuf]) -> Vec<String> {
    files.iter().map(|path| path.to_string_lossy().into_owned()).collect()
}

#[test]
fn the_list_holds_the_project_and_not_what_the_tree_hides() {
    let root = scratch("maxx_quick_open");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    // What a built project holds, and what the panel hides: the shared build
    // directory, and the machine-local cargo configuration.
    std::fs::create_dir_all(root.join("target/debug")).unwrap();
    std::fs::write(root.join("target/debug/trial"), "binary").unwrap();

    let files = project::walk_files(&root);
    let labels = labels(&files);

    assert!(labels.iter().any(|path| path == "src/ui/home.rs"), "{labels:?}");
    assert!(labels.iter().any(|path| path == "Cargo.toml"), "{labels:?}");
    assert!(!labels.iter().any(|path| path.starts_with("target")), "{labels:?}");
    assert!(!labels.iter().any(|path| path.starts_with(".cargo")), "{labels:?}");
    assert!(!labels.iter().any(|path| path.starts_with(".gitignore")), "{labels:?}");
}

#[test]
fn the_whole_path_answers_the_query() {
    let root = scratch("maxx_quick_open_query");
    scaffold::create_project(&root, "trial", Template::Sidebar).unwrap();
    let files = project::walk_files(&root);
    let labels = labels(&files);
    let of = |query: &str| -> Vec<String> {
        matching_labels(labels.iter().map(String::as_str), query)
            .into_iter()
            .map(|index| labels[index].clone())
            .collect()
    };

    // Words in any order, and the directory counts: it is the question one
    // actually has.
    assert_eq!(of("ui home"), vec!["src/ui/home.rs".to_string()]);
    assert_eq!(of("home ui"), vec!["src/ui/home.rs".to_string()]);
    assert!(of("shell").contains(&"src/ui/shell.rs".to_string()));
    assert!(of("").len() == labels.len(), "an empty query keeps everything");
    assert!(of("zzz").is_empty());
}

#[test]
fn a_project_of_its_own_size_is_walked_whole() {
    let root = scratch("maxx_quick_open_many");
    std::fs::create_dir_all(root.join("src")).unwrap();
    for index in 0..50 {
        std::fs::write(root.join(format!("src/file_{index}.rs")), "").unwrap();
    }

    let files = project::walk_files(&root);
    assert_eq!(files.len(), 50);
    // And the list is bounded, so a vendored dependency tree cannot freeze the
    // window on the one gesture that has to feel instant.
    assert!(files.len() <= project::MAX_QUICK_OPEN_FILES);
}
