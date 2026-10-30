//! Encrypted fragmented MP4 (CENC), as Apple Music and SoundCloud serve it:
//! where its encrypted samples are ([`fmp4`]), and a stream that reads as the
//! plain MP4 while a CDM decrypts it in the background ([`AudioStream`]).

pub mod fmp4;
mod stream;

pub use fmp4::{
    EncryptedSample, Fmp4Layout, Tenc, audio_specific_config, first_sample_entry, index_fmp4,
    last_sample_duration, one_sample_entry, patch_init, tenc, tencs,
};
pub use stream::{AudioStream, Decrypt};
