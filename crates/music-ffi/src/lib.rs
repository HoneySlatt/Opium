//! The Rust core as the Android app sees it, through UniFFI: Kotlin calls
//! these as suspending functions and objects.

use std::sync::Arc;

use apple_music::{AppleMusic, Protected};
use music_core::model::{ItemId, ProviderKind};

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MusicError {
    #[error("not signed in")]
    NotSignedIn,
    // Not `message`: Kotlin's exceptions have one of their own.
    #[error("{reason}")]
    Failed { reason: String },
}

impl From<music_core::Error> for MusicError {
    fn from(e: music_core::Error) -> Self {
        match e {
            music_core::Error::NotSignedIn => Self::NotSignedIn,
            e => Self::Failed {
                reason: e.to_string(),
            },
        }
    }
}

/// Apple Music, signed in with the user's `media-user-token`.
#[derive(uniffi::Object)]
pub struct Apple(Arc<AppleMusic>);

#[uniffi::export(async_runtime = "tokio")]
impl Apple {
    #[uniffi::constructor]
    pub async fn connect(user_token: String) -> Result<Arc<Self>, MusicError> {
        Ok(Arc::new(Self(Arc::new(
            AppleMusic::connect(&user_token).await?,
        ))))
    }

    pub fn storefront(&self) -> String {
        self.0.storefront().to_owned()
    }

    /// A catalog song's encrypted file and its key; `None` for an upload.
    pub async fn protected(&self, song: String) -> Result<Option<Arc<ProtectedTrack>>, MusicError> {
        let id = ItemId::new(ProviderKind::AppleMusic, song);
        Ok(self.0.protected(&id).await?.map(|protected| {
            Arc::new(ProtectedTrack {
                apple: self.0.clone(),
                protected,
            })
        }))
    }
}

/// An encrypted song, for the player's own CDM to license.
#[derive(uniffi::Object)]
pub struct ProtectedTrack {
    apple: Arc<AppleMusic>,
    protected: Protected,
}

#[uniffi::export(async_runtime = "tokio")]
impl ProtectedTrack {
    pub fn url(&self) -> String {
        self.protected.url.clone()
    }

    /// The `pssh` box the file lacks, to add to it.
    pub fn pssh(&self) -> Vec<u8> {
        self.protected.pssh.clone()
    }

    /// The media playlist, with the key as players read Widevine CENC.
    pub fn playlist(&self) -> String {
        self.protected.playlist.clone()
    }

    pub fn kbps(&self) -> u32 {
        self.protected.kbps
    }

    /// The frames the encoder put before the music, to leave out.
    pub fn priming(&self) -> u32 {
        apple_music::PRIMING
    }

    /// The frames after the music in the last frame, to leave out.
    pub fn padding(&self) -> u32 {
        self.protected.padding
    }

    /// The licence, in answer to the CDM's challenge.
    pub async fn license(&self, challenge: Vec<u8>) -> Result<Vec<u8>, MusicError> {
        Ok(self.apple.license(&self.protected, &challenge).await?)
    }
}

/// TLS checks certificates against Android's own store, which needs the
/// app's context: called once, before any request, from `Tls.init`.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_honeyslatt_music_core_Tls_init<'caller>(
    mut env: jni::EnvUnowned<'caller>,
    _class: jni::objects::JClass<'caller>,
    context: jni::objects::JObject<'caller>,
) {
    env.with_env(|env| rustls_platform_verifier::android::init_with_env(env, context))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
