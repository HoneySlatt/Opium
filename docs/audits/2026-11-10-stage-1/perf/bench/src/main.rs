//! M1.8 performance audit: the window's Songs page and the files' library search at 8,500 and
//! 100,000 songs, and the cost of the `stat` each local row makes per frame.
//!
//! Each measured function copies, line for line, the code it stands for (cited above it), over
//! `opium_core`'s own `fold` and `dedupe::songs`, so that the figures are those of that code.

use std::collections::HashSet;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use opium_core::model::{ItemId, ProviderKind, Track};
use opium_core::text::fold;

/// The heap in use, counted: what a structure holds is the difference before and after it is
/// made (capacity, not length, plus nothing of the allocator's own overhead).
struct Counting;

static HELD: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

// SAFETY: every call goes to the system allocator with the layout it was given; the counter
// only adds and takes away the sizes.
unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        HELD.fetch_add(layout.size() as isize, std::sync::atomic::Ordering::Relaxed);
        // SAFETY: the caller's layout, passed on as it is.
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        HELD.fetch_sub(layout.size() as isize, std::sync::atomic::Ordering::Relaxed);
        // SAFETY: `ptr` came from `alloc` above with this layout.
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn held() -> isize {
    HELD.load(std::sync::atomic::Ordering::Relaxed)
}

/// A small deterministic generator: the same library every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const SYLLABLES: [&str; 24] = [
    "nu", "mor", "wen", "mi", "qua", "ka", "sol", "sive", "da", "bel", "ve", "ti", "ra", "lo",
    "zen", "fa", "ri", "mon", "sha", "el", "do", "vi", "ne", "tor",
];

fn word(rng: &mut Rng) -> String {
    let n = 2 + rng.below(3);
    let mut w = String::new();
    for _ in 0..n {
        w.push_str(SYLLABLES[rng.below(SYLLABLES.len() as u64) as usize]);
    }
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => w,
    }
}

fn words(rng: &mut Rng, at_least: u64, at_most: u64) -> String {
    let n = at_least + rng.below(at_most - at_least + 1);
    (0..n).map(|_| word(rng)).collect::<Vec<_>>().join(" ")
}

/// `n` songs as the generated library has them (13 a album, 44 an artist), 80 % files, 10 %
/// Apple Music's copies of a file (a second apart), 10 % SoundCloud's of their own.
fn library(n: usize) -> Vec<Track> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let artists: Vec<String> = (0..(n / 44).max(1)).map(|_| words(&mut rng, 1, 2)).collect();
    let mut songs: Vec<Track> = Vec::with_capacity(n);
    let mut album = String::new();
    let mut artist = String::new();
    for i in 0..n {
        if i % 13 == 0 {
            album = words(&mut rng, 1, 3);
            artist = artists[rng.below(artists.len() as u64) as usize].clone();
        }
        let provider = match i % 10 {
            8 => ProviderKind::AppleMusic,
            9 => ProviderKind::SoundCloud,
            _ => ProviderKind::Local,
        };
        let copy_of = (provider == ProviderKind::AppleMusic).then(|| songs.last().cloned()).flatten();
        // As the services fill them: a file's path, album key, kept cover and quality
        // (`crates/local/src/lib.rs:308-336`); Apple's catalog and library ids, artwork template
        // and quality (`crates/apple-music/src/convert.rs:50-84`).
        let apple_artwork = |n: usize| opium_core::model::Artwork {
            url_template: format!(
                "https://is1-ssl.mzstatic.com/image/thumb/Music126/v4/3f/0c/9e/3f0c9e1a-6b2d-4e4f-9a7b-{n:012x}/198391{n:06}.jpg/{{size}}x{{size}}bb.jpg"
            ),
            background: Some(0x1a1a1a),
        };
        let track = match copy_of {
            Some(mut copy) => {
                copy.id = ItemId::new(provider, format!("{}", 1_000_000_000 + i));
                copy.library_id = Some(ItemId::new(provider, format!("i.{:014X}", i)));
                copy.album_id = Some(ItemId::new(provider, format!("{}", 1_400_000_000 + i / 13)));
                copy.artwork = Some(apple_artwork(i / 13));
                copy.quality = Some(opium_core::model::AudioQuality {
                    codec: opium_core::model::Codec::Aac,
                    sample_rate: Some(44_100),
                    bit_depth: None,
                    bitrate_kbps: Some(256),
                });
                copy.duration = copy.duration.map(|d| d + Duration::from_secs(1));
                copy
            }
            None => Track {
                id: ItemId::new(
                    provider,
                    format!("/home/user/Music/{artist}/{album}/{:02} {}.flac", i % 13 + 1, i),
                ),
                title: words(&mut rng, 1, 4),
                artists: vec![artist.clone()],
                album: Some(album.clone()),
                album_id: Some(ItemId::new(provider, format!("album:{}\u{1f}{}", fold(&artist), fold(&album)))),
                duration: Some(Duration::from_secs(120 + rng.below(300))),
                track_number: Some((i % 13 + 1) as u32),
                disc_number: None,
                artwork: Some(opium_core::model::Artwork {
                    url_template: format!(
                        "file:///home/user/.local/share/opium/local/covers/{:016x}.jpg",
                        (i / 13) as u64 * 0x9E37_79B9
                    ),
                    background: None,
                }),
                explicit: false,
                quality: Some(opium_core::model::AudioQuality {
                    codec: opium_core::model::Codec::Flac,
                    sample_rate: Some(44_100),
                    bit_depth: Some(16),
                    bitrate_kbps: Some(900),
                }),
                in_library: true,
                library_id: None,
            },
        };
        songs.push(track);
    }
    songs
}

