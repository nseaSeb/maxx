# Architecture

This document says how maxx is made, and why. `README.md` says what it does;
`BACKLOG.md` says what is missing. The three are read in that order.

The fine documentation — the contract of a function, the trap in a call — lives
in the module headers and the comments of the code, where it ages along with it.
`cargo doc --open` makes it readable.

## The rule that commands everything else

**The `.rs` file is the truth.** maxx has no screen format, no project database,
no representation that would survive a `git clone` other than as Rust.
Everything below follows from that.

The direct consequence: maxx has to be able to *read* what it did not write, and
to *rewrite* without damaging what it does not understand. That is what the
model and the parser exist to guarantee.

## The tour of the modules

| Module | Role |
|---|---|
| `model.rs` | the tree: a base, an ordered list of calls, arguments, opaque nodes |
| `parser.rs` | Rust text to model; finding the markers, splicing the text |
| `codegen.rs` | model to Rust text |
| `registry.rs` | the catalogue's types, lookup by identifier, instantiation |
| `registry/catalogue.rs` | the catalogue itself — the only place to extend to add a component |
| `registry/props.rs` | what the inspector shows, reads, and is allowed to write |
| `registry/state.rs` | state fields, handlers, what a copy has to rename |
| `registry/ids.rs` | the element identifiers gpui asks for |
| `registry/scrollbar.rs` | the scrolling box and its bar: one thing on the canvas, two in the source |
| `view.rs` | one open view: loading, saving, inserting state fields |
| `menu_model.rs` | the equivalent of the model, for a menu bar |
| `menufile.rs` | the equivalent of `view.rs`, for `src/menus.rs` |
| `scaffold.rs` | creating a project, and its three shapes |
| `scaffold/views.rs` | creating a view, renaming it, naming the one the window opens on |
| `scaffold/modules.rs` | the copied modules: versions, fingerprints, updates |
| `scaffold/{system,settings,theme,assets,window}.rs` | one copied module and its template |
| `scaffold/components.rs` | the component library copied into `src/components/` |
| `scaffold/menubar.rs` | the menu bar written into the project |
| `scaffold/templates.rs` | the shells, compiled by `examples/shapes.rs` |
| `project.rs` | the file tree shown in the project panel |
| `workspace.rs` | the window: the state, opening and closing a project |
| `workspace/views.rs` | the tabs, reading and writing a view |
| `workspace/inspector.rs` | the selection, the properties, the state fields, typing |
| `workspace/edits.rs` | dropping, duplicating, pasting, deleting, and the checkpoints |
| `workspace/handlers.rs` | a component's handler: opened, written, reached in your editor |
| `workspace/explorer.rs` | the file tree, its selection, its deletions |
| `workspace/code.rs` | the code reader: any text file, read-only |
| `workspace/menus.rs` | the menu bar editor |
| `workspace/chrome.rs` | the shell: title, welcome screen, status bar, `Render` |
| `workspace/process.rs` | `cargo run` and the output panel |
| `workspace/modules.rs` | the modules copied into the project |
| `workspace/palette_file.rs` | the project's colours, read and written from the screen |
| `themefile.rs` | a project's `src/theme.rs`: its roles, and the patch that changes one |
| `designer.rs` | what holds the panels together: tabs, side strip, drag ghost |
| `designer/canvas.rs` | the view as it will be drawn |
| `designer/tree.rs` | the structure, and where a dropped node lands |
| `designer/inspector.rs` | the properties, and what typing rewrites in the tree |
| `designer/menus.rs` | the menu bar editor |
| `designer/palette.rs` | the components on offer, and the search that finds one |
| `preferences.rs` | the settings screen |
| `about.rs` | the About window |
| `settings.rs` | what maxx remembers from one launch to the next |
| `cli.rs` | the command line, read before gpui: `new`, `--help`, `--version` |
| `actions.rs` | the actions, their handlers, the keyboard |
| `menus.rs` | maxx's own menu bar |
| `tools.rs` | the catalogue of editors and terminals, and their detection |
| `run.rs` | everything that assumes a system: `cargo`, terminal, editor, trash |
| `watch.rs` | watching the project on disk: what wakes the window |
| `theme.rs` | the palette, in two modes |
| `palette.rs` | the ⌘K palette: the menu bar, flattened |
| `locales/app.yml` | the translations, one entry per key |

## The life cycle of a view

