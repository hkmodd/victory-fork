//! NVIDIA's Streamline files for DLSS (`VICTORY_DLSS=1`): where `run` looks
//! for them and, where they are missing, fetching them from NVIDIA's own
//! GitHub after asking. They are NVIDIA's, under NVIDIA's licences, and not
//! part of this program; `victory get-dlss` fetches them by hand.
//! (Port of trustno1 play.rs, GPL-3.0.)

use std::ffi::OsString;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use victory_hle::streamline;

/// `scripts\get-nvidia-sdks.ps1`, run by Windows PowerShell.
const FETCH: &str = include_str!("../../../scripts/get-nvidia-sdks.ps1");
/// Written into the NVIDIA folder when the user said no; `get-dlss` removes it.
const DECLINED: &str = "declined";

/// The first folder with `sl.interposer.dll`: the one named
/// (`VICTORY_STREAMLINE_DIR`), ours (`get-dlss`'s), then for each folder of
/// `beside` its `nvidia-sdk\Streamline\bin\x64` and the folder itself.
fn first_with_interposer(named: Option<OsString>, ours: Option<PathBuf>, beside: &[&Path]) -> Option<PathBuf> {
    let named = named.filter(|v| !v.is_empty()).map(PathBuf::from);
    let beside = beside.iter().flat_map(|b| [streamline::bin_dir(&b.join("nvidia-sdk")), b.to_path_buf()]);
    named.into_iter().chain(ours).chain(beside).find(|d| streamline::has_interposer(d))
}

/// The folder with NVIDIA's files, looked for in `VICTORY_STREAMLINE_DIR`,
/// `%LOCALAPPDATA%\victory\nvidia-sdk\Streamline\bin\x64` and beside the
/// folders given (the program's, the title's).
pub(crate) fn streamline_dir(beside: &[&Path]) -> Option<PathBuf> {
    let ours = streamline::nvidia_home().map(|home| streamline::bin_dir(&home));
    first_with_interposer(std::env::var_os("VICTORY_STREAMLINE_DIR"), ours, beside)
}

fn nvidia_home() -> Result<PathBuf> {
    streamline::nvidia_home().context("no %LOCALAPPDATA% to keep NVIDIA's files in")
}

/// Runs the fetcher: NVIDIA's files from NVIDIA's GitHub into `to`.
fn fetch(to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
    let script = to.join("get-nvidia-sdks.ps1");
    std::fs::write(&script, FETCH).with_context(|| format!("writing {}", script.display()))?;
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg("-To")
        .arg(to)
        .status()
        .context("starting Windows PowerShell");
    let _ = std::fs::remove_file(&script);
    anyhow::ensure!(
        status?.success() && streamline::has_interposer(&streamline::bin_dir(to)),
        "NVIDIA's files did not arrive (see above); DLSS stays off"
    );
    Ok(())
}

fn get_dlss_in(to: &Path, fetch: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    let _ = std::fs::remove_file(to.join(DECLINED));
    fetch(to)?;
    println!("NVIDIA's files are in {}", streamline::bin_dir(to).display());
    Ok(())
}

/// `victory get-dlss`: fetches them whether or not the user declined before.
pub(crate) fn get_dlss() -> Result<()> {
    get_dlss_in(&nvidia_home()?, fetch)
}

/// What became of the offer.
#[derive(Debug, PartialEq, Eq)]
enum Offer {
    /// Declined at an earlier start (the marker is there).
    Declined,
    /// Nobody to ask: input or output is not a terminal (a scripted run).
    NotAsked,
    /// The user said no now (the marker is written).
    Refused,
    Fetched,
    Failed,
}

