# Changelog

What changed, for whoever opens maxx and wonders what is new. The commits say
how; this says what for.

## 0.2.0

### Your own components

maxx's catalogue was a table compiled into maxx, so it knew nothing about the
`Card` you wrote on your third day — the day a designer usually stops being
useful, since your own components are what a project is made of once it has any.

It reads them now, out of the Rust itself rather than a file describing them.
`pub fn new` says what it takes to build one; a builder method taking one string
is a text field in the inspector, one taking nothing is a switch. They appear at
the top of the palette under **Of this project**, dropping one writes the call
and the `use` line that names it, and a hover affordance opens the source.

What maxx cannot write, it does not offer — a constructor taking a `&mut Window`,
an argument that is not a string, a name the catalogue already answers to.

### A library to start from

`File ▸ Add to project ▸ The components` writes `src/components/` with a `Card`,
a `Toolbar` and an `EmptyState`, one file each, painting with the project's own
palette. Ordinary Rust, yours from the moment it lands.

It is also what made the reader above possible: maxx writes the shape it will
have to read, so the hard half of the problem — reading arbitrary Rust — is
bounded by a worked example of every shape it accepts.

### The palette, edited in maxx

Select `src/theme.rs` in the project panel and every role opens as two colour
pickers. Only the value you change is rewritten: the comments, the roles you
added and everything else in the file stay as you wrote them.

`Preferences ▸ Appearance ▸ Your palette` opens a `theme.rs` of maxx's own, in
that same editor. A project created after it starts from those values and owns
its copy at once. Updating the palette module later keeps the colours the project
is painted with — the values are the project's, the code around them is maxx's.

And the canvas paints with the project's palette, so a colour chosen shows where
the choice was made rather than only after `cargo run`.

### The window, rearranged

Four columns, every join between them moving. The components moved out of the
right panel to sit under the project's files: the tool you reach for most while
building was below the fold, past twenty properties, and it is a source rather
than a property of the selection.

The right panel parts in two, structure above and inspector below, so selecting
a node deep in the tree no longer pushes the tree out of sight. The inspector
files properties under five foldable headings, each carrying the count of what
it hides, with a search of its own — and both search boxes stay put while their
lists scroll.

### What it will not do quietly any more

- An import written twice is pointed at with a comment on the line, at save
  time. maxx adds; it does not take away what it did not write, so one of the
  two lines being yours is why it says something instead of merging them.
- An import that was only partly there adds the names that are missing rather
  than the whole statement again. That was `E0252` in your project, on a line
  maxx had written.

### Under the floor

Four files over a thousand lines were split by subject; the code, its comments,
the CI and the scripts are in English. `tests/property.rs` states the round-trip
contract — tree → source → tree — and checks it on a thousand trees an execution
rather than on examples somebody thought of; it found a comment that gained a
comma at every save. Three tests drive the workspace with no window open, on the
commands where a mistake costs somebody's work.

## 0.1.0

The first release: a visual workshop that builds GPUI views and writes them out
as real Rust source.