```
src/ui/home.rs
   │  view::View::load
   ▼
the // maxx:begin / // maxx:end markers are found   (parser, textual scan)
   │
   ▼
syn parses the single expression between them
   │
   ▼
model::Node  ── base + ordered calls + children
   │                          ▲
   │  edited in the designer  │
   ▼                          │
codegen renders the expression│
   │                          │
   ▼                          │
parser::splice rewrites only the byte range between the markers
   │
   ▼
src/ui/home.rs
```

Three things to take from that cycle.

**`syn` never sees the whole file**, because it loses the comments. The managed
region is found by scanning the text, and saving touches that byte range alone.
Imports, `impl`, methods, formatting, comments outside the region: untouched by
construction, not by care.

**What maxx does not understand is carried, not lost.** An unknown method
becomes data written back verbatim; an expression that is not a builder chain —
an `if`, a `match`, a component of your own — becomes an opaque node, shown but
never rewritten.

**Opaque text is stored dedented.** `parser::splice` re-indents every line of
the block it writes: storing a slice with its file indentation would make it
gain one level per save, without bound.

The menu bar follows exactly the same cycle, with `menu_model` and `menufile` in
place of `model` and `view`. It has a model of its own because
`vec![Menu { name, items }]` is a struct literal: the node parser would degrade
it into a single opaque blob.

## The state of a window

`Workspace` is the logical root view of each window. It holds the project, the
open views, the project panel's selection, and the *mode* of the central panel —
designer, menu editor, or preferences.

Two gpui constraints, which explain things that would look contorted without
them.

**The window's real root is `gpui_component::Root`**, not `Workspace`: several
components walk up to it and abort the process when it is missing. The workspace
is therefore not reachable by downcasting the window handle; it is registered in
a global `WindowId -> WeakEntity<Workspace>` table.

**An action handler runs inside a window's own update.** Opening, activating or
updating a window from there fails silently — no error, no panic. Anything that
touches a window from `cx.on_action` goes through `cx.defer`: that is why
`about::open` is split into two functions, and why the workspace's actions go
through `workspace::defer_active`.

A third, particular to the inspector: **typing does not bump `revision` and does
not take an undo checkpoint**, because `revision` is what triggers the rebuild
of the `InputState` entities — bumping it on every keystroke would recreate the
field under the caret.

The step is therefore taken **when the field is left**, once what was typed is
already in the tree — every keystroke writes it there — and nothing is waiting
under the caret any more: one visit to a field is one `⌘Z`, not one keystroke.
The field taking focus holds on to the tree as it was; losing focus, `⏎`,
changing the selection or `⌘S` turns that tree into a step.

Two things keep that promise. The session carries **the identity of the field
that opened it**: gpui delivers one focus event to all of its listeners, in the
order they registered, so moving up the inspector makes the arriving field's
`Focus` speak before the leaving field's `Blur` — an anonymous session would be
closed by the wrong half, and typing would leave no step at all. And taking the
step **adopts the key of `sync_prop_inputs`**: `revision` moves on, so that the
code panel catches up, but the fields already hold what the tree holds, so
nothing is rebuilt under a caret that may just have arrived in the next field.

`checkpoint` takes that step before its own, for the very reason it exists: a
command reaches the workspace while a field still holds the caret, and the
keystrokes pushed afterwards would end up *above* the command — one `⌘Z` undoing
both, the next giving the word back.

## The settings

Two files, the way Zed separates them, because they are not two of the same
thing.

`settings.json` belongs to the user. It is edited by hand as much as by maxx, so
**maxx rewrites only the key it changes**: `walk` runs through the object's
members and `splice_key` replaces the value's byte range alone, exactly as
`parser::splice` does in a `.rs`. Comments and layout survive.

That walk has to know about comments, not only about strings and nesting, and
that is not a refinement: a textual search for the key finds it in a comment
that quotes it, and an odd quotation mark inside a comment leaves a naive scan
"inside a string" to the end of the file — closing brace included. A missing key
is added just after the opening brace and not before the closing one: the last
thing in an object is often a comment, and a comma added there ends up commented
out. A missing file is written with all of its defaults and a line of
explanation per key — that is the part of Zed's settings worth copying, before
any question of format.

`state.json` belongs to the machine: recent projects, window geometry. Nobody
edits it, so it is rewritten whole.

