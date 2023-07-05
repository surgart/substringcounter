use std::collections::HashMap;
use std::fs::File;
use std::io::{self, prelude::*, SeekFrom, Seek};
use std::path::{Path, PathBuf};

use itertools::Itertools;
use walkdir::WalkDir;


/// Walks through `directory` and it's subdirectories and returns iterator of files
fn walkfiles<P>(directory: P) -> impl Iterator<Item = PathBuf>
where P: AsRef<Path>, {
    WalkDir::new(directory)
        .into_iter()
        .filter(|result| {
            match result {
                Ok(entry) => entry.file_type().is_file(),
                Err(error) => {
                    eprintln!("{error}");
                    false
                },
            }
        })
        .map(|entry| {
            entry.expect("Expected filepath").into_path()
        })
}

/// Counts `substring` matches in a chunk, returns `count` and `position` of the last match
/// If `count` is zero (no any matches) then `position` equals to zero
fn count_matches_in_chunk(chunk: &[u8], substring: &str) -> (usize, usize) {
    let mut counter: usize = 0;
    let substring = substring.as_bytes();
    let substring_length = substring.len();
    let chunk_length = chunk.len();
    let mut pivot = 0;
    let mut pos_of_the_last_match = 0;
    loop {
        let (_, tail) = chunk.split_at(pivot);
        if tail.starts_with(substring) {
            counter += 1;
            pos_of_the_last_match = pivot;
            pivot += substring_length;
        } else {
            pivot += 1;
        }
        if pivot >= chunk_length {
            break;
        }
    }
    (counter, pos_of_the_last_match)
}

/// Count `substring` matches in a file
fn count_matches_in_file<P>(filepath: P, substring: &str) -> Result<usize, io::Error>
where P: AsRef<Path>, {
    const BUFFER_SIZE: usize = 8192;
    let mut buffer = [0 as u8; BUFFER_SIZE];
    let mut counter: usize = 0;
    let mut f = File::open(filepath)?;
    let filesize = f.metadata()?.len();
    let mut seekfrom: u64 = 0;
    loop {
        seekfrom = f.seek(SeekFrom::Start(seekfrom))?;
        let size = f.read(&mut buffer)?;
        if size == 0 {
            break;
        }

        let (chunk, _) = buffer.split_at(size);
        let (count, pos_of_the_last_match) = count_matches_in_chunk(chunk, substring);
        counter += count;

        // Calculate next `seekfrom`
        let pos_after_the_last_match = if count > 0 { pos_of_the_last_match + substring.len() } else { 0 };

        seekfrom += chunk.len() as u64;
        let correction = chunk.len() - pos_after_the_last_match;
        let correction = if correction >= substring.len() { substring.len() - 1 } else { correction };
        if pos_after_the_last_match > 0 {
            seekfrom -= correction as u64;
        }

        if seekfrom > filesize {
            break;
        }
    }
    Ok(counter)
}

/// Count `substring` in a file asynchronously
pub async fn async_count_substring_in_file(filepath: String, substring: String) -> Option<usize>
{
    let (send, recv) = tokio::sync::oneshot::channel();
    let filepath = filepath.to_string();
    let substring = substring.to_string();
    rayon::spawn(move || {
        let filepath = filepath.as_str();
        let substring = substring.as_str();
        let result = count_matches_in_file(filepath, substring);
        match result {
            Ok(count) => {
                let _ = send.send(count);
            },
            Err(error) => eprintln!("{}: {}", filepath, error),
        };
    });

    match recv.await {
        Ok(count) => Some(count),
        Err(error) => {
            eprintln!("Thread message: {error}");
            None
        }
    }
}

/// Count asynchronously `substring` in files in a given `directory` and print results into stdio
pub async fn async_count_substring_in_files<P>(directory: P, substring: String)
where P: AsRef<Path>, {
    let tasks = walkfiles(directory)
    .map(|path| {
        let filepath = path.as_path().to_str().unwrap().to_string();
        let filepath2 = filepath.clone();
        let substring = substring.clone();
        (filepath, async_count_substring_in_file(filepath2, substring))
    });

    let mut storage: HashMap<String, usize> = HashMap::new();
    const CHUNK_SIZE: usize = 500;
    for chunk in tasks.chunks(CHUNK_SIZE).into_iter() {
        let mut results = Vec::with_capacity(CHUNK_SIZE);
        let chunk: Vec<_> = chunk.collect();

        for (filepath, future) in chunk {
            results.push((filepath, tokio::spawn(future)));
        }
        for (filepath, future) in results {
            match future.await {
                Ok(result) => {
                    if let Some(count) = result {
                        storage.insert(filepath, count);
                    }
                },
                Err(error) => eprintln!("{filepath}: {error}"),
            };
        }
    }

    let result = serde_json::to_string_pretty(&storage);
    match result {
        Ok(json) => println!("{}", json),
        Err(error) => eprintln!("{}", error),
    }
}

mod test {
    #[test]
    fn test_count_matches_in_chunk() {
        let cases = [
            ("ababababab".as_bytes(), "ab", (5, 8)),
            ("ababababab".as_bytes(), "aba", (2, 4)),
        ];
        for case in cases {
            let (chunk, substring, expected) = case;
            let count = crate::count_matches_in_chunk(&chunk, &substring);
            assert_eq!(count, expected);
        }
    }

    #[test]
    fn test_count_matches_in_small_files() {
        let cases = [
            ("./samples/small/aaaa10e2.txt", "aaaa", 100),
            ("./samples/small/ababa10e6_no_newlines.txt", "aba", 1000_000),
            ("./samples/small/aaaa10e6.txt", "aaaa", 1000_000),
            ("./samples/small/aaaa10e5.txt", "aaaa", 100_000),
            ("./samples/small/aaaa10e3.txt", "aaa", 1000),
            ("./samples/small/abcde5432_no_newlines.txt", "bcd", 5432),
        ];
        for (filepath, substring, expected) in cases {
            let result = crate::count_matches_in_file(filepath, substring);
            let count = match result {
                Ok(count) => count,
                Err(error) => {
                    assert!(false, "{filepath} {error}");
                    continue
                },
            };
            assert_eq!(count, expected);
        }
    }

    #[test]
    #[ignore]
    fn test_count_matches_in_large_files() {
        let cases = [
            ("./samples/large/ababa10e9_no_newlines.txt", "aba", 1000_000_000),
        ];
        for (filepath, substring, expected) in cases {
            let result = crate::count_matches_in_file(filepath, substring);
            let count = match result {
                Ok(count) => count,
                Err(error) => {
                    assert!(true, "{filepath} {error}");
                    continue
                },
            };

            assert_eq!(count, expected);
        }
    }
}