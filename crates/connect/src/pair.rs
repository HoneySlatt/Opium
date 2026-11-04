//! Pairing: one device shows a code, the other's user types it in, and
//! the two come out with a key of their own. SPAKE2 makes the code good
//! for one guess only: what passes over the network tells an onlooker
//! nothing to try codes against.

use hmac::{KeyInit, Mac};
use serde::{Deserialize, Serialize};
use spake2::{Ed25519Group, Identity, Password, Spake2};

/// A step of pairing, after the greetings.
#[derive(Serialize, Deserialize)]
pub enum Step {
    /// Each side's half of the exchange.
    Spake(Vec<u8>),
    /// Proof of the key both should now have.
    Confirm(Vec<u8>),
    /// The code typed in wasn't the one shown, or pairing was called off.
    Refused,
}

/// Six digits, as a phone's keypad types them.
pub fn code() -> String {
    let n = u32::from_be_bytes(crate::store::random::<4>()) % 1_000_000;
    format!("{n:06}")
}

/// Start the exchange: `asker` is the device the code is typed into,
/// `shower` the one showing it.
pub fn start(
    code: &str,
    asker: &str,
    shower: &str,
    as_asker: bool,
) -> (Spake2<Ed25519Group>, Vec<u8>) {
    let (password, a, b) = (
        Password::new(code.as_bytes()),
        Identity::new(asker.as_bytes()),
        Identity::new(shower.as_bytes()),
    );
    if as_asker {
        Spake2::<Ed25519Group>::start_a(&password, &a, &b)
    } else {
        Spake2::<Ed25519Group>::start_b(&password, &a, &b)
    }
}

fn mac(key: &[u8], label: &[u8]) -> Vec<u8> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(key).expect("any key length");
    mac.update(label);
    mac.finalize().into_bytes().to_vec()
}

/// What `side` sends to show it has the key.
pub fn proof(key: &[u8], side: &str) -> Vec<u8> {
    mac(key, format!("opium-connect confirm {side}").as_bytes())
}

pub fn proves(key: &[u8], side: &str, proof: &[u8]) -> bool {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(key).expect("any key length");
    mac.update(format!("opium-connect confirm {side}").as_bytes());
    mac.verify_slice(proof).is_ok()
}

/// The key the two keep, apart from the one they confirmed with.
pub fn kept(key: &[u8]) -> [u8; 32] {
    mac(key, b"opium-connect psk")
        .try_into()
        .expect("SHA-256 is 32 bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange(typed: &str, shown: &str) -> (Vec<u8>, Vec<u8>) {
        let (a, ma) = start(typed, "phone", "desktop", true);
        let (b, mb) = start(shown, "phone", "desktop", false);
        (a.finish(&mb).unwrap(), b.finish(&ma).unwrap())
    }

    #[test]
    fn the_right_code_gives_both_the_same_key() {
        let (a, b) = exchange("123456", "123456");
        assert_eq!(a, b);
        assert!(proves(&b, "asker", &proof(&a, "asker")));
        assert_eq!(kept(&a), kept(&b));
    }

    #[test]
    fn a_wrong_code_proves_nothing() {
        let (a, b) = exchange("123456", "123457");
        assert!(!proves(&b, "asker", &proof(&a, "asker")));
    }

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..100 {
            let c = code();
            assert!(c.len() == 6 && c.bytes().all(|b| b.is_ascii_digit()));
        }
    }
}
