use gpui::{
    App, Bounds, Context, Menu, MenuItem, OsAction, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};
use gpui_component::Root;

use crate::ui::inspecteur::Inspecteur;

actions!(
    app,
    [
        About,
        Quit,
        HideApp,
        HideOthers,
        ShowAll,
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        Minimize,
        OuvrirInspecteur
    ]
);

/// Wires what the menu entries do.
pub fn register(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| open_about(cx));
    cx.set_global(Inspecteurs::default());
    cx.on_action(|_: &OuvrirInspecteur, cx: &mut App| ouvrir_inspecteur(cx));
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
        KeyBinding::new("cmd-i", OuvrirInspecteur, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// Ouvre l'inspecteur dans sa propre fenêtre.
///
/// Deux choses que cette fonction existe pour montrer.
///
/// D'abord `cx.defer` : un gestionnaire d'action tourne à l'intérieur de la
/// mise à jour de la fenêtre qui l'a émis, et gpui refuse d'en entrer une
/// seconde. Ouvrir une fenêtre directement depuis `cx.on_action` ne fait rien
/// du tout — sans erreur, sans panique, ce qui est bien pire.
///
/// Ensuite `Root` : une fenêtre qui dessine le moindre composant de
/// gpui-component doit être enracinée dedans. Plusieurs composants remontent
/// jusqu'à lui et interrompent le processus s'il manque.
pub fn ouvrir_inspecteur(cx: &mut App) {
    cx.defer(|cx: &mut App| {
        // Une seule à la fois. La fenêtre est retenue dans un global : toutes
        // les fenêtres de la démo sont enracinées dans `Root`, donc les
        // distinguer par leur type est impossible.
        let known = cx.global::<Inspecteurs>().0;
        if let Some(handle) = known
            && cx.windows().contains(&handle)
        {
            let _ = handle.update(cx, |_, window, _| window.activate_window());
            return;
        }

        let bounds = Bounds::centered(None, size(px(520.), px(420.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Inspecteur")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Inspecteur::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .map(|handle| cx.set_global(Inspecteurs(Some(handle.into()))))
        .ok();
    });
}

/// La fenêtre de l'inspecteur, quand elle est ouverte.
#[derive(Default)]
struct Inspecteurs(Option<gpui::AnyWindowHandle>);

impl gpui::Global for Inspecteurs {}

/// La fenêtre « À propos », en gpui pur.
///
/// Pas de composant ici, donc pas besoin de `Root` : c'est la seule fenêtre de
/// la démo qui s'en passe.
fn open_about(cx: &mut App) {
    cx.defer(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(320.), px(180.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("À propos")),
                    ..Default::default()
                }),
                is_resizable: false,
                ..Default::default()
            },
            |_window, cx| cx.new(|_| APropos),
        )
        .ok();
    });
}

struct APropos;

impl Render for APropos {
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
            .child(div().text_2xl().child(env!("CARGO_PKG_NAME")))
            .child(
                div()
                    .text_color(rgb(0x7f8896))
                    .child(format!("version {}", env!("CARGO_PKG_VERSION"))),
            )
    }
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
            items: vec![
                MenuItem::action("Ouvrir l'inspecteur", OuvrirInspecteur),
                MenuItem::separator(),
                MenuItem::action("Réduire", Minimize),
            ],
        },
    ]
    // maxx:end
}
