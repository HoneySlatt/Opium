//! Writing a file's tags without putting the file at risk: the change is made
//! on a copy, and the file is replaced by the copy only once it is whole.
//!
//! On the desktop, the copy sits beside the file and is renamed over it,
//! which the file system does in one step: the file is either as it was or
//! as edited, whatever happens. On the phone, the file is a descriptor that
//! Android hands over and that cannot be renamed. There, the original is
//! copied aside first and the edited copy is written over the file, with a
//! note kept meanwhile; a write cut short (the app killed) leaves the file
//! half written until [`undo_tag_write`] puts it back, when the app starts
//! again or before that file is next written.
//!
//! One write runs at a time in the process, so that two never mix their
//! bytes in one file, nor sweep each other's copies.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::tags::{TagEdit, edit_tags};

/// Held by each write, and by the sweep of copies left behind.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    // A write that panicked leaves no state behind the lock to distrust.
    ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The start of a copy's name beside the file: short, so that any name the
/// file has still fits; hidden, and not named as music, so the library never
/// reads it. Each write adds its own id, so that two Opiums (two users
/// sharing a folder) never touch each other's copy.
const ASIDE: &str = ".opium-tags-";

/// Write `edit` into the file at `path`, in its own kind of tag: into a copy
/// beside it, renamed over it once written and on disk. A link is followed,
/// so the file it names is replaced, not the link. Errors leave the file as
/// it was: the folder cannot take a new file, the disk is full, the file is
/// not one whose tags can be written.
///
/// The copy is a new file with the old one's permissions: what else the old
/// one carried is not kept (its extended attributes, ACLs and owner), and a
/// second hard link to it keeps the old tags. A folder the user can't write
/// refuses the copy, so its files' tags are not written, even where the
/// files themselves could be.
pub fn write_tags(path: &Path, edit: &TagEdit) -> Result<(), String> {
    let _one = one_at_a_time();
    let real = fs::canonicalize(path).map_err(|e| e.to_string())?;
    let Some(dir) = real.parent() else {
        return Err("this isn't a file".into());
    };
    let aside = dir.join(format!("{ASIDE}{}", new_id()));
    let written = (|| -> io::Result<()> {
        // A file the user can't write stays refused, as when it was written
        // in place: the rename alone would need only the folder's leave.
        drop(OpenOptions::new().write(true).open(&real)?);
        let mut original = File::open(&real)?;
        let permissions = original.metadata()?.permissions();
        // A new file, never one that a link put there would lead elsewhere,
        // and from the start no more open to others than the original.
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(
            &mut options,
            std::os::unix::fs::PermissionsExt::mode(&permissions),
        );
        let mut copy = options.open(&aside)?;
        io::copy(&mut original, &mut copy)?;
        copy.set_permissions(permissions)?;
        copy.rewind()?;
        edit_tags(&mut copy, edit).map_err(io::Error::other)?;
        copy.sync_all()?;
        fs::rename(&aside, &real)
    })();
    match written {
        Ok(()) => {
            // The rename itself on disk; the file is whole either way.
            let _ = File::open(dir).and_then(|dir| dir.sync_all());
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&aside);
            Err(e.to_string())
        }
    }
}

/// The first line of a note: what kind of file it is, and its version.
const NOTE: &str = "opium-tag-write 1";

