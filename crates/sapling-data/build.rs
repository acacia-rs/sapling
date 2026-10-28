//! `data/names.tsv`, `data/command-names.tsv` and `data/command-texts.tsv` are made from your own
//! BDS (README.md "Provenance"), so a build without them yet gets empty tables: names then fall
//! back to what the code makes of identifiers, commands take every name, and the console prints
//! translation keys.

fn main() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    for file in ["names.tsv", "command-names.tsv", "command-texts.tsv"] {
        std::fs::write(out.join(file), std::fs::read(format!("data/{file}")).unwrap_or_default()).expect("OUT_DIR is writable");
        println!("cargo:rerun-if-changed=data/{file}");
    }
}
