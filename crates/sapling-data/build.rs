//! `data/names.tsv` is made by a probe of this workspace (README.md "Provenance"), so a build without
//! it yet gets an empty table: names then fall back to what the code makes of identifiers.

fn main() {
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("names.tsv");
    std::fs::write(out, std::fs::read("data/names.tsv").unwrap_or_default()).expect("OUT_DIR is writable");
    println!("cargo:rerun-if-changed=data/names.tsv");
}
