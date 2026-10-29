//! Encrypted fragmented MP4 (CENC), as Apple Music and SoundCloud serve it:
//! where its encrypted samples are ([`fmp4`]), and a stream that reads as the
//! plain MP4 while a CDM decrypts it in the background ([`AudioStream`]).

pub mod fmp4;
mod stream;

pub use fmp4::{
    EncryptedSample, Fmp4Layout, Tenc, audio_specific_config, index_fmp4, patch_init, tenc,
};
pub use stream::{AudioStream, Decrypt};
