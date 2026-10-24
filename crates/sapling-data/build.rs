//! `data/names.tsv` and `data/command-names.tsv` are made by probes of this workspace (README.md
//! "Provenance"), so a build without them yet gets empty tables: names then fall back to what the
//! code makes of identifiers, and commands take every name.

fn main() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    for file in ["names.tsv", "command-names.tsv"] {
        std::fs::write(out.join(file), std::fs::read(format!("data/{file}")).unwrap_or_default()).expect("OUT_DIR is writable");
        println!("cargo:rerun-if-changed=data/{file}");
    }
}
