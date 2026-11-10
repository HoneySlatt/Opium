// Belongs in: crates/navidrome/src/lib.rs, inside its `mod tests` (it uses the crate's private
// `convert::Urls` through `super::`).
//
// Run: cargo test -p navidrome a_password_typed_in_the_address
//
// What it shows: an address typed as `https://user:password@host` (the form a server behind a
// proxy's basic authentication takes, and one reqwest honours, from memory and not checked, by turning it into an
// `Authorization: Basic` header) is kept as typed (lib.rs:105-111), and every cover's address
// starts with it (convert.rs:33-36). Those addresses are kept unsigned on purpose, to be written
// to the library's cache, the history and Opium's playlists, and shared with the paired devices
// (ADR-0013): the password goes with them. Today the test fails; refusing such an address, or
// keeping the address without its user and password, makes it pass.

#[test]
fn a_password_typed_in_the_address_goes_into_no_kept_address() {
    let typed = "https://me:hunter2@music.example.org";
    let Ok(credentials) = super::Credentials::new(typed, "me", "a password") else {
        // Refused, saying why: nothing is kept.
        return;
    };
    let urls = super::convert::Urls {
        base: credentials.url.clone(),
        auth: credentials.query(),
    };
    let cover = urls.cover(Some("al-1")).expect("a cover's address");
    assert!(
        !cover.url_template.contains("hunter2"),
        "a kept cover address holds the password: {}",
        cover.url_template
    );
}