The format is JSON with comments, read by `serde_json_lenient` — the crate Zed
reads its own with, already in the tree through gpui. Strict JSON cannot carry a
comment, and a settings file you cannot annotate is one whose documentation has
to be kept somewhere else. A JSON schema is written beside it, derived from the
struct by `schemars`, so that the editor completes and flags typos.

The settings are loaded once at startup into a `Global`, and that is the **only**
source: the workspace keeps no copy of the panels' state, it reads at render
time. That is what stops the preferences screen, the menu bar and the window
from diverging — and it is necessary, gpui-component's `SettingField` reading
and writing the application without going through the view.

Three writing paths, deliberately distinct:

- `update_prefs` changes a preference and patches the user's file.
- `update_state` changes the machine state and rewrites it.
- `stage_state` changes memory only, and `flush` writes at shutdown. For what
  moves continuously: the window's geometry, where one file per frame would be
  absurd. The accepted corollary: a `kill -9` loses the geometry.

The reading principle: a missing, partial or damaged file is never worse than no
file. `serde(default)` makes a missing key fall back to its default rather than
failing the whole read, and an unreadable file is reported and then left intact
— overwriting it would lose whatever the user was in the middle of writing.

The previous version's `settings.toml` is taken over once at startup, split in
two, then renamed `settings.toml.repris` — not deleted: a migration that eats
data is a migration nobody believes.

## Opening a project: two paths, not one

`workspace::open_folder` only reaches `set_project` when it reuses a window that
is already open and empty. Otherwise — and this is also the case for
`maxx <path>` on the command line — it goes through `open_workspace_window`,
which builds `Workspace::new` without ever touching `set_project`.

Any side effect attached to "a project is being opened" therefore has to be
wired in both places. That is exactly what made recent projects fail to be
recorded on the first attempt.

## Editor and terminal

`tools.rs` holds a table, not a heuristic, and that is the point: opening a file
*at a line* is spelled differently by each of them — `zed file:12`,
`code -g file:12`, `nvim +12 file`, `idea --line 12 file` — and there is no
majority to follow. Confusing two of those forms does not fail outright:
`code file:12` opens a file named "file:12".

The trap that is invisible at the moment of choosing: `hx`, `nvim` and `vim` are
not applications but programs that need a terminal around them. The two settings
are therefore not independent, and not every terminal can be handed a command —
the macOS one gives access to it only through AppleScript, which asks for an
automation permission in the middle of a click.

`auto`, the default, takes the first one installed from the catalogue; for the
editor, `$VISUAL` and `$EDITOR` come first, because whoever set them has already
said what they want. Detection looks for the command on the `PATH` and, on
macOS, for the bundle in `/Applications`.

## The system, and nothing else

Everything that assumes a platform is in `run.rs`, and nowhere else: `cargo`,
the terminal, the editor, the trash, the cache, the way the launched process
tree is killed. `settings.rs` additionally knows the configuration directory
conventions, and `tools.rs` the detection ones.

What genuinely differs, system by system:

- **The cache and the configuration.** `XDG_*` when the user has set them,
  `LOCALAPPDATA` and `APPDATA` on Windows, `Library/Caches` and
  `Library/Application Support` on macOS, `.cache` and `.config` elsewhere.
- **The trash.** `~/.Trash` on macOS; on Linux the freedesktop specification,
  `$XDG_DATA_HOME/Trash/files` plus a `.trashinfo` without which the desktop
  does not know where the file came from and cannot restore it; on Windows a
  trash of maxx's own, because the real one is reachable only through the shell
  API — which would cost a dependency and an `unsafe` block for a gesture that
  has to stay simple. maxx says so rather than pretending.
- **Killing what was launched.** `cargo` launches the application itself:
  signalling `cargo` alone would leave the window open. On unix the child gets a
  process group of its own, which `kill -TERM -pid` reaches whole; Windows has
  no such notion and `taskkill /T` makes the equivalent gesture.
- **The application-bundle fallback.** `open -a` is a macOS tool and a `.app` a
  macOS notion: elsewhere, the command on the `PATH` is the only way in, and its
  absence is the reason nothing happened.

What the CI matrix proves: that it compiles and that the suite passes on all
three. That is what stops a line such as
`std::os::unix::process::CommandExt` from finding its way back in unseen. What
it does not prove: that the interface is usable, since no test opens a window.
maxx has only ever been tried by hand on macOS.

