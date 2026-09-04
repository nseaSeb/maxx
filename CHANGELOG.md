# Changelog

What changed, for whoever opens maxx and wonders what is new. The commits say
how; this says what for.

## 0.3.0

### Building without reaching for the menu bar

Everything maxx could do was reachable from the menu bar or from `⌘K`, which
is true and is the problem: the hand that builds a view is on the canvas, and
the menu bar is at the other end of the screen.

- **Right-click, everywhere.** On a node of the canvas or of the structure
  tree: duplicate, copy, paste, wrap, unwrap, move up or down, delete, see the
  code, go to the handler. On a tab: close, close the others, close those to
  the right, reveal in the project, copy the path, open in your editor. On the
  palette: insert before, after, or inside the selection. In the menu editor:
  the verbs its buttons already offered.
- **Wrap and unwrap.** "These two buttons belong together, in a row" is the
  most frequent gesture of laying anything out, and it did not exist. `⌘⌥G`
  wraps in a column, `⌘⌥⇧G` in a row, `⌘⌥U` takes the box away.
- **The keyboard in the tree.** Arrows for the selection and the depth, `⌥↑`
  and `⌥↓` to move a node, `⏎` to type what the node says, `⌫` to delete —
  in a context of their own, so nothing is taken from the rest of the window.
- **Type on the canvas.** Double-click a label or a button and the box opens
  over it, where the words are read rather than twenty properties away.
- **Handles.** Drag the right or bottom edge of the selected node to write its
  width and height. One drag is one undo step.
- **`⌘Z` covers typing.** Text edits escaped it entirely; a word typed is now
  one step, taken when you leave the field.
- The output panel and the menu editor's inspector resize like every other
  panel, and the welcome screen shows each recent project as a picture of the
  view its window opens on.

### More to drop, and more to say about it

- **Nine components**: avatar, breadcrumb, keyboard key, clipboard, tab bar,
  date picker, calendar, number field, one-time-code field — plus SVG, and a
  multi-line switch on the text field.
- **Style you had to type in Zed.** Margins, borders and their colour,
  minimum and maximum sizes, justification, wrapping, opacity, shadows,
  italic, underline, truncation, no-wrap, line height, clipping, the pointer
  cursor. The shared properties went from four to seventeen, filed under a
  heading of their own so a button is not buried under them.
- **Hover.** A second chain of style that applies only under the pointer, and
  the canvas shows it.
- **Eighty-six icons** instead of twenty-two, read out of `gpui-component`
  itself rather than copied by hand.
- **The entries of a dropdown** are a property now, written into the
  initializer maxx already writes.

### Projects that do something

The three shapes answered "what holds what". None answered "what does it do",
so the first three days of a desktop project started over anyway.

- **Six shapes more**: list and detail, form, dashboard, wizard, utility,
  editor. A shape declares its own state, which is what makes the form's
  fields real fields.
- **Five subtree templates more** — a form field, a page header, a status bar,
  an empty state, a Cancel/OK row — and a template may now carry state and
  handlers of its own.
- **`maxx new <path> --shape …`** creates a project from the command line,
  without opening a window.

### What will not be lost any more

Every one of these compiled, and several reached the file the developer was
working in.

- **Unwrapping a box kept its sentences.** The comments written above the
  box's own calls went with the box, silently, on the developer's words.
- **A dropped dropdown owned its own state.** Dragging one in bound it to the
  same entity as the last, so two of them mirrored each other once the project
  ran. Only the text field was spared.
- **An initializer you extended is yours.** A `.placeholder("…")` added by
  hand made maxx read the line as its own; the next toggle rewrote it whole.
- **The scrollbar is one piece.** Moving, wrapping or promoting half of its
  assembly left a shape the inspector no longer recognised: the switch read
  *off* while the file still held the bar, and turning it on wrote a second
  one.
- **Eleven components ignored every style.** Their properties were written to
  the file correctly and had no effect on the canvas at all.
- **`⏎` on a button typed its label**, not its element id — which used to take
  the tooltip and the handler hung on that id with it.
- An emptied shortcut says so instead of quietly returning; a border colour
  maxx cannot read no longer hides the border; the demo button no longer
  offers a `demo` folder that happens to sit in your home directory; and
  `maxx new .` names the crate after the directory you are in.

### Under the floor

A test now reads the modules of `gpui-component` and fails on one nobody has
classified, so a version bump cannot quietly add a component the catalogue
never hears about. And the test suite no longer writes to fixed temporary
names, so two runs at once stop failing each other.

## 0.2.1

### An import maxx adds is an import that compiles

maxx writes the `use` lines a view needs into your file. It could put them where
Rust does not accept them, and the file you were working in stopped compiling on
a line you had not written.

- **Above the `//!`.** A view carrying no import yet got its first one before the
  file's own doc comment, which is not merely odd — inner attributes have to come
  first, and the file is refused.
- **Between an attribute and what it decorates.** The `use` landed under a
  `#[derive(…)]` or a `///`, so the attribute applied to the import instead of to
  your struct.
- **At the bottom of the file.** A `use` written below an `impl` — which anyone
  leaves behind while moving a type around — pulled every import maxx adds down
  with it, out of the block where imports are read. That one compiled, which is
  why it could last.

The worst of it was that a line put in the wrong place stayed there: maxx found
it in the text wherever it had landed and never wrote it again in the right one.
So a single bad save was permanent, and the import the view needed was never
added at all.

### While you are still typing

A view being edited is, for a moment, a file Rust cannot parse — and that is
exactly when maxx is likely to save it. In that moment the import could be
written inside a module body, inside a block comment, or at the very bottom of
the file, and the same permanence applied. maxx now reads the header the same
way whether or not the file parses.

### Under the floor

The five tests that scaffold whole projects and compile them — the only proof
that what maxx writes holds together — now run every Monday in the CI instead of
only by hand. And what maxx does to your undo history while you type in the
inspector is under test: a word typed is one step, not one per keystroke, and
never a rebuild of the box under the caret.

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
