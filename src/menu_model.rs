//! The menu bar of a generated project, as data.
//!
//! Menus do not go through [`crate::model`]: `vec![Menu { .. }]` is a struct
//! literal, not a builder chain, and forcing it into the node model would mean
//! either an opaque blob or a bespoke builder in the generated project. Plain
//! `Menu` / `MenuItem` is what a GPUI developer writes, so that is what maxx
//! reads and writes.

use syn::{Expr, Lit};

/// One menu of the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuDef {
    /// Title shown in the bar.
    pub name: String,
    /// Entries, in order.
    pub items: Vec<ItemDef>,
    /// Source text of a menu maxx did not understand, re-emitted unchanged.
    ///
    /// A menu whose `items` is not a literal `vec![..]` used to make the whole
    /// file unopenable, which contradicted the carry-through promise the
    /// entries already honoured.
    pub opaque: Option<String>,
}

impl MenuDef {
    /// A menu with a title and no entries.
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), items: Vec::new(), opaque: None }
    }

    /// Whether maxx can edit this menu.
    pub fn is_opaque(&self) -> bool {
        self.opaque.is_some()
    }
}

/// One entry of a menu.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemDef {
    /// A line between groups.
    Separator,
    /// An entry that dispatches an action.
    Action {
        /// Label shown in the menu.
        label: String,
        /// Name of the action type, as written in `actions!`.
        action: String,
        /// The system action it stands for, if any: `Copy`, `Undo`…
        os_action: Option<String>,
    },
    /// A menu inside a menu.
    ///
    /// One level, and one only. macOS allows deeper, but a submenu of a
    /// submenu is a place nobody finds twice, and stopping here keeps
    /// [`crate::menufile::Selection`] a plain `Copy` triple instead of a path.
    Submenu(MenuDef),
    /// An entry maxx did not recognize, kept as source text.
    Opaque(String),
}

impl ItemDef {
    /// The label, for the tree.
    pub fn label(&self) -> String {
        match self {
            ItemDef::Separator => "———".into(),
            ItemDef::Action { label, .. } => label.clone(),
            ItemDef::Submenu(menu) => menu.name.clone(),
            ItemDef::Opaque(_) => "code Rust".into(),
        }
    }
}

/// Reads `vec![Menu { .. }, ..]` into menus.
///
/// Anything unexpected inside a menu becomes an [`ItemDef::Opaque`] carrying its
/// source text, so it comes back out unchanged.
pub fn parse(expr: &Expr, source: &str) -> Option<Vec<MenuDef>> {
    let entries = macro_elements(expr)?;
    Some(entries.iter().map(|entry| parse_menu(entry, source)).collect())
}

fn parse_menu(expr: &Expr, source: &str) -> MenuDef {
    let opaque = |expr: &Expr| MenuDef {
        name: "code Rust".into(),
        items: Vec::new(),
        opaque: Some(text(expr, source)),
    };

    let Expr::Struct(structure) = expr else {
        return opaque(expr);
    };
    if !path_ends_with(&structure.path, "Menu") {
        return opaque(expr);
    }

    let mut name = String::new();
    let mut items = Vec::new();
    for field in &structure.fields {
        let syn::Member::Named(ident) = &field.member else {
            continue;
        };
        match ident.to_string().as_str() {
            "name" => name = string_of(&field.expr).unwrap_or_default(),
            "items" => match macro_elements(&field.expr) {
                Some(entries) => {
                    items = entries.iter().map(|item| parse_item(item, source)).collect()
                }
                // `items: build_items()` — readable, but not editable.
                None => return opaque(expr),
            },
            _ => {}
        }
    }
    MenuDef { name, items, opaque: None }
}

fn parse_item(expr: &Expr, source: &str) -> ItemDef {
    let Expr::Call(call) = expr else {
        return ItemDef::Opaque(text(expr, source));
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return ItemDef::Opaque(text(expr, source));
    };
    let name =
        path.path.segments.last().map(|segment| segment.ident.to_string()).unwrap_or_default();

    let mut args = call.args.iter();
    match name.as_str() {
        "separator" => ItemDef::Separator,
        "submenu" => match args.next() {
            // Un sous-menu dont le contenu n'est pas un littéral — `submenu(
            // build())` — reste opaque : il est lisible, pas modifiable.
            Some(inner) => match parse_menu(inner, source) {
                MenuDef { opaque: Some(_), .. } => ItemDef::Opaque(text(expr, source)),
                menu => ItemDef::Submenu(menu),
            },
            None => ItemDef::Opaque(text(expr, source)),
        },
        "action" => match (args.next().and_then(string_of), args.next()) {
            (Some(label), Some(action)) => {
                ItemDef::Action { label, action: path_text(action, source), os_action: None }
            }
            _ => ItemDef::Opaque(text(expr, source)),
        },
        "os_action" => match (args.next().and_then(string_of), args.next(), args.next()) {
            (Some(label), Some(action), Some(os)) => ItemDef::Action {
                label,
                action: path_text(action, source),
                os_action: Some(last_segment(os).unwrap_or_else(|| text(os, source))),
            },
            _ => ItemDef::Opaque(text(expr, source)),
        },
        _ => ItemDef::Opaque(text(expr, source)),
    }
}