Two workflows, two roles. `ci.yml` gives a fast, frequent signal; `release.yml`
is the release gate: it starts on a `v*` tag, runs the whole matrix, builds in
release — which the ordinary CI never does, and an optimisation reveals what a
debug build tolerates — checks what would go into a crates.io package, then
opens the release on GitHub with the CHANGELOG's section for its body. It
attaches no binary, and that is deliberate: an executable is a distribution in
the sense the licences mean — Apache-2.0 for gpui, MPL-2.0 for option-ext — with
the notices that have to travel with it, when publishing the source asks for
none and `cargo install` is enough for anyone holding a toolchain.
`scripts/bundle-macos.sh`, which assembles a `maxx.app` around a built binary,
is therefore part of no workflow: it is run by hand. A tag remains the worst
place to *discover* a breakage, the commit being already the one you meant to
publish: this gate doubles the weekly net at the moment the mistake costs most,
it does not replace it.

What the first Windows run taught, with figures: `cargo check` 18 min, `clippy`
16 s behind it, `cargo test` **37 min**. The first two stop at the metadata;
only `cargo test` produces machine code and links the binaries. Dropping the
separate `check` therefore gains almost nothing — the cost is elsewhere, and it
comes down another way: a `ci` profile that compiles the dependencies at O0
instead of O2, with no debug information, and a Defender exclusion on the
Windows runners, where the antivirus inspects every one of the tens of thousands
of files rustc writes.

There is no local equivalent for Windows: Docker on a Mac runs Linux containers,
a Windows container demanding a Windows host. `scripts/verify-linux.sh` replays
the Linux branch, and that is all that can be replayed.

On a public repository the standard runners are free and unlimited: all three
systems run on every push. While the repository was private that was not
sustainable — minutes there are counted with multipliers, ×1 on Linux, ×2 on
Windows, ×10 on macOS, so that a single cold full run cost close to 400 billed
minutes out of a monthly allowance of 2,000. Were the repository to become
private again, the rationing the history keeps would have to come back.

## What maxx adds to a project

Six things are added to an existing project, by textual insertion and never by
rewriting from the template: a view, the menu bar, the system module, the
settings, the images, the window. The settings and the window pull the system
module along with them and declare two crates in the project's `Cargo.toml` —
inserted into the dependencies section, not at the end of the file, so that a
`[profile]` block stays after them. The project may predate maxx and do
something else at startup — it has to keep it.

The images and the window are the first two modules `main.rs` **calls** and does
not merely declare: `.with_assets(assets::Assets)` on `Application::new()`,
`window::bounds(bounds)` and `window::remember(&window, cx)` around the opening.
Hence the shape of the wiring — a rebinding that shadows rather than an
argument, a statement rather than a nested call: every line written has to be a
line whose removal leaves a file that compiles, because deleting the module
removes the line. The images module additionally carries a `build.rs`, which is
not tracked in `maxx.toml`: the contract between the two — `assets.rs` in
`OUT_DIR`, the `ASSETS` symbol — is written in the header of each.

The system module deserves its rule: it holds only what differs from one system
to the next **and** what gpui does not already provide. The clipboard,
`open_url`, `reveal_path`, `open_with_system`, the file pickers are in gpui;
wrapping them would add a layer to maintain for nothing. What is left is where
an application's files go and what "delete" means — what every desktop
application ends up writing, and what nobody wants to write a third time.

A necessary symmetry: deleting `src/<module>.rs` from the project panel removes
its `mod` line from `main.rs`, and with it the wiring the module gave itself —
the `scaffold::modules::WIRING` table says, per module, the whole statements to
remove and the fragments to take out of the line that carries them. Without it,
deleting a file breaks the build, which is the opposite of the point.

**A copy is a debt**, and it has to be named: the system module and the settings
take over code maxx wrote for itself. A defect found on one side has to be
carried to the other. It has happened already — the `.trashinfo` record,
non-conforming in both at once.

`maxx.toml`, committed at the root of the project, makes that debt recoverable.
It notes which module was copied, in which version, and the fingerprint it had
on the way out. maxx then knows which projects carry a version it has since
fixed, and the fingerprint tells it whether the developer has touched it: **a
modified file is never replaced**, it is reported. That is a third way between
extracting a crate — which would break the promise that a generated project owes
maxx nothing — and generating the template from maxx's own code.

