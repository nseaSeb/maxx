//! The command line, answered before there is a window.
//!
//! maxx is a window, but two of the things it does are not: writing a project,
//! and saying what it is. A script and a CI job ask for both where no display
//! exists to open a window on — so they are settled here, on `std::env::args`
//! alone, and the process is over before gpui is started at all.

use std::path::PathBuf;

use crate::scaffold::{self, Template};

/// What the arguments ask maxx to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Invocation {
    /// Open the workshop, on this project if a path was given.
    Open(Option<PathBuf>),
    /// Write a project, and nothing else.
    New { path: PathBuf, template: Template },
    /// The usage text, on stdout.
    Usage,
    /// The version, on stdout.
    Version,
    /// A refusal: this goes to stderr, and maxx exits non-zero.
    Fail(String),
}

/// Reads the arguments, and nothing else — no disk, no process, no window.
///
/// Only `new`, `--help` and `--version` are claimed. Anything else is the path
/// maxx has always taken as its first argument, unknown leading dash included:
/// refusing to open a window over a flag nobody knows would be a regression.
pub fn parse(args: impl IntoIterator<Item = String>) -> Invocation {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Invocation::Open(None);
    };
    match first.as_str() {
        "new" => parse_new(args),
        "-h" | "--help" => Invocation::Usage,
        "-V" | "--version" => Invocation::Version,
        _ => Invocation::Open(Some(PathBuf::from(first))),
    }
}

/// The arguments after `new`: one path, and an optional shape.
fn parse_new(args: impl IntoIterator<Item = String>) -> Invocation {
    let mut args = args.into_iter();
    let mut path: Option<PathBuf> = None;
    let mut template = Template::default();

    // `while let` and not `for`: `--shape` reads the argument after it.
    while let Some(argument) = args.next() {
        // Both spellings, because a shell completes one and a script writes the
        // other, and neither is worth an error message.
        let shape = if argument == "--shape" {
            match args.next() {
                Some(shape) => shape,
                None => return Invocation::Fail(format!("--shape needs a shape: {}.", shapes())),
            }
        } else if let Some(shape) = argument.strip_prefix("--shape=") {
            shape.to_string()
        } else if argument.starts_with('-') {
            return Invocation::Fail(format!("unknown option `{argument}`.\n\n{}", usage()));
        } else if path.is_none() {
            path = Some(PathBuf::from(argument));
            continue;
        } else {
            return Invocation::Fail(format!(
                "`maxx new` writes one project, and `{argument}` is a second path."
            ));
        };

        let Some(named) = Template::from_name(&shape) else {
            return Invocation::Fail(format!(
                "unknown shape `{shape}`. The shapes are: {}.",
                shapes()
            ));
        };
        template = named;
    }

    match path {
        Some(path) => Invocation::New { path, template },
        None => Invocation::Fail(format!("`maxx new` needs a path.\n\n{}", usage())),
    }
}

/// Carries out everything that needs no window, and answers the project the
/// window should open on.
///
/// Every other invocation ends the process right here: its answer is written,
/// and there is nothing left to draw.
pub fn dispatch(args: impl IntoIterator<Item = String>) -> Option<PathBuf> {
    match parse(args) {
        // A path that is not a directory is not a refusal — maxx comes up on
        // its welcome screen, as it did before it had a command line at all.
        Invocation::Open(path) => path.filter(|path| path.is_dir()),
        Invocation::New { path, template } => {
            // The same name the save panel gives a project: the last segment of
            // the path, so the two ways in cannot disagree.
            match scaffold::create_project(&path, &scaffold::project_name(&path), template) {
                Ok(()) => {
                    let shown = path.display();
                    println!(
                        "Created {shown} (shape: {}). Next: cd {shown} && cargo run",
                        template.name()
                    );
                    std::process::exit(0)
                }
                Err(error) => fail(&error.to_string()),
            }
        }
        Invocation::Usage => {
            println!("{}", usage());
            std::process::exit(0)
        }
        Invocation::Version => {
            println!("maxx {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0)
        }
        Invocation::Fail(message) => fail(&message),
    }
}

/// Says why on stderr and ends the process non-zero, so a script stops.
fn fail(message: &str) -> ! {
    eprintln!("maxx: {message}");
    std::process::exit(1)
}

/// The usage text, with the shapes read from the table rather than typed here.
pub fn usage() -> String {
    format!(
        "maxx — a visual workshop that builds GPUI views.

Usage:
  maxx [<path>]                      open the workshop, on <path> if it is a project
  maxx new <path> [--shape <shape>]  write a project there, without a window
  maxx --help                        this text
  maxx --version                     the version

Shapes: {} (default: {})",
        shapes(),
        Template::default().name()
    )
}

/// The shape names, as an error message and the usage both list them.
fn shapes() -> String {
    Template::ALL.iter().map(|template| template.name()).collect::<Vec<_>>().join(", ")
}
