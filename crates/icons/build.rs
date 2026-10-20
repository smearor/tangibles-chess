//! Build script: exports the whitelisted icon SVGs declared in
//! `icons.toml` and generates the iconset module (`iconset.rs`).

fn main() -> miette::Result<()> {
    fonts_rs_generator::IconSetBuild::from_file("icons.toml")?.run()?;
    Ok(())
}
