//! The key downloads are sealed with, kept in the desktop's keyring: made
//! the first time, the same one after.

use std::collections::HashMap;

use crate::sealed::{Key, new_key};

const LABEL: &str = "Opium downloads";

fn attributes() -> HashMap<&'static str, &'static str> {
    HashMap::from([("application", "opium"), ("purpose", "downloads")])
}

/// The key, made and kept the first time; why not, said for the user.
pub async fn key() -> Result<Key, String> {
    kept()
        .await
        .map_err(|e| format!("the keyring that keeps the downloads' key: {e}"))
}

async fn kept() -> Result<Key, Box<oo7::Error>> {
    let keyring = oo7::Keyring::new().await?;
    keyring.unlock().await?;
    if let Some(item) = keyring.search_items(&attributes()).await?.first()
        && let Ok(key) = Key::try_from(item.secret().await?.as_bytes())
    {
        return Ok(key);
    }
    let key = new_key();
    keyring
        .create_item(LABEL, &attributes(), oo7::Secret::blob(key), true)
        .await?;
    Ok(key)
}