The same file carries **the project itself**, and not only what it borrowed: the
view its window opens on, and the cargo line that launches it — profile,
features, arguments handed to the application. Both were set in stone: the entry
view in `main.rs`, the launch a bare `cargo run`. A project that wanted
`--release`, a feature or a different first screen had nowhere to say so. Every
key is optional, and a project that sets none behaves exactly as before.

Moving the entry view writes in two places that have to agree: `src/main.rs`,
which actually opens the window, and `maxx.toml`, where maxx reads it back. The
code first — a `maxx.toml` announcing an entry the code does not open would be
worse than no record at all. And it is the construction site that is
authoritative, not the `use` line: a `main.rs` may import several views, only one
is handed to `Root`.

**The managed region's comments are in the model.** `syn` throws them away —
they are not tokens — and `codegen` rewrites the region from the model:
everything the model does not carry is erased on the first save. The reader
therefore scans the region's text before handing it to `syn`, skipping strings,
raw strings and character literals, then attributes each comment to **what
follows it** — the walk through the chain goes in file order, and a queue of
comments empties as it goes. Hence three places in the model: above a call
(`Call::comments`), above a node (`Node::comments`, written by the parent, which
knows the column), and after the last call (`Node::trailing`).

Two traps the mechanism has to avoid and that the tests hold: a comment written
**inside** an argument — a closure, a `match` kept verbatim — is already in that
argument's text, so it is taken off the queue without being kept, or it would be
written twice; and a chain carrying a comment is never rendered on a single
line, a comment having nowhere to go on one.

**Two calls do not live on an ordinary element**: scrolling and the tooltip.
gpui offers them only to a *stateful* element, that is, once `id` has been set —
written in the other order they do not compile, in the developer's project and
on a line maxx wrote. Hence targets of their own, `Target::Scrollable` and
`Target::Tooltip`, which set the `id` first. And hence `Common::Element`, which
only the column, the row and the spacer carry: no `gpui-component` component is
a gpui element, so a tooltip offered on all of them would be a call that does
not exist.

**What the catalogue imports** has been conditional ever since it started
writing variants. A call such as `.primary()` or `.disabled(…)` comes from a
trait, and a trait has to be in scope — but importing it as soon as the
component is seen leaves an unused `use` on the button that has no variant, and
so a warning in a project maxx has just written. `Spec::extra_imports` therefore
holds pairs of "these calls ask for this line", and the condition is per
component: `outline` is a button variant *and* a badge flag, so that a shared
table of calls would import the button's trait into a file that holds only
badges.

**The project shapes** are the third kind of code maxx writes, after the views
it draws and the modules it copies. `src/ui/shell.rs` — a sidebar and the view
of the moment — is ordinary Rust written once at creation: no `maxx:` markers,
no version, no catching up. That is deliberate, and it is the difference from a
module: a shell is the shape of the project, which the developer will make their
own from the first page added.

They posed a problem the views do not have: nothing compiled them.
`src/scaffold/templates.rs` depends on nothing, so `build.rs` includes it, calls
the same functions the projects receive and writes their output into `OUT_DIR`;
`examples/shapes.rs` compiles it there, against a view and a settings module
reduced to their surface. A `clippy --all-targets` therefore catches a method
`gpui-component` does not have. What it does not catch — the wiring of
`main.rs`, the declared crates, the whole settings module — is in
`tests/project.rs::every_shape_compiles`, ignored by default because it builds
two entire projects.

**The fingerprint cannot be the bytes alone.** A project formatted by its
developer — `cargo fmt`, the most ordinary gesture there is — is no longer byte
for byte the one maxx wrote, even though no line of code has moved: the default
layout moves ten lines of `system.rs` and fifty-six of `theme.rs`, and goes as
far as writing `else { return; }` where the template says `else { return }`.
maxx concluded "edited by the developer" and stopped there, without a word.
`maxx.toml` therefore carries **two** fingerprints: the bytes, and the shape —
the same text put through the default `rustfmt`, the project's configuration
ignored, because what is wanted here is a fixed standard and not the one of the
moment. Either one is enough to recognise a file; both have to fail for it to be
held as edited.

The guard that holds the whole thing together is in `tests/modules.rs`: it keeps
each template's fingerprint at its current version. Changing a template makes
that test fail, which forces a decision about whether the fix should reach the
projects already written. Without it, a version would never be raised and the
mechanism would be decorative.

## Formatting what maxx writes

