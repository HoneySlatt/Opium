//! Load the system CDM and ask it for a licence challenge, without any server.
//!
//!   cargo run -p widevine --example probe

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = widevine::locate().ok_or("no Widevine CDM found")?;
    println!("CDM: {}", path.display());
    let cdm = widevine::Cdm::open(&path)?;
    let (_session, challenge) = cdm.start_session(&widevine::build_pssh(&[0x42; 16]))?;
    println!("licence challenge: {} bytes", challenge.len());
    Ok(())
}
