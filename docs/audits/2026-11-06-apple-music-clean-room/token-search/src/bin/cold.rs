// The first search of a process, its regular expressions built then, as at Opium's start:
// `cold old|new page script`.
#[path = "../new_search.rs"]
mod new_search;
use std::time::Instant;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let page = std::fs::read_to_string(&a[1]).unwrap();
    let script = std::fs::read_to_string(&a[2]).unwrap();
    let t = Instant::now();
    if a[0] == "old" {
        let bundle = regex::Regex::new(r"/assets/index~[^/]+\.js").unwrap();
        let jwt = regex::Regex::new(r"eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").unwrap();
        std::hint::black_box((bundle.find(&page).is_some(), jwt.find(&script).is_some()));
    } else {
        let b = new_search::main_script(&page).is_some();
        let j = new_search::first_token(&script).and_then(new_search::claims).is_some();
        std::hint::black_box((b, j));
    }
    println!("{}", t.elapsed().as_nanos());
}