A setting, off by default, runs `rustfmt` over the file after every write.
`rustfmt` and not `cargo fmt`: the second formats a whole crate, the first takes
a file and finds the project's `rustfmt.toml` on its own by walking up from it —
the developer's conventions win over `codegen`'s.

It is also the only place where maxx launches a process and waits for it,
against the rule laid down elsewhere: it has to read the file back afterwards.
And that re-read is not optional — maxx keeps a copy of the file and compares it
to the disk to spot what changed outside it. Leaving the copy behind would make
the next save believe someone had touched the file: maxx accusing itself.

Why it is on by default, and this is the design point: a Rust editor formats on
save — that is the default in Zed as in rust-analyzer — and `codegen` does not
write what rustfmt would write. Checked rather than assumed: rustfmt rewrites
the demo's managed region. Without this setting, the editor reformats what maxx
wrote, maxx rewrites it its own way on the next save, and the two pass the ball
back and forth with one spurious diff per turn. maxx therefore applies itself
what the editor would apply anyway.

A consequence to state honestly: **maxx's round trip is neutral up to rustfmt**,
and it is that composition `tests/demo.rs` checks. The template itself already
comes out in rustfmt's format — a freshly generated project comes back through
unchanged, which a test observes.

It remains that rustfmt formats the whole file, so beyond the managed region. On
a project already put through the formatter that changes nothing elsewhere; on a
project that ignores it, the setting goes off.

## An entry's shortcut

In gpui a shortcut is not a property of the menu entry: it is a separate list,
`key_bindings`, which the bar reads to display the accelerator. It lives outside
the managed region, in a function the developer edits too.

A first attempt, set aside: writing the shortcut to disk at the moment it is
typed, since the model did not carry it. That was wrong on four counts at once —
one write per key, so one for every intermediate state of the keystroke; a write
that went around the "file changed outside maxx" guard; a shortcut written for
an action `actions!` did not declare yet; and a shortcut that outlived the entry
being renamed or deleted.

What settles all four: **the shortcut is in the model**, read at opening and set
on the entry, written at `⌘S` with the rest. It then travels with the entry you
rename or move, it leaves with it when you delete it, and it is written after
`ensure_action`, never before.

Two edge rules. Every line binding an action is removed before one is written,
because gpui accepts several for the same action: rewriting only one would leave
the old one alive. And a binding for an action that appears in no menu belongs
to the developer — saving does not touch it.

## The repository's formatting

maxx goes through `rustfmt`, with a single exception: `use_small_heuristics =
"Max"`, which lets a short expression stay on its line. That is what preserves
the tables of `registry/catalogue.rs`, the file others are invited to extend —
reading a list of styles there at one word per line would be a punishment. The
rest is the default rustfmt.

The reason for going along with it is the same as for generated projects: a Rust
editor formats on save. With no shared reference, the first contributor to open
a file in Zed reformats the whole thing and their real change drowns in it.
`cargo fmt --check` in CI closes the question.

The demo has a `rustfmt.toml` of its own, empty, and is therefore not concerned:
it has to be formatted the way a project generated elsewhere is, at the default
rustfmt.

## The demo as a reference

`demo/` is a complete project, committed, with a workspace root of its own — so
`cargo check` at the root of the repository does not build it. It is written in
the exact form `codegen` produces, which makes a property checkable:
`tests/demo.rs` reads every view back, rewrites it, and demands the file be
identical to the byte. Any divergence is a loss.

It is also the only reference for what maxx has to understand. It replaced an
absolute path to a personal folder, in a test that stopped without failing when
it was missing: on anybody else's machine the coverage was nil, and silent.

## Where to plug what in

- **One more component**: `registry/catalogue.rs`, one entry. Nothing else to
  touch. If it needs an entity the view owns — a text field, a dropdown — the
  entry additionally carries a `StateSpec`: the field's type, its imports, and
  the expression `new` gives it. That is all `view::save` needs to know to
  declare the field and initialise it, and the list of fields offered in the
  inspector is filtered on that type — offering a text field's field to a
  dropdown would be offering what does not compile.
- **One more setting**: a field in `settings::Preferences` with its default, a
  line in `documented_defaults`, then a `SettingItem` in `preferences.rs`. The
  field reads and writes the settings, it copies nothing.
- **One more menu entry**: an action in `actions.rs`, its handler, and the line
  in `menus.rs`. An action carrying data cannot come from the `actions!` macro —
  see `OpenRecent`.
- **One more system call**: `run.rs`, and nowhere else.