/// `crates/app-desktop/src/ui/songs.rs:445-460`: what each search and filter of the Songs page
/// starts from, made again whenever the song count changes.
struct PreparedSongs {
    copies: HashSet<usize>,
    folded: Vec<String>,
}

fn prepare(songs: &[Track]) -> PreparedSongs {
    PreparedSongs {
        copies: opium_core::dedupe::songs(songs),
        folded: songs
            .iter()
            .map(|t| {
                fold(&format!(
                    "{} {} {}",
                    t.title,
                    t.artists.join(" "),
                    t.album.as_deref().unwrap_or_default()
                ))
            })
            .collect(),
    }
}

/// `crates/app-desktop/src/ui/songs.rs:412-500` (`rebuild_order`), all sources, past its key
/// check: the filter, then the sort by title when asked.
fn order(songs: &[Track], prepared: &PreparedSongs, search: &str, by_title: bool) -> Vec<usize> {
    let query = fold(search.trim());
    let words: Vec<&str> = query.split_whitespace().collect();
    let found = |i: usize| words.is_empty() || words.iter().all(|w| prepared.folded[i].contains(w));
    let mut order: Vec<usize> = (0..songs.len())
        .filter(|&i| !prepared.copies.contains(&i) && found(i))
        .collect();
    if by_title {
        order.sort_by_cached_key(|&i| songs[i].title.to_lowercase());
    }
    order
}

/// `crates/local/src/lib.rs:449-452`.
fn matches(words: &[String], text: &str) -> bool {
    let text = fold(text);
    words.iter().all(|w| text.contains(w.as_str()))
}

/// `crates/local/src/lib.rs:458-467`.
fn rank(query: &str, text: &str) -> u8 {
    let text = fold(text);
    if text == query {
        0
    } else if text.starts_with(query) {
        1
    } else {
        2
    }
}

/// `crates/local/src/lib.rs:486-508`, the songs' part of `search_library`, which is most of it
/// (albums are a 13th as many, artists a 44th): every file folded again at every search.
fn search_library(entries: &[Track], query: &str) -> usize {
    let words: Vec<String> = fold(query).split_whitespace().map(str::to_owned).collect();
    if words.is_empty() {
        return 0;
    }
    let whole = words.join(" ");
    let mut found: Vec<(u8, &Track)> = entries
        .iter()
        .filter(|e| {
            let album = e.album.as_deref().unwrap_or_default();
            matches(
                &words,
                &format!("{} {} {album}", e.title, e.artists.join(" ")),
            )
        })
        .map(|e| (rank(&whole, &e.title), e))
        .collect();
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().take(50).count()
}

/// Median, lowest and highest of `runs` timings of `f`.
fn time(runs: usize, mut f: impl FnMut()) -> (Duration, Duration, Duration) {
    let mut all: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .collect();
    all.sort();
    (all[runs / 2], all[0], all[runs - 1])
}

fn ms(d: Duration) -> String {
    format!("{:.2} ms", d.as_secs_f64() * 1e3)
}

fn report(what: &str, (median, low, high): (Duration, Duration, Duration)) {
    println!("{what}: median {}, {} to {}", ms(median), ms(low), ms(high));
}

/// `stat` on music files, as `App::elsewhere` (`crates/app-desktop/src/app.rs:3816-3818`) does
/// for each local row drawn: `n` files in folders of 13, under `root`.
fn stat_files(root: &Path, n: usize) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(n);
    for i in 0..n {
        let dir = root.join(format!("artist{}", i / 572)).join(format!("album{}", i / 13));
        if i % 13 == 0 {
            std::fs::create_dir_all(&dir).expect("make the folder");
        }
        let path = dir.join(format!("{:02} song {i}.flac", i % 13 + 1));
        if !path.exists() {
            std::fs::write(&path, b"").expect("make the file");
        }
        paths.push(path);
    }
    paths
}

