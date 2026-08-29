//! Rasterises `assets/icon.svg` into `assets/icon-1024.png`, which the `.icns`
//! is cut from.
//!
//! The drawing is SVG because that is the only shape that can be corrected: a
//! gear tooth moves in a text file, not in a million pixels. Nothing to install
//! to read it — `resvg` is already in the tree, pulled in by gpui, which uses it
//! for its own icons.
//!
//! ```sh
//! cargo run --example make_icon
//! # then, for the .icns: see README.md
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
