use clap::ArgMatches;
use substringcounter::async_count_substring_in_files;
use clap::{Command, Arg};

fn parse_config() -> Command {
    Command::new("Substring Match Counter")
    .version("0.0.1")
    .about("Count substring matches for every file in a directory")
    .arg(Arg::new("directory")
        .required(true)
        .help("Directory path"))
    .arg(Arg::new("substring")
        .required(true)
        .help("Substring"))
}

#[tokio::main]
async fn main(){
    let command: Command = parse_config();
    let matches: ArgMatches = command.get_matches();

    let directory = matches.get_one::<String>("directory").expect("Directory argument is required").to_string();
    let substring = matches.get_one::<String>("substring").expect("Substring argument is required").to_string();
    async_count_substring_in_files(directory, substring).await;
}
