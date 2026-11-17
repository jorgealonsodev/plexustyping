//! Prints the licence notices that the embedded `two-face` syntax definitions
//! require, as Markdown for `THIRD_PARTY_NOTICES.md`.
//!
//! Run with `cargo run -p plexustyping-highlight --example acknowledgements`.

fn main() {
    let listing = two_face::acknowledgement::listing();
    let mut md = String::new();
    for license in listing.for_syntaxes() {
        license.write_md(&mut md);
    }
    print!("{md}");
}
