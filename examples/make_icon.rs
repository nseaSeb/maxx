//! Rasterise `assets/icon.svg` en `assets/icon-1024.png`, d'où vient l'`.icns`.
//!
//! Le dessin est en SVG parce que c'est la seule forme qui se corrige : une
//! dent d'engrenage se déplace dans un fichier texte, pas dans un million de
//! pixels. Rien à installer pour le lire — `resvg` est déjà dans l'arbre, tiré
//! par gpui, qui s'en sert pour ses propres icônes.
//!
//! ```sh
//! cargo run --example make_icon
//! # puis, pour l'.icns : voir README.md
//! ```

fn main() {
    let source = std::env::args().nth(1).unwrap_or_else(|| "assets/icon.svg".into());
    let target = std::env::args().nth(2).unwrap_or_else(|| "assets/icon-1024.png".into());
    let size: u32 = std::env::args().nth(3).map(|s| s.parse().expect("taille")).unwrap_or(1024);

    let data = std::fs::read(&source).unwrap_or_else(|error| panic!("{source} : {error}"));
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default())
        .unwrap_or_else(|error| panic!("{source} : {error}"));

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(&target).unwrap_or_else(|error| panic!("{target} : {error}"));
    println!("{target} — {size} px");
}
