//! Handlers: the method a component calls, opened, written into the view's
//! `impl`, and reached in the editor.

use super::*;

impl Workspace {
    /// Opens what is being edited in Zed: the file if one is open, the project
    /// otherwise.
    ///
    /// Opening the folder when a view is on screen means finding the file again
    /// by hand, which is the whole gesture one wanted to avoid.
    pub fn open_in_editor(&mut self, cx: &mut Context<Self>) {
        // The explorer selection comes first: it is what the context menu is
        // about, and a left click sets it to the open view anyway.
        let path = self
            .selected
            .clone()
            .or_else(|| self.menu_file.as_ref().map(|menus| menus.path.clone()))
            .or_else(|| self.view().map(|view| view.path.clone()))
            .or_else(|| self.project().map(|project| project.root.clone()));

        match path {
            Some(path) => crate::tools::open_in_editor(cx, &path, None),
            None => {
                self.message = Some(crate::tr("message.no_project"));
                cx.notify();
            }
        }
    }

    /// Opens the handler of a property in Zed, on its own line.
    pub fn open_handler(&mut self, prop: &'static crate::registry::Prop, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let node = view.selected();
        let Some(name) = registry::read(node, prop).filter(|name| !name.is_empty()) else {
            self.message = Some(crate::tr("message.no_action"));
            cx.notify();
            return;
        };
        match view.method_line(&name) {
            Some(line) => crate::tools::open_in_editor(cx, &view.path, Some(line)),
            None => {
                self.message = Some(SharedString::from(
                    t!("message.handler_unwritten", name = name).into_owned(),
                ));
                cx.notify();
            }
        }
    }

    /// Fills the selected node's handler with the body that opens a box.
    ///
    /// Written straight to the file rather than into the tree: a handler is not
    /// part of the managed region — it is a method of the view, beside it — so
    /// there is nothing here for `⌘S` to carry. The view is re-read afterwards,
    /// for the reason `format_after_save` gives: maxx holds a copy of the file
    /// and compares it with the disk, and a copy left behind would make the
    /// next save believe someone else had written.
    pub fn fill_handler(
        &mut self,
        prop: &'static crate::registry::Prop,
        kind: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.view() else {
            return;
        };
        let Some(name) = registry::read(view.selected(), prop).filter(|name| !name.is_empty())
        else {
            self.message = Some(crate::tr("message.no_action"));
            cx.notify();
            return;
        };

        // The gate `add_state_field` applies, and for the same reason: this
        // writes the file from maxx's copy of it, so a file changed elsewhere
        // would be overwritten with a version that predates the change.
        if view.disk_changed() {
            self.message = Some(crate::tr("error.changed_on_disk_reload"));
            cx.notify();
            return;
        }

        let filled = match crate::view::fill_handler(&view.source, &name, kind) {
            Ok(filled) => filled,
            Err(error) => {
                self.message = Some(SharedString::from(error));
                cx.notify();
                return;
            }
        };

        let path = view.path.clone();
        if let Err(error) = std::fs::write(&path, &filled) {
            self.message = Some(SharedString::from(error.to_string()));
            cx.notify();
            return;
        }
        // The copy is taken forward rather than re-read. A handler lives beside
        // the managed region, so nothing in the tree changed — and `reload`
        // would replace the tree from disk, which is the last saved one: an
        // unsaved edit on the canvas, and the undo stack behind it, would go
        // with it. `add_state_field` writes the same way, for the same reason.
        if let Some(view) = self.view_mut() {
            view.source = filled;
        }
        self.message = Some(SharedString::from(
            t!("message.handler_filled", name = name, kind = kind).into_owned(),
        ));
        self.revision += 1;
        cx.notify();
    }

    /// Gives the selected node a handler, named after it, if its component has
    /// an action property and none is set yet. Bound to double-click.
    pub fn add_handler_to_selection(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let node = view.selected();
        let Some(spec) = registry::of(node) else {
            return;
        };
        let Some(prop) =
            spec.props.iter().find(|prop| matches!(prop.kind, crate::registry::Kind::Handler))
        else {
            return;
        };
        if registry::read(node, prop).is_some_and(|name| !name.is_empty()) {
            return;
        }

        let name = registry::suggested_handler(node);
        let selected = view.selected.clone();
        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if let Some(node) = view.root.at_mut(&selected) {
            registry::write(node, prop, &name);
        }
        self.message =
            Some(SharedString::from(t!("message.action_written", name = name).into_owned()));
        cx.notify();
    }
}
