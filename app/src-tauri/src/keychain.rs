//! The live talks key's home on the Mac: the login Keychain (live talks
//! lane 1). The game keeps the key in memory only; the app saves a copy here
//! when it is pasted into settings, hands it to the game once at start, and
//! removes it when it is cleared. The key never goes in the game folder, a
//! save, a world file, a log or a screen.
//!
//! The Keychain code is compiled on a Mac only. On other systems every call
//! is a no-op that says so, and the key lives for the session.

/// Where the key is kept: the Keychain item's service and account.
#[cfg(target_os = "macos")]
const SERVICE: &str = "AFM";
#[cfg(target_os = "macos")]
const ACCOUNT: &str = "Anthropic API key (live talks)";

#[cfg(target_os = "macos")]
mod imp {
    use keyring::{Entry, Error};

    fn entry() -> Result<Entry, String> {
        Entry::new(super::SERVICE, super::ACCOUNT).map_err(|e| e.to_string())
    }

    /// The saved key, or none if nothing is saved.
    pub fn get() -> Result<Option<String>, String> {
        match entry()?.get_password() {
            Ok(k) => Ok(Some(k)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(key: &str) -> Result<(), String> {
        entry()?.set_password(key).map_err(|e| e.to_string())
    }

    /// Removes the saved key. Nothing saved is fine.
    pub fn clear() -> Result<(), String> {
        match entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn get() -> Result<Option<String>, String> {
        Ok(None)
    }

    pub fn set(_key: &str) -> Result<(), String> {
        Err("This system has no Keychain. The key is kept until you quit AFM.".into())
    }

    pub fn clear() -> Result<(), String> {
        Ok(())
    }
}

pub use imp::{clear, get, set};
