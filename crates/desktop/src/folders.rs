//! Opium's three folders, found as the XDG Base Directory specification
//! says: each from its variable, or under `$HOME` when the variable is
//! unset, empty or not an absolute path (the specification: "If an
//! implementation encounters a relative path in any of these variables it
//! should consider the path invalid and ignore it"). With no usable `HOME`
//! either there is no folder, never one relative to where Opium was started.

use std::ffi::OsString;
use std::path::PathBuf;

/// `$XDG_CONFIG_HOME/opium`, or `$HOME/.config/opium`: the settings, the
/// sign-ins, the paired devices, the shared state, the instance's lock.
/// `None` when neither variable names an absolute folder; the app then
/// refuses to start ([`crate::instance::take`]).
pub fn config() -> Option<PathBuf> {
    opium_folder("XDG_CONFIG_HOME", ".config")
}

/// `$XDG_CACHE_HOME/opium`, or `$HOME/.cache/opium`: what can be fetched or
/// read again (the library as last read, covers, the files' index,
/// `exit.log`). `None` when neither variable names an absolute folder:
/// nothing is cached then.
pub fn cache() -> Option<PathBuf> {
    opium_folder("XDG_CACHE_HOME", ".cache")
}

/// `$XDG_DATA_HOME/opium`, or `$HOME/.local/share/opium`: the downloads.
/// `None` when neither variable names an absolute folder: there are no
/// downloads then.
pub fn data() -> Option<PathBuf> {
    opium_folder("XDG_DATA_HOME", ".local/share")
}

fn opium_folder(variable: &str, under_home: &str) -> Option<PathBuf> {
    let base = base(
        std::env::var_os(variable),
        std::env::var_os("HOME"),
        under_home,
    )?;
    Some(base.join("opium"))
}

/// `variable` when it is an absolute path, else `under_home` in `home` when
/// that is one.
fn base(variable: Option<OsString>, home: Option<OsString>, under_home: &str) -> Option<PathBuf> {
    let absolute = |value: OsString| Some(PathBuf::from(value)).filter(|path| path.is_absolute());
    variable
        .and_then(absolute)
        .or_else(|| Some(home.and_then(absolute)?.join(under_home)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(value: &str) -> Option<OsString> {
        Some(value.into())
    }

    #[test]
    fn an_absolute_variable_is_used() {
        assert_eq!(
            base(some("/x/config"), some("/home/a"), ".config"),
            Some("/x/config".into())
        );
    }

    #[test]
    fn a_variable_unset_empty_or_relative_falls_back_on_home() {
        for variable in [None, some(""), some("config"), some("./config")] {
            assert_eq!(
                base(variable.clone(), some("/home/a"), ".local/share"),
                Some("/home/a/.local/share".into()),
                "{variable:?}"
            );
        }
    }

    #[test]
    fn with_no_usable_home_there_is_no_folder() {
        for home in [None, some(""), some("home/a")] {
            assert_eq!(base(some(""), home.clone(), ".config"), None, "{home:?}");
            assert_eq!(base(None, home.clone(), ".cache"), None, "{home:?}");
        }
    }
}
