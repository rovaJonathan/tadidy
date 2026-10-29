//! SRC-01 / MVP criterion 6: typing "sel" answers in under 50 ms over 10 000 clips. The clips are
//! synthetic: words drawn from a seeded generator, never real clipboard content (AGENTS.md §2).

#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use rusqlite::{Connection, params};
use tadidy_lib::clip::{ClipKind, ClipQuery, content_hash};
use tadidy_lib::crypto::DbKey;
use tadidy_lib::storage::clips::list;
use tadidy_lib::storage::open;

const CLIPS: i64 = 10_000;
const RUNS: usize = 20;
const BUDGET: Duration = Duration::from_millis(50);
const WORDS_PER_CLIP: usize = 12;

/// A small deterministic generator (xorshift64), so every run searches the same data.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).unwrap()
    }
}

/// 2 000 pseudo-words, 40 of which start with "sel" (about a quarter of the clips contain one).
fn vocabulary(random: &mut Seeded) -> Vec<String> {
    const SYLLABLES: [&str; 16] = [
        "ba", "ko", "ri", "ta", "mu", "ne", "so", "la", "vi", "do", "pe", "gu", "fa", "zo", "hi",
        "ju",
    ];
    let mut words: Vec<String> = (0..2_000)
        .map(|_| {
            (0..2 + random.below(3))
                .map(|_| SYLLABLES[random.below(SYLLABLES.len())])
                .collect()
        })
        .collect();
    for (n, word) in words.iter_mut().take(40).enumerate() {
        *word = format!(
            "sel{}{}",
            SYLLABLES[n % SYLLABLES.len()],
            n / SYLLABLES.len()
        );
    }
    words
}

/// Fills `clips` in one transaction; the full-text triggers index every row.
fn seed(conn: &Connection) {
    let mut random = Seeded(0x9E37_79B9_7F4A_7C15);
    let words = vocabulary(&mut random);
    let tx = conn.unchecked_transaction().unwrap();
    {
        let mut insert = tx
            .prepare(
                "INSERT INTO clips (id, kind, content_hash, plain_text, preview, source_name, \
                 byte_size, created_at, last_copied_at) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?7)",
            )
            .unwrap();
        for n in 0..CLIPS {
            let text: Vec<&str> = (0..WORDS_PER_CLIP)
                .map(|_| words[random.below(words.len())].as_str())
                .collect();
            let text = text.join(" ");
            insert
                .execute(params![
                    format!("clip-{n:05}"),
                    ClipKind::Text,
                    content_hash(text.as_bytes()),
                    text,
                    "Editor",
                    i64::try_from(text.len()).unwrap(),
                    1_000 + n,
                ])
                .unwrap();
        }
    }
    tx.commit().unwrap();
}

#[test]
fn searching_sel_over_10_000_clips_takes_under_50_ms_at_the_median() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(&dir.path().join("perf.db"), &DbKey::from_bytes([0xA; 32])).unwrap();
    db.with_conn(seed);
    let query = ClipQuery {
        search: Some("sel".to_owned()),
        ..ClipQuery::default()
    };

    let mut timings: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            let page = db.with_conn(|conn| list(conn, &query)).unwrap();
            let elapsed = started.elapsed();
            assert_eq!(page.items.len(), 50);
            assert!(
                page.total > 1_000,
                "the data should make the search work: {}",
                page.total
            );
            elapsed
        })
        .collect();
    timings.sort();
    let median = timings[RUNS / 2];

    println!(
        "search \"sel\" over {CLIPS} clips: median {median:?}, worst {:?}",
        timings[RUNS - 1]
    );
    assert!(median < BUDGET, "median {median:?} is over {BUDGET:?}");
}
