// Times the token's search on Apple's page and script, the code before (two regular
// expressions, built once as its LazyLock did) against the new auth.rs's searches.
mod new_search;
use std::time::{Duration, Instant};
fn median_p95(mut t: Vec<Duration>) -> (Duration, Duration) {
    t.sort();
    (t[t.len() / 2], t[t.len() * 95 / 100])
}
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let page = std::fs::read_to_string(&a[0]).unwrap();
    let script = std::fs::read_to_string(&a[1]).unwrap();
    let bundle = regex::Regex::new(r"/assets/index~[^/]+\.js").unwrap();
    let jwt = regex::Regex::new(r"eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").unwrap();
    // Built once before timing, as both sides' statics are.
    new_search::main_script("");
    new_search::first_token("");
    let (mut old, mut new) = (Vec::new(), Vec::new());
    for _ in 0..101 {
        let t = Instant::now();
        let b = bundle.find(&page).unwrap().as_str().to_owned();
        let j = jwt.find(&script).unwrap().as_str().to_owned();
        old.push(t.elapsed());
        std::hint::black_box((b, j));
        let t = Instant::now();
        let b = new_search::main_script(&page).unwrap().to_owned();
        let j = new_search::first_token(&script).unwrap();
        let ok = new_search::claims(j).is_some();
        new.push(t.elapsed());
        std::hint::black_box((b, j.len(), ok));
    }
    assert_eq!(bundle.find(&page).unwrap().as_str(), new_search::main_script(&page).unwrap());
    assert_eq!(jwt.find(&script).unwrap().as_str(), new_search::first_token(&script).unwrap());
    let (m, p) = median_p95(old);
    println!("before: median {m:?}, p95 {p:?}");
    let (m, p) = median_p95(new);
    println!("after:  median {m:?}, p95 {p:?}");
}
