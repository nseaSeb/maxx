# A maxx demonstration

An ordinary `gpui` + `gpui-component` project, written in the shape maxx
produces and reads back. It does not depend on maxx: `cargo run` is enough.

```sh
cd demo
cargo run
```

## What it shows

**The catalogue's components**, all in `src/ui/home.rs`: group box, label, text
field bound to a field of the view, checkbox, switch, radio, tag, progress bar,
divider, alert, link, and a button with a tooltip and a handler.

**A window opened from the menu bar** — `Window > Open the inspector`, or `⌘I`,
or the button on the home view. That is the gesture bringing together two traps
nothing warns you about at the moment you fall into them:

- an action handler runs inside the update of the window that dispatched it, and
  gpui refuses to enter a second one. Opening a window straight from
  `cx.on_action` does **nothing at all** — no error, no panic. Hence the
  `cx.defer` in `src/menus.rs`;
- a window drawing the smallest `gpui-component` widget has to be rooted in
  `Root`. Several components walk up to it and abort the process when it is
  missing. The About window, which uses nothing but bare gpui, is the only one
  that does without it.

**A palette in two modes** — `src/theme.rs`, written by maxx: roles rather than
colours, two values each, and the switch labelled "Dark" moves the whole window
because it goes through `gpui_component`'s own theme. Note the handler's shape:
`Switch::on_click` hands the state it moved to, not a click event, and that is
what maxx writes for a switch.

**An editable menu bar.** Open `demo/` in maxx and click `src/menus.rs`: the
menu editor appears.

## Why it is in the repository

It is the tests' reference: `tests/demo.rs` checks that maxx reads every view
back, that rewriting is neutral to the byte, and that the menu bar reads back
with its window-opening entry. A demo that breaks fails the suite.