/// Write `edit` into `open`, a file open to read and write that cannot be
/// renamed (the phone's), keeping its copies in `work`, a folder of the
/// app's own that the system never clears. `key` names the file for
/// [`unfinished_tag_writes`], as the caller can open it again (its address).
///
/// The original is copied into `work` and edited there; only then is a note
/// written and the edited copy written over `open`. Errors before that leave
/// the file untouched. An error while replacing it puts the original back;
/// if that fails too, the note stays, and the error says the file will be
/// put back. A file whose earlier write is still to be put back is refused.
pub fn write_tags_over(
    open: &mut File,
    edit: &TagEdit,
    work: &Path,
    key: &str,
) -> Result<(), String> {
    let _one = one_at_a_time();
    if notes(work).iter().any(|(_, noted)| noted == key) {
        return Err(
            "an earlier change to this file was cut short and is to be undone first".into(),
        );
    }
    fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let copies = Copies::of(work, &new_id());
    let prepared = (|| -> io::Result<()> {
        copy_out(open, &copies.original)?;
        fs::copy(&copies.original, &copies.edited)?;
        let mut edited = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&copies.edited)?;
        edit_tags(&mut edited, edit).map_err(io::Error::other)?;
        edited.sync_all()?;
        write_note(&copies, key)
    })();
    if let Err(e) = prepared {
        copies.remove();
        return Err(e.to_string());
    }
    match replace(open, &copies.edited) {
        Ok(()) => {
            copies.remove();
            Ok(())
        }
        Err(e) => match replace(open, &copies.original) {
            Ok(()) => {
                copies.remove();
                Err(format!("{e}; the file was put back as it was"))
            }
            Err(_) => Err(format!(
                "{e}; the file will be put back as it was once Opium can open it again"
            )),
        },
    }
}

/// A tag write cut short while it replaced a file: [`undo_tag_write`] puts
/// the file back as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unfinished {
    /// Which write it was, for [`undo_tag_write`].
    pub id: String,
    /// The `key` the write was given: the file's address.
    pub key: String,
}

/// The writes in `work` that still have their note: cut short while they
/// replaced a file, or finished, or stopped before touching it, with the
/// note left behind; [`undo_tag_write`] tells them apart and clears them.
/// Copies without a note are removed. A note of another version, or
/// unreadable, is left with its copies, untouched.
pub fn unfinished_tag_writes(work: &Path) -> Vec<Unfinished> {
    let _one = one_at_a_time();
    let noted = notes(work);
    let Ok(entries) = fs::read_dir(work) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Some((id, kind)) = name.split_once('.') else {
            continue;
        };
        let has_note = work.join(format!("{id}.note")).exists();
        if is_id(id) && !has_note && matches!(kind, "original" | "edited" | "note.part") {
            let _ = fs::remove_file(entry.path());
        }
    }
    let mut found: Vec<Unfinished> = noted
        .into_iter()
        .filter_map(|(id, key)| {
            let copies = Copies::of(work, &id);
            if copies.original.exists() {
                Some(Unfinished { id, key })
            } else {
                // The original gone: nothing to put back.
                copies.remove();
                None
            }
        })
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// Put the file of the unfinished write `id` back as it was, from the
/// original kept in `work`, then remove the write's copies. The file is
/// first compared with the copies. One already as edited (the write had
/// finished) or as it was is left; one mixing the two, as a write or an
/// undo cut short leaves it, is replaced: past the shorter copy's end, it
/// may hold zeros where a power cut kept the file's new length but not the
/// bytes written there. One that is neither (changed since by another app,
/// or damaged some other way) is left too, and its original kept in `work`
/// as `<id>.kept`, never removed by Opium. On an error the copies are kept,
/// for the next try.
pub fn undo_tag_write(open: &mut File, work: &Path, id: &str) -> Result<(), String> {
    let _one = one_at_a_time();
    if !is_id(id) {
        return Err("no such write".into());
    }
    let copies = Copies::of(work, id);
    if !copies.note.exists() {
        return Err("no such write".into());
    }
    match state_of(open, &copies).map_err(|e| e.to_string())? {
        State::Original | State::Edited => {}
        State::Mixed => replace(open, &copies.original).map_err(|e| e.to_string())?,
        State::Unknown => {
            let kept = work.join(format!("{id}.kept"));
            fs::rename(&copies.original, &kept).map_err(|e| e.to_string())?;
        }
    }
    copies.remove();
    Ok(())
}

/// The files of one write in `work`.
struct Copies {
    note: PathBuf,
    original: PathBuf,
    edited: PathBuf,
}

