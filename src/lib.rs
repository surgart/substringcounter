use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::thread;
use std::fs::File;
use std::io::{self, prelude::*, SeekFrom, Seek};
use std::path::Path;
use std::sync::{Arc, Mutex};

use walkdir::WalkDir;
use threadpool::ThreadPool;


/// Walk through `directory` and it's subdirectories and return iterator of files
fn walkfiles<P>(directory: P) -> impl Iterator<Item = Result<walkdir::DirEntry, walkdir::Error>>
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
}

/// Counts `substring` in a chunk, returns `count` and `position` of the last match
/// If `count` is zero (no any matches) then `position` equals to zero
fn count_substring_in_chunk(chunk: &[u8], substring: &str) -> (usize, usize) {
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

/// Count `substring` in a file
fn count_substring_in_file<P>(filepath: P, substring: &str) -> Result<usize, io::Error>
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
        let (count, pos_of_the_last_match) = count_substring_in_chunk(chunk, substring);
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

/// Count `substring` in files in a given `directory` and collect results into HashMap
pub fn count_substring_in_files<P>(directory: P, substring: &str)
where P: AsRef<Path> + {
    let storage: HashMap<String, usize> = HashMap::new();
    let storage_mutex = Arc::new(Mutex::new(storage));

    let n_workers = NonZeroUsize::new(8).expect("Count of workers must be non zero usize");
    let n_workers = thread::available_parallelism().unwrap_or(n_workers);
    let pool = ThreadPool::new(n_workers.get());

    for entry in walkfiles(directory) {
        match entry {
            Ok(entry) => {
                let storage = Arc::clone(&storage_mutex);
                let filepath = String::from(entry.path().to_str().unwrap());
                let substring = String::from(substring);
                pool.execute(move || {
                    let result = count_substring_in_file(&filepath, &substring);
                    match result {
                        Ok(count) => {
                            let mut storage = storage.lock().unwrap();
                            let default = 0;
                            let value = storage.get(&filepath).unwrap_or(&default);
                            let value = value + count;
                            storage.insert(filepath, value);
                        },
                        Err(error) => eprintln!("{}: {}", filepath, error),
                    };
                });
            },
            Err(err) => eprintln!("{err}"),
        };
    };
    pool.join();

    let storage_mutex = Arc::try_unwrap(storage_mutex).unwrap();
    let storage = storage_mutex.into_inner().unwrap();
    let result = serde_json::to_string_pretty(&storage);
    match result {
        Ok(json) => println!("{}", json),
        Err(error) => eprintln!("{}", error),
    }
}

mod test {
    #[test]
    fn test_count_substring_in_chunk() {
        let cases = [
            ("ababababab".as_bytes(), "ab", (5, 8)),
            ("ababababab".as_bytes(), "aba", (2, 4)),
        ];
        for case in cases {
            let (chunk, substring, expected) = case;
            let count = crate::count_substring_in_chunk(&chunk, &substring);
            assert_eq!(count, expected);
        }
    }

    #[test]
    fn test_count_substring_in_small_files() {
        let cases = [
            ("./samples/small/aaaa10e2.txt", "aaaa", 100),
            ("./samples/small/ababa10e6_no_newlines.txt", "aba", 1000_000),
            ("./samples/small/aaaa10e6.txt", "aaaa", 1000_000),
            ("./samples/small/aaaa10e5.txt", "aaaa", 100_000),
            ("./samples/small/aaaa10e3.txt", "aaa", 1000),
            ("./samples/small/abcde5432_no_newlines.txt", "bcd", 5432),
        ];
        for (filepath, substring, expected) in cases {
            let result = crate::count_substring_in_file(filepath, substring);
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
    fn test_count_substring_in_large_files() {
        let cases = [
            ("./samples/large/ababa10e9_no_newlines.txt", "aba", 1000_000_000),
        ];
        for (filepath, substring, expected) in cases {
            let result = crate::count_substring_in_file(filepath, substring);
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