fn offer_in(
    to: &Path,
    interactive: bool,
    ask: impl FnOnce() -> Option<String>,
    fetch: impl FnOnce(&Path) -> Result<()>,
) -> Offer {
    let declined = to.join(DECLINED);
    if declined.exists() || !interactive {
        println!("upscaler       no NVIDIA Streamline files here: DLSS stays off (`victory get-dlss` fetches them)");
        return if interactive { Offer::Declined } else { Offer::NotAsked };
    }
    println!("NVIDIA's DLSS files (Streamline 2.14.16) are not on this PC yet. They are NVIDIA's own, under");
    println!("NVIDIA's licences, and come from github.com/NVIDIA-RTX/Streamline (about 70 MB, once), into");
    println!("{}.", to.display());
    print!("Download them now? [Y/n] ");
    let _ = std::io::stdout().flush();
    let Some(answer) = ask() else {
        println!();
        println!("No answer: DLSS stays off (`victory get-dlss` fetches them).");
        return Offer::NotAsked;
    };
    if answer.trim().to_ascii_lowercase().starts_with('n') {
        let _ = std::fs::create_dir_all(to);
        let _ = std::fs::write(&declined, "asked once; `victory get-dlss` fetches them\r\n");
        println!("Not downloaded. `victory get-dlss` does it later; DLSS stays off.");
        return Offer::Refused;
    }
    match fetch(to) {
        Ok(()) => Offer::Fetched,
        Err(e) => {
            println!("{e:#}");
            Offer::Failed
        }
    }
}

/// At a start with `VICTORY_DLSS=1` and no NVIDIA files: asks once whether
/// to fetch them (only with a terminal for input and output). The folder of
/// `sl.interposer.dll` once they are there.
pub(crate) fn offer_dlss() -> Option<PathBuf> {
    let to = match nvidia_home() {
        Ok(to) => to,
        Err(e) => {
            println!("upscaler       {e:#}; DLSS stays off");
            return None;
        }
    };
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let ask = || {
        let mut answer = String::new();
        matches!(std::io::stdin().read_line(&mut answer), Ok(n) if n > 0).then_some(answer)
    };
    (offer_in(&to, interactive, ask, fetch) == Offer::Fetched).then(|| streamline::bin_dir(&to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_interposer(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("sl.interposer.dll"), b"").unwrap();
        dir.to_path_buf()
    }

    #[test]
    fn folders_are_tried_in_order() {
        let t = tempfile::tempdir().unwrap();
        let named = t.path().join("named");
        let ours = t.path().join("ours");
        let program = t.path().join("program");
        let title = t.path().join("title");
        for d in [&named, &ours, &program, &title] {
            std::fs::create_dir_all(d).unwrap();
        }
        let beside = [program.as_path(), title.as_path()];
        let find = || first_with_interposer(Some(named.clone().into()), Some(ours.clone()), &beside);
        assert_eq!(find(), None);
        let title_sdk = with_interposer(&streamline::bin_dir(&title.join("nvidia-sdk")));
        assert_eq!(find(), Some(title_sdk));
        let program_own = with_interposer(&program);
        assert_eq!(find(), Some(program_own));
        let program_sdk = with_interposer(&streamline::bin_dir(&program.join("nvidia-sdk")));
        assert_eq!(find(), Some(program_sdk));
        with_interposer(&ours);
        assert_eq!(find(), Some(ours.clone()));
        with_interposer(&named);
        assert_eq!(find(), Some(named.clone()));
        // An empty VICTORY_STREAMLINE_DIR is unset.
        assert_eq!(first_with_interposer(Some(OsString::new()), Some(ours.clone()), &beside), Some(ours));
    }

    #[test]
    fn a_no_is_remembered_and_get_dlss_forgets_it() {
        let t = tempfile::tempdir().unwrap();
        let to = t.path().join("nvidia-sdk");
        let never = |_: &Path| -> Result<()> { panic!("fetched") };
        assert_eq!(offer_in(&to, false, || panic!("asked"), never), Offer::NotAsked);
        assert_eq!(offer_in(&to, true, || None, never), Offer::NotAsked);
        assert!(!to.join(DECLINED).exists());
        assert_eq!(offer_in(&to, true, || Some("n\r\n".into()), never), Offer::Refused);
        assert!(to.join(DECLINED).is_file());
        assert_eq!(offer_in(&to, true, || panic!("asked"), never), Offer::Declined);
        let mut fetched = false;
        get_dlss_in(&to, |dir| {
            assert!(!dir.join(DECLINED).exists());
            fetched = true;
            Ok(())
        })
        .unwrap();
        assert!(fetched);
    }

    #[test]
    fn enter_fetches() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(offer_in(t.path(), true, || Some("\r\n".into()), |_| Ok(())), Offer::Fetched);
        assert_eq!(offer_in(t.path(), true, || Some("Y\n".into()), |_| anyhow::bail!("offline")), Offer::Failed);
        assert!(!t.path().join(DECLINED).exists());
    }
}