impl Copies {
    fn of(work: &Path, id: &str) -> Self {
        Self {
            note: work.join(format!("{id}.note")),
            original: work.join(format!("{id}.original")),
            edited: work.join(format!("{id}.edited")),
        }
    }

    /// The note first, on disk: once it is gone the write is never undone,
    /// and copies left behind are swept at the next start.
    fn remove(&self) {
        let _ = fs::remove_file(&self.note);
        if let Some(work) = self.note.parent() {
            let _ = File::open(work).and_then(|work| work.sync_all());
        }
        let _ = fs::remove_file(self.note.with_extension("note.part"));
        let _ = fs::remove_file(&self.original);
        let _ = fs::remove_file(&self.edited);
    }
}

/// The notes in `work` this version reads: each write's id and key.
fn notes(work: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(work) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let id = name.strip_suffix(".note").filter(|id| is_id(id))?;
            let text = fs::read_to_string(entry.path()).ok()?;
            let (NOTE, rest) = text.split_once('\n')? else {
                return None;
            };
            Some((id.to_owned(), rest.trim_end_matches('\n').to_owned()))
        })
        .collect()
}

/// A new write's id, unique in the folder: the time, the process, a count.
fn new_id() -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let count = COUNT.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{:x}-{count:x}", std::process::id())
}

/// Whether `id` is one [`new_id`] makes: no path can be built from another.
fn is_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// `open`'s whole content copied into a new file at `to`, on disk.
fn copy_out(open: &mut File, to: &Path) -> io::Result<()> {
    open.rewind()?;
    let mut copy = File::create(to)?;
    io::copy(open, &mut copy)?;
    copy.sync_all()
}

/// `open`'s content replaced by the file at `from`, on disk.
fn replace(open: &mut File, from: &Path) -> io::Result<()> {
    open.rewind()?;
    let length = io::copy(&mut File::open(from)?, open)?;
    open.set_len(length)?;
    open.sync_all()
}

/// What a file under an unfinished write is, against its two copies.
#[derive(Debug, PartialEq, Eq)]
enum State {
    /// The original whole: the write never reached it, or was undone.
    Original,
    /// The edited copy whole: the write had finished.
    Edited,
    /// Each byte the edited copy's or the original's at its place, the
    /// length between theirs: as a write or an undo cut short leaves it,
    /// even with its blocks reaching the disk out of order. Past the shorter
    /// copy's end, a zero counts as the longer one's not yet on disk, as a
    /// power cut leaves it.
    Mixed,
    /// Neither: changed since some other way.
    Unknown,
}

/// `open`'s state against `copies`, read through once.
fn state_of(open: &mut File, copies: &Copies) -> io::Result<State> {
    open.rewind()?;
    let edited_length = fs::metadata(&copies.edited)?.len();
    let original_length = fs::metadata(&copies.original)?.len();
    let mut edited = BufReader::new(File::open(&copies.edited)?).bytes();
    let mut original = BufReader::new(File::open(&copies.original)?).bytes();
    let (mut as_edited, mut as_original) = (true, true);
    let mut length: u64 = 0;
    for byte in BufReader::new(&mut *open).bytes() {
        let byte = Some(byte?);
        let from_edited = edited.next().transpose()?;
        let from_original = original.next().transpose()?;
        as_edited &= byte == from_edited;
        as_original &= byte == from_original;
        // Past the shorter copy's end the file grew as the longer one was
        // written over it; a power cut can keep the new length but not the
        // blocks yet to reach the disk, which read as zeros.
        let one_copy_ended = from_edited.is_none() != from_original.is_none();
        let not_yet_on_disk = byte == Some(0) && one_copy_ended;
        if byte != from_edited && byte != from_original && !not_yet_on_disk {
            return Ok(State::Unknown);
        }
        length += 1;
    }
    Ok(match length {
        _ if as_edited && length == edited_length => State::Edited,
        _ if as_original && length == original_length => State::Original,
        _ if (edited_length.min(original_length)..=edited_length.max(original_length))
            .contains(&length) =>
        {
            State::Mixed
        }
        _ => State::Unknown,
    })
}

