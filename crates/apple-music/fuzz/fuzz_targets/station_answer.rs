//! A station's gathering, its calls answered with what the input says: no panic, an end
//! within the bounds on calls (in flight and in all), and at most the songs asked, each once.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// A `next-tracks` answer of one song for each byte of `ids`, the byte its id: answers that
/// read, which repeat songs as often as the input repeats bytes.
fn answer_of(ids: &[u8]) -> Vec<u8> {
    let songs: Vec<String> = ids
        .iter()
        .map(|id| {
            format!("{{\"id\":\"{id}\",\"type\":\"songs\",\"attributes\":{{\"name\":\"s\"}}}}")
        })
        .collect();
    format!("{{\"data\":[{}]}}", songs.join(",")).into_bytes()
}

fuzz_target!(|data: &[u8]| {
    // The songs asked, then the answers in turn, split at 0xFF: one starting with `{` is a
    // body as it is, any other the songs its bytes name. The last answer is given again
    // once they run out.
    let Some((&count, rest)) = data.split_first() else {
        return;
    };
    let count = usize::from(count % 64);
    let answers: Vec<Vec<u8>> = rest
        .split(|&b| b == 0xFF)
        .map(|piece| {
            if piece.first() == Some(&b'{') {
                piece.to_vec()
            } else {
                answer_of(piece)
            }
        })
        .collect();
    let mut next = 0;
    let gathered = apple_music::fuzzing::gather_station(count, || {
        let answer = answers[next.min(answers.len() - 1)].clone();
        next += 1;
        answer
    });
    assert!(gathered.calls_sent <= apple_music::fuzzing::MOST_CALLS);
    assert!(gathered.most_in_flight <= apple_music::fuzzing::MOST_IN_FLIGHT);
    assert!(next <= gathered.calls_sent, "no answer without its call");
    if count == 0 {
        assert_eq!(gathered.calls_sent, 0);
    }
    if let Ok(songs) = gathered.songs {
        assert!(songs.len() <= count);
        let mut ids = songs.clone();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), songs.len(), "each song once");
    }
});