/// Renders menus as the Rust the generated project holds.
pub fn render(menus: &[MenuDef]) -> String {
    let mut out = String::from("vec![\n");
    for menu in menus {
        if let Some(source) = &menu.opaque {
            out.push_str("    ");
            out.push_str(source);
            out.push_str(",\n");
            continue;
        }
        out.push_str("    Menu {\n");
        out.push_str(&format!("        name: \"{}\".into(),\n", escape(&menu.name)));
        if menu.items.is_empty() {
            out.push_str("        items: vec![],\n");
        } else {
            out.push_str("        items: vec![\n");
            for item in &menu.items {
                out.push_str("            ");
                out.push_str(&render_item(item, 12));
                out.push_str(",\n");
            }
            out.push_str("        ],\n");
        }
        out.push_str("    },\n");
    }
    out.push(']');
    out
}

/// Renders one entry, its continuation lines indented from `column`.
///
/// The column is passed rather than assumed because a submenu spans several
/// lines: the caller places the first one, the entry has to place the rest.
/// `parser::splice` re-indents the whole block afterwards, so what matters here
/// is only that the lines are consistent with each other.
fn render_item(item: &ItemDef, column: usize) -> String {
    let pad = " ".repeat(column);
    match item {
        ItemDef::Separator => "MenuItem::separator()".into(),
        ItemDef::Action { label, action, os_action: None } => {
            format!("MenuItem::action(\"{}\", {action})", escape(label))
        }
        ItemDef::Action { label, action, os_action: Some(os) } => {
            format!("MenuItem::os_action(\"{}\", {action}, OsAction::{os})", escape(label))
        }
        ItemDef::Submenu(menu) => {
            if let Some(source) = &menu.opaque {
                return source.clone();
            }
            let mut out = String::from("MenuItem::submenu(Menu {\n");
            out.push_str(&format!("{pad}    name: \"{}\".into(),\n", escape(&menu.name)));
            if menu.items.is_empty() {
                out.push_str(&format!("{pad}    items: vec![],\n"));
            } else {
                out.push_str(&format!("{pad}    items: vec![\n"));
                for item in &menu.items {
                    out.push_str(&format!("{pad}        "));
                    out.push_str(&render_item(item, column + 8));
                    out.push_str(",\n");
                }
                out.push_str(&format!("{pad}    ],\n"));
            }
            out.push_str(&format!("{pad}}})"));
            out
        }
        ItemDef::Opaque(source) => source.clone(),
    }
}

/// The elements of a `vec![a, b, c]`.
fn macro_elements(expr: &Expr) -> Option<Vec<Expr>> {
    let Expr::Macro(macro_expr) = expr else {
        return None;
    };
    if !path_ends_with(&macro_expr.mac.path, "vec") {
        return None;
    }
    let parser = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated;
    let parsed = macro_expr.mac.parse_body_with(parser).ok()?;
    Some(parsed.into_iter().collect())
}

fn path_ends_with(path: &syn::Path, name: &str) -> bool {
    path.segments.last().is_some_and(|segment| segment.ident == name)
}

fn string_of(expr: &Expr) -> Option<String> {
    // `"Fichier".into()` as well as `"Fichier"`.
    if let Expr::MethodCall(call) = expr {
        return string_of(&call.receiver);
    }
    let Expr::Lit(literal) = expr else {
        return None;
    };
    match &literal.lit {
        Lit::Str(value) => Some(value.value()),
        _ => None,
    }
}

/// The action's path as written: `Open` stays `Open`, `file::Open` stays
/// qualified — flattening it to `Open` stops the project compiling on the first
/// save.
fn path_text(expr: &Expr, source: &str) -> String {
    let Expr::Path(path) = expr else {
        return text(expr, source);
    };
    path.path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn last_segment(expr: &Expr) -> Option<String> {
    let Expr::Path(path) = expr else {
        return None;
    };
    path.path.segments.last().map(|segment| segment.ident.to_string())
}

fn text(expr: &Expr, source: &str) -> String {
    use syn::spanned::Spanned as _;
    match source.get(expr.span().byte_range()) {
        Some(slice) => slice.to_string(),
        None => {
            use quote::ToTokens as _;
            expr.to_token_stream().to_string()
        }
    }
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
