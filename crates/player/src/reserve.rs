//! Borrowing a sound card from the sound server, the way JACK does.
//!
//! WirePlumber holds `org.freedesktop.ReserveDevice1.Audio<card>` for every card
//! it uses, at priority −20. Asking the holder to release it with a higher
//! priority makes it close the card (its streams move elsewhere); dropping the
//! reservation hands the card back.
//!
//! Protocol: <http://git.0pointer.net/reserve.git/tree/reserve.txt>

use std::time::Duration;

use zbus::blocking::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use crate::{Error, Result};

/// Above WirePlumber's −20, below JACK's usual 10.
const PRIORITY: i32 = 0;

/// Held for as long as the card is ours; dropping it releases the name.
pub(crate) struct Reservation {
    _connection: Connection,
}

/// The ALSA card number behind `hw:2,0`, `hw:CARD=USB,DEV=0`, `hw:USB`…
pub(crate) fn card_index(device: &str) -> Option<i32> {
    let spec = device.strip_prefix("hw:").or_else(|| device.strip_prefix("plughw:"))?;
    let card = spec.split(',').next()?;
    let card = card.strip_prefix("CARD=").unwrap_or(card);
    if let Ok(index) = card.parse() {
        return Some(index);
    }
    let name = std::ffi::CString::new(card).ok()?;
    alsa::card::Card::from_str(&name).ok().map(|c| c.get_index())
}

/// Try to own `name`. `Ok(false)` while someone else holds it: zbus reports
/// that as an error rather than as the `Exists` reply.
fn acquire(connection: &Connection, name: &str, flags: RequestNameFlags) -> zbus::Result<bool> {
    match connection.request_name_with_flags(name, flags.into()) {
        Ok(reply) => Ok(reply == RequestNameReply::PrimaryOwner),
        Err(zbus::Error::NameTaken) => Ok(false),
        Err(e) => Err(e),
    }
}

pub(crate) fn reserve(card: i32) -> Result<Reservation> {
    let fail = |e: zbus::Error| Error::Output(format!("couldn't reserve card {card}: {e}"));
    let name = format!("org.freedesktop.ReserveDevice1.Audio{card}");
    let connection = Connection::session().map_err(fail)?;

    if acquire(&connection, &name, RequestNameFlags::DoNotQueue).map_err(fail)? {
        return Ok(Reservation {
            _connection: connection,
        });
    }

    let path = format!("/org/freedesktop/ReserveDevice1/Audio{card}");
    let granted: bool = connection
        .call_method(
            Some(name.as_str()),
            path.as_str(),
            Some("org.freedesktop.ReserveDevice1"),
            "RequestRelease",
            &(PRIORITY,),
        )
        .and_then(|reply| reply.body().deserialize())
        .map_err(fail)?;
    if !granted {
        return Err(Error::Output(format!(
            "the program using card {card} refused to release it"
        )));
    }

    // The holder gives the name up once it has closed the card.
    for _ in 0..50 {
        if acquire(&connection, &name, RequestNameFlags::DoNotQueue).map_err(fail)? {
            return Ok(Reservation {
                _connection: connection,
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(Error::Output(format!("card {card} was not released in time")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_numbers_are_read_from_device_names() {
        assert_eq!(card_index("hw:2,0"), Some(2));
        assert_eq!(card_index("hw:CARD=3,DEV=0"), Some(3));
        assert_eq!(card_index("default"), None);
    }
}