/// The note that a write is replacing its file, on disk before it starts:
/// written aside, then renamed.
fn write_note(copies: &Copies, key: &str) -> io::Result<()> {
    let part = copies.note.with_extension("note.part");
    {
        let mut note = File::create(&part)?;
        note.write_all(format!("{NOTE}\n{key}\n").as_bytes())?;
        note.sync_all()?;
    }
    fs::rename(&part, &copies.note)?;
    if let Some(work) = copies.note.parent() {
        File::open(work)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_tags;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("local-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn titled(title: &str) -> TagEdit {
        TagEdit {
            title: Some(title.into()),
            ..TagEdit::default()
        }
    }

    fn open_rw(path: &Path) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap()
    }

    fn wav() -> Vec<u8> {
        crate::scan::tests::wav()
    }

    /// A write of `title` over `path` as a killed app leaves it: its copies
    /// and note kept, and the file `written` bytes into the replace.
    fn cut_short(path: &Path, work: &Path, title: &str, written: usize) -> Copies {
        fs::create_dir_all(work).unwrap();
        let copies = Copies::of(work, &new_id());
        let original = fs::read(path).unwrap();
        fs::write(&copies.original, &original).unwrap();
        fs::write(&copies.edited, &original).unwrap();
        edit_tags(&mut open_rw(&copies.edited), &titled(title)).unwrap();
        write_note(&copies, "content://media/7").unwrap();
        let edited = fs::read(&copies.edited).unwrap();
        fs::write(path, stopped(&original, &edited, written)).unwrap();
        copies
    }

    /// What a replace of `from` over `under` leaves, stopped `written`
    /// bytes in, before its length was set.
    fn stopped(under: &[u8], from: &[u8], written: usize) -> Vec<u8> {
        let mut file = from[..written].to_vec();
        if written < under.len() {
            file.extend_from_slice(&under[written..]);
        }
        file
    }

    fn id_of(copies: &Copies) -> String {
        copies
            .note
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn the_file_is_replaced_and_never_rewritten_where_it_is() {
        let dir = folder("tags-replaced");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        // Held open, as the player holds a song it plays: the file it has
        // is never written to, so it reads the old one whole to its end.
        let mut held = File::open(&path).unwrap();
        write_tags(&path, &titled("Replaced")).unwrap();
        let mut read = Vec::new();
        held.read_to_end(&mut read).unwrap();
        assert_eq!(read, wav());
        assert_eq!(read_tags(&path).unwrap().title, "Replaced");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_failed_write_leaves_the_file_as_it_was_and_nothing_beside_it() {
        let dir = folder("tags-failed");
        let path = dir.join("song.mp3");
        // Not an MP3 its tags can be written into.
        fs::write(&path, b"not an mp3").unwrap();
        assert!(write_tags(&path, &titled("New")).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not an mp3");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "no copy left");
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_write_replaces_the_file_whole_through_a_link() {
        let dir = folder("tags-link");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        let link = dir.join("link.wav");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        write_tags(&link, &titled("Through")).unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(read_tags(&path).unwrap().title, "Through");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2, "no copy left");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn another_writes_copy_beside_the_file_is_left_alone() {
        let dir = folder("tags-leftover");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        // Another Opium's, being written, or one left by a crash.
        let other = dir.join(format!("{ASIDE}1-2-3"));
        fs::write(&other, b"another write's copy").unwrap();
        write_tags(&path, &titled("Next")).unwrap();
        assert_eq!(read_tags(&path).unwrap().title, "Next");
        assert_eq!(fs::read(&other).unwrap(), b"another write's copy");
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            2,
            "no copy of its own left"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_with_the_longest_name_is_written() {
        let dir = folder("tags-long");
        let path = dir.join(format!("{}.wav", "a".repeat(251)));
        fs::write(&path, wav()).unwrap();
        write_tags(&path, &titled("Long")).unwrap();
        assert_eq!(read_tags(&path).unwrap().title, "Long");
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_file_the_user_cant_write_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = folder("tags-read-only");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        assert!(write_tags(&path, &titled("New")).is_err());
        assert_eq!(fs::read(&path).unwrap(), wav());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "no copy left");
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_write_keeps_the_files_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = folder("tags-mode");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        write_tags(&path, &titled("Kept")).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writes_at_the_same_time_to_one_file_never_mix() {
        let dir = folder("tags-together");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        let work = dir.join("work");
        let writers: Vec<_> = (0..8)
            .map(|n| {
                let (path, work) = (path.clone(), work.clone());
                // Titles of lengths far apart: bytes of two writes mixed in the
                // file would read as neither, or not at all.
                let title = format!("Writer {n} {}", "x".repeat(1 + n * 300));
                std::thread::spawn(move || match n % 2 {
                    0 => write_tags(&path, &titled(&title)).map(|()| title),
                    _ => write_tags_over(&mut open_rw(&path), &titled(&title), &work, "k")
                        .map(|()| title),
                })
            })
            .collect();
        let titles: Vec<String> = writers
            .into_iter()
            .map(|writer| writer.join().unwrap().unwrap())
            .collect();
        let title = read_tags(&path).expect("a file whose tags read").title;
        assert!(titles.contains(&title), "{title}");
        assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn every_write_and_sweep_waits_for_the_one_running() {
        let dir = folder("tags-waits");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        let work = dir.join("work");
        let running = one_at_a_time();
        let (done, finished) = std::sync::mpsc::channel();
        let waiting: Vec<_> = (0..3)
            .map(|n| {
                let (path, work, done) = (path.clone(), work.clone(), done.clone());
                std::thread::spawn(move || {
                    match n {
                        0 => write_tags(&path, &titled("Desk")).unwrap(),
                        1 => write_tags_over(&mut open_rw(&path), &titled("Phone"), &work, "k")
                            .unwrap(),
                        _ => drop(unfinished_tag_writes(&work)),
                    }
                    done.send(n).unwrap();
                })
            })
            .collect();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(
            finished.try_recv().is_err(),
            "none ran beside the one running"
        );
        drop(running);
        for thread in waiting {
            thread.join().unwrap();
        }
        assert_eq!(finished.try_iter().count(), 3);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_write_over_an_open_file_leaves_no_copies() {
        let dir = folder("tags-over");
        let work = dir.join("work");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        write_tags_over(
            &mut open_rw(&path),
            &titled("Over"),
            &work,
            "content://song",
        )
        .unwrap();
        assert_eq!(read_tags(&path).unwrap().title, "Over");
        assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
        assert!(unfinished_tag_writes(&work).is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_failed_write_over_an_open_file_leaves_it_untouched() {
        let dir = folder("tags-over-failed");
        let work = dir.join("work");
        let path = dir.join("song.mp3");
        fs::write(&path, b"not an mp3").unwrap();
        assert!(write_tags_over(&mut open_rw(&path), &titled("New"), &work, "k").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not an mp3");
        assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_replace_that_fails_twice_is_left_to_undo_and_blocks_the_next_write() {
        let dir = folder("tags-over-replace");
        let work = dir.join("work");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        // Readable, but refusing writes: the replace fails after the copies
        // and the note are made, and so does putting the original back.
        let mut read_only = File::open(&path).unwrap();
        let error =
            write_tags_over(&mut read_only, &titled("New"), &work, "content://a").unwrap_err();
        assert!(error.contains("once Opium can open it again"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), wav(), "never written");
        let again = write_tags_over(&mut open_rw(&path), &titled("New"), &work, "content://a");
        assert!(again.unwrap_err().contains("undone first"));
        let unfinished = unfinished_tag_writes(&work);
        assert_eq!(unfinished.len(), 1);
        assert_eq!(unfinished[0].key, "content://a");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_write_cut_short_is_undone_wherever_it_stopped() {
        let dir = folder("tags-undo");
        let path = dir.join("song.wav");
        let length = wav().len();
        for written in [1, 40, length / 2, length - 1] {
            let work = dir.join(format!("work-{written}"));
            fs::write(&path, wav()).unwrap();
            let copies = cut_short(&path, &work, "Cut", written);
            assert_eq!(
                unfinished_tag_writes(&work),
                vec![Unfinished {
                    id: id_of(&copies),
                    key: "content://media/7".into()
                }]
            );
            undo_tag_write(&mut open_rw(&path), &work, &id_of(&copies)).unwrap();
            assert_eq!(fs::read(&path).unwrap(), wav(), "put back, {written}");
            assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_write_cut_short_is_undone_when_the_tag_shifts_the_audio() {
        // An MP3's tag comes first: the edited copy's audio sits further on.
        let dir = folder("tags-undo-mp3");
        let path = dir.join("song.mp3");
        let original = crate::scan::tests::mp3();
        fs::write(&path, &original).unwrap();
        let work = dir.join("probe");
        let edited_length = fs::read(cut_short(&path, &work, "Cut", 0).edited)
            .unwrap()
            .len();
        assert!(edited_length > original.len());
        for written in [
            1,
            100,
            original.len() - 1,
            original.len() + 1,
            edited_length - 1,
        ] {
            let work = dir.join(format!("work-{written}"));
            fs::write(&path, &original).unwrap();
            let copies = cut_short(&path, &work, "Cut", written);
            undo_tag_write(&mut open_rw(&path), &work, &id_of(&copies)).unwrap();
            assert_eq!(fs::read(&path).unwrap(), original, "put back, {written}");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_undo_cut_short_is_undone_at_the_next_try() {
        let dir = folder("tags-undo-twice");
        let path = dir.join("song.mp3");
        let original = crate::scan::tests::mp3();
        fs::write(&path, &original).unwrap();
        let work = dir.join("work");
        let copies = cut_short(&path, &work, "Cut", 2000);
        // The undo stopped too, 50 bytes in: the original's, the edited
        // copy's, then the original's again.
        let half = fs::read(&path).unwrap();
        fs::write(&path, stopped(&half, &original, 50)).unwrap();
        undo_tag_write(&mut open_rw(&path), &work, &id_of(&copies)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_write_that_had_finished_is_never_undone() {
        let dir = folder("tags-finished");
        let work = dir.join("work");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        // Its note back after a power cut: the file is already as edited.
        let copies = cut_short(&path, &work, "Done", 0);
        let edited = fs::read(&copies.edited).unwrap();
        fs::write(&path, &edited).unwrap();
        undo_tag_write(&mut open_rw(&path), &work, &id_of(&copies)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), edited);
        assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_changed_since_is_never_undone() {
        let dir = folder("tags-changed");
        let work = dir.join("work");
        let path = dir.join("song.wav");
        fs::write(&path, wav()).unwrap();
        let copies = cut_short(&path, &work, "Cut", 10);
        // Written whole since, by another app.
        let mut other = wav();
        other.extend_from_slice(b"another app's");
        fs::write(&path, &other).unwrap();
        let id = id_of(&copies);
        undo_tag_write(&mut open_rw(&path), &work, &id).unwrap();
        assert_eq!(fs::read(&path).unwrap(), other);
        // Its original kept all the same, and never swept.
        assert!(unfinished_tag_writes(&work).is_empty());
        assert_eq!(fs::read(work.join(format!("{id}.kept"))).unwrap(), wav());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn copies_without_a_note_are_swept() {
        let dir = folder("tags-sweep");
        let work = dir.join("work");
        fs::create_dir_all(&work).unwrap();
        for name in ["1-2-0.original", "1-2-0.edited", "3-2-1.note.part"] {
            fs::write(work.join(name), b"x").unwrap();
        }
        fs::write(work.join("someone else's"), b"x").unwrap();
        assert!(unfinished_tag_writes(&work).is_empty());
        let left: Vec<_> = fs::read_dir(&work)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["someone else's"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_note_of_another_version_is_left_alone() {
        let dir = folder("tags-version");
        let work = dir.join("work");
        fs::create_dir_all(&work).unwrap();
        let copies = Copies::of(&work, "3c-2-0");
        fs::write(&copies.original, b"audio").unwrap();
        fs::write(&copies.note, "opium-tag-write 2\ncontent://x\n").unwrap();
        assert!(unfinished_tag_writes(&work).is_empty());
        assert!(copies.original.exists() && copies.note.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_id_that_names_another_path_is_refused() {
        let dir = folder("tags-id");
        let path = dir.join("song.wav");
        fs::write(&path, b"x").unwrap();
        assert!(undo_tag_write(&mut open_rw(&path), &dir, "../song").is_err());
        assert!(undo_tag_write(&mut open_rw(&path), &dir, "").is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    // Stage 1 audit, L10: a power cut's zeros past the old end left the song half written.

    /// Ten frames of MPEG-1 Layer III, as `scan::tests::mp3`, but with audio that is not silence.
    fn loud_mp3() -> Vec<u8> {
        let mut bytes = Vec::new();
        for frame in 0..10u32 {
            bytes.extend([0xFF, 0xFB, 0x90, 0x00]);
            bytes.extend((0..417u32 - 4).map(|i| 1 + ((i + frame) % 200) as u8));
        }
        bytes
    }

    #[test]
    fn a_write_a_power_cut_left_with_zeros_past_the_old_end_is_put_back() {
        let dir = folder("tags-power-cut");
        let path = dir.join("song.mp3");
        let original = loud_mp3();
        fs::write(&path, &original).unwrap();
        let work = dir.join("work");
        // The phone's replace stopped halfway: the edited copy's first half, the original's rest.
        let copies = cut_short(&path, &work, "Cut", original.len() / 2);
        let edited = fs::read(&copies.edited).unwrap();
        // The ID3v2 tag goes first: the edited copy is longer and its audio sits further on.
        assert!(edited.len() > original.len());
        // The power cut: the file's new length reached the disk, the blocks past its old end did
        // not, and read back as zeros.
        let mut on_disk = fs::read(&path).unwrap();
        on_disk.resize(edited.len(), 0);
        fs::write(&path, &on_disk).unwrap();
        let id = id_of(&copies);
        undo_tag_write(&mut open_rw(&path), &work, &id).unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            original,
            "the song is left half written; its original is only in {id}.kept"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    /// Zeros count as a power cut's only past the shorter copy's end: one
    /// before it, where neither copy has one, is a change made elsewhere.
    #[test]
    fn a_zero_before_the_shorter_copys_end_is_never_taken_for_a_power_cuts() {
        let dir = folder("tags-zero-inside");
        let path = dir.join("song.mp3");
        let original = loud_mp3();
        fs::write(&path, &original).unwrap();
        let work = dir.join("work");
        let copies = cut_short(&path, &work, "Cut", original.len() / 2);
        let edited = fs::read(&copies.edited).unwrap();
        let mut changed = fs::read(&path).unwrap();
        let inside = (original.len() / 2..original.len())
            .find(|&at| original[at] != 0 && edited[at] != 0)
            .unwrap();
        changed[inside] = 0;
        changed.resize(edited.len(), 0);
        fs::write(&path, &changed).unwrap();
        let id = id_of(&copies);
        undo_tag_write(&mut open_rw(&path), &work, &id).unwrap();
        assert_eq!(fs::read(&path).unwrap(), changed, "left as it is");
        assert_eq!(fs::read(work.join(format!("{id}.kept"))).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
    }
}
