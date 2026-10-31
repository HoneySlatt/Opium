//! Where a song stands in a playlist: a key that sorts it among the others,
//! one found between any two, so that a song put somewhere changes only its
//! own place and two devices doing so at once don't undo each other.

use rand::RngExt;

/// The digits of a place, in the order they sort in.
const DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const LAST: usize = DIGITS.len() - 1;

fn value(c: u8) -> usize {
    DIGITS.iter().position(|&d| d == c).unwrap_or(0)
}

/// A place after `a` and before `b`, either left out at an end. Two
/// random digits close it: two devices putting a song at the same spot at
/// once get places of their own, and never one ending in `0`, which nothing
/// would fit before.
pub fn between(a: Option<&str>, b: Option<&str>) -> String {
    let a = a.unwrap_or("").as_bytes();
    let b = b.map(str::as_bytes).filter(|b| *b > a);
    let mut key = match b {
        Some(b) => mid(a, Some(b)),
        None => after(a),
    };
    let mut rng = rand::rng();
    for _ in 0..2 {
        key.push(DIGITS[rng.random_range(1..DIGITS.len())]);
    }
    String::from_utf8(key).unwrap_or_default()
}

/// The next place after `a`, counting on its first three digits, so that
/// places stay short as songs are added one after another: a quarter of a
/// million of them before one gets longer.
fn after(a: &[u8]) -> Vec<u8> {
    const WIDTH: usize = 3;
    let mut count: Vec<usize> = (0..WIDTH)
        .map(|i| a.get(i).map_or(0, |&c| value(c)))
        .collect();
    for i in (0..WIDTH).rev() {
        if count[i] < LAST {
            count[i] += 1;
            count[i + 1..].fill(0);
            return count.into_iter().map(|d| DIGITS[d]).collect();
        }
    }
    let mut key = a[..WIDTH].to_vec();
    key.extend(after(&a[WIDTH..]));
    key
}

/// Halfway between `a` and `b`, read as fractions. Never a beginning of `b`,
/// so that digits put after it keep it before `b`.
fn mid(a: &[u8], b: Option<&[u8]>) -> Vec<u8> {
    // The digits both start with, which the place starts with too; `a`
    // shorter reads as followed by zeros.
    let digit = |i: usize| a.get(i).copied().unwrap_or(DIGITS[0]);
    let n = b.map_or(0, |b| {
        (0..b.len()).take_while(|&i| digit(i) == b[i]).count()
    });
    let mut key: Vec<u8> = (0..n).map(digit).collect();
    let rest_a = a.get(n..).unwrap_or(&[]);
    let rest_b = b.map(|b| &b[n..]).filter(|b| !b.is_empty());
    let da = rest_a.first().map_or(0, |&c| value(c));
    let db = rest_b
        .and_then(|b| b.first())
        .map_or(DIGITS.len(), |&c| value(c));
    if db > da + 1 {
        key.push(DIGITS[(da + db) / 2]);
    } else {
        // Digits next to each other: `a`'s, then anywhere after the rest of
        // `a`, which `b` no longer bounds.
        key.push(DIGITS[da]);
        key.extend(mid(rest_a.get(1..).unwrap_or(&[]), None));
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn between_two_is_between_them() {
        let pairs = [
            (None, None),
            (Some("V"), None),
            (None, Some("V")),
            (Some("1"), Some("2")),
            (Some("1"), Some("2X")),
            (Some("1Z"), Some("2")),
            (Some("zz"), None),
            (None, Some("01")),
            (Some("Vab"), Some("Vac")),
        ];
        for (a, b) in pairs {
            for _ in 0..50 {
                let k = between(a, b);
                assert!(a.is_none_or(|a| a < k.as_str()), "{a:?} < {k}");
                assert!(b.is_none_or(|b| k.as_str() < b), "{k} < {b:?}");
                assert!(!k.ends_with('0'));
            }
        }
    }

    #[test]
    fn songs_added_one_after_another_keep_short_places() {
        let mut last: Option<String> = None;
        for _ in 0..2000 {
            let next = between(last.as_deref(), None);
            assert!(last.as_deref().is_none_or(|l| l < next.as_str()));
            last = Some(next);
        }
        assert_eq!(last.unwrap().len(), 5);
        assert!(between(Some("zzzab"), None).as_str() > "zzzab");
    }

    #[test]
    fn songs_put_anywhere_keep_their_order() {
        let mut places: Vec<String> = Vec::new();
        let mut rng = rand::rng();
        for _ in 0..3000 {
            let at = rng.random_range(0..=places.len());
            let a = at.checked_sub(1).map(|i| places[i].as_str());
            let b = places.get(at).map(String::as_str);
            let k = between(a, b);
            places.insert(at, k);
        }
        assert!(places.windows(2).all(|w| w[0] < w[1]));
    }
}
