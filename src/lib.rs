use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::thread;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::{Arc, Mutex};

use itertools::Itertools;
use walkdir::WalkDir;
use threadpool::ThreadPool;


/// Return an iterator of lines for a given file
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

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
                let filename = String::from(entry.path().to_str().unwrap());
                let substring = String::from(substring);
                pool.execute(move || {
                    let result = read_lines(&filename);
                    match result {
                        Ok(lines) => {
                            let mut count = 0;
                            for chunk in &lines.into_iter().chunks(100) {
                                count += chunk.map(|line| {
                                    line.unwrap_or("".to_string()).matches(&substring).count()
                                }).sum::<usize>()
                            }
                            let mut storage = storage.lock().unwrap();
                            let default = 0;
                            let value = storage.get(&filename).unwrap_or(&default);
                            let value = value + count;
                            storage.insert(filename, value);
                        },
                        Err(error) => eprintln!("{filename}: {error}"),
                    }
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
