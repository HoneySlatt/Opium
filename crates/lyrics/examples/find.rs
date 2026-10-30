//! `find <seconds> <artist> -- <title>`: the song's lyrics from each source,
//! to see what they give.

use std::time::Duration;

use lyrics::{Finder, Query, Source};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let split = args.iter().position(|a| a == "--").unwrap_or(args.len());
    let secs: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let query = Query {
        artist: args.get(1..split).unwrap_or_default().join(" "),
        title: args.get(split + 1..).unwrap_or_default().join(" "),
        album: None,
        duration: (secs > 0).then(|| Duration::from_secs(secs)),
    };
    let finder = Finder::new();
    for source in Source::ALL {
        let started = std::time::Instant::now();
        match finder.find(&query, &[source]).await {
            Some((l, _)) => println!(
                "{:<20} {} lines, synced {}, in {:?}: {:?}",
                source.name(),
                l.lines.len(),
                l.synced,
                started.elapsed(),
                l.lines
                    .iter()
                    .find(|x| !x.text.is_empty())
                    .map(|x| (x.start, &x.text))
            ),
            None => println!("{:<20} nothing, in {:?}", source.name(), started.elapsed()),
        }
    }
}