fn main() {
    let runs = 7;
    for n in [8_500, 100_000] {
        let before = held();
        let songs = library(n);
        let library_bytes = held() - before;
        println!("--- {n} songs");
        let before = held();
        let kept_prepared = prepare(&songs);
        let prepared_bytes = held() - before;
        drop(kept_prepared);
        println!(
            "Heap held: the songs as the window keeps them (`App::songs`) {:.1} MiB, {} bytes a song; `prepared` {:.1} MiB more",
            library_bytes as f64 / (1024.0 * 1024.0),
            library_bytes / n as isize,
            prepared_bytes as f64 / (1024.0 * 1024.0)
        );
        report("Songs page, `prepared` made (dedupe::songs + fold of each song)", time(runs, || {
            black_box(prepare(&songs));
        }));
        report("  of which dedupe::songs alone", time(runs, || {
            black_box(opium_core::dedupe::songs(&songs));
        }));
        let prepared = prepare(&songs);
        report("Songs page, the order with no search, unsorted (a page or a count change)", time(runs, || {
            black_box(order(&songs, &prepared, "", false));
        }));
        report("Songs page, the order with no search, by title", time(runs, || {
            black_box(order(&songs, &prepared, "", true));
        }));
        report("Songs page, a key typed: search \"a\", by title", time(runs, || {
            black_box(order(&songs, &prepared, "a", true));
        }));
        report("Songs page, a key typed: search \"mor\", by title", time(runs, || {
            black_box(order(&songs, &prepared, "mor", true));
        }));
        report("Songs page, a key typed: search \"mor\", unsorted", time(runs, || {
            black_box(order(&songs, &prepared, "mor", false));
        }));
        println!(
            "  (\"a\" keeps {} songs, \"mor\" {})",
            order(&songs, &prepared, "a", false).len(),
            order(&songs, &prepared, "mor", false).len()
        );
        // A song double-clicked in Songs: `shown_tracks` (`ui/songs.rs:403-410`) clones every song
        // listed; played on a paired device, `play_list` (`app.rs:3874-3880`) sends them all,
        // serialized by `connect::Inner::encode` (`crates/connect/src/lib.rs:804-806`).
        let listed = order(&songs, &prepared, "", false);
        report("A song played from Songs: shown_tracks' clone of every song listed", time(runs, || {
            black_box(listed.iter().map(|&i| songs[i].clone()).collect::<Vec<Track>>());
        }));
        let shown: Vec<Track> = listed.iter().map(|&i| songs[i].clone()).collect();
        let mut json_len = 0;
        report("  and its PlayList to a paired device, serialized", time(runs, || {
            json_len = black_box(serde_json::to_vec(&shown).expect("tracks serialize")).len();
        }));
        println!(
            "  (PlayList's songs: {} listed, {:.1} MiB of JSON, {} bytes a song; the receiver refuses past 32 MiB, `crates/connect/src/wire.rs:18,163-165`)",
            shown.len(),
            json_len as f64 / (1024.0 * 1024.0),
            json_len / shown.len().max(1)
        );
        report("Files' search_library, songs part: \"mor\"", time(runs, || {
            black_box(search_library(&songs, "mor"));
        }));
        report("Files' search_library, songs part: \"numor wen\"", time(runs, || {
            black_box(search_library(&songs, "numor wen"));
        }));
    }

    // A first load page by page (Navidrome's 500, `crates/navidrome/src/lib.rs:29`), the Songs
    // page shown: each page changes the count, so `prepared` and the order are made again over
    // every song so far (`crates/desktop/src/backend.rs:3058-3063`, `songs.rs:439-447`).
    for (n, page) in [(20_000usize, 500usize), (40_000, 500)] {
        let all = library(n);
        let mut total = Duration::ZERO;
        let mut longest = Duration::ZERO;
        let mut shown: Vec<Track> = Vec::new();
        for chunk in all.chunks(page) {
            shown.extend_from_slice(chunk);
            let start = Instant::now();
            let prepared = prepare(&shown);
            black_box(order(&shown, &prepared, "", false));
            let took = start.elapsed();
            total += took;
            longest = longest.max(took);
        }
        println!(
            "--- first load of {n} songs in pages of {page}, Songs shown: {} pages, window thread {} in all, the longest frame's share {}",
            n / page,
            ms(total),
            ms(longest)
        );
    }

    // The `stat` of `App::elsewhere`, once per local row drawn, on files whose folder entries
    // are cached (the best case: a local disk, warm).
    let root = std::env::args().nth(1).map(PathBuf::from).expect("a folder for the stat test");
    let paths = stat_files(&root, 8_500);
    // Warm.
    for p in &paths {
        black_box(p.is_file());
    }
    let rows = 26; // 1309 points of window over rows of 52 (`ui/tracks.rs:16`)
    let frames = 240;
    let (median, low, high) = time(runs, || {
        for f in 0..frames {
            for r in 0..rows {
                black_box(paths[(f * 7 + r) % paths.len()].is_file());
            }
        }
    });
    let per = median.as_secs_f64() / (rows * frames) as f64;
    println!(
        "--- stat: {} calls (26 rows x 240 frames, one second scrolling at 240 Hz): median {}, {} to {}; {:.2} us a call",
        rows * frames,
        ms(median),
        ms(low),
        ms(high),
        per * 1e6
    );
}
