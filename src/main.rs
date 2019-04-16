use std::io::{stdin, Read};
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;

extern crate clap;
use clap::{Arg, App, AppSettings, SubCommand};

mod lib;
use lib::KeystrokeLogger;
use lib::{Digraph, DigraphStats};

fn save_user_profile(stats: &HashMap<Digraph, DigraphStats>, filename: &str) {
    let serialized = KeystrokeLogger::serialize_digraph_statistics(stats);

    let path = Path::new(filename);
    let mut file = File::create(&path).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();
}

fn load_user_profile(filename: &str) -> HashMap<Digraph, DigraphStats> {
    let path = Path::new(filename);
    let mut file = File::open(&path).unwrap();

    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    let stats = KeystrokeLogger::deserialize_digraph_statistics(&s);
    stats
}

fn main() {

    // set commandline options
    let matches = App::new("Keynome")
        .author("Minhwan Kim <azurelysium@gmail.com>")
        .about("Continuous authentication using Keystroke Dynamics")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("v")
             .short("v")
             .multiple(true)
             .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("profile")
                    .about("generates a user profile")
                    .arg(Arg::with_name("outfile")
                         .short("o")
                         .long("outfile")
                         .value_name("FILE")
                         .help("Sets an output file where a user profile will be stored")
                         .takes_value(true)))
        .subcommand(SubCommand::with_name("auth")
                    .about("authenticates a user using the pre-computed user profile")
                    .arg(Arg::with_name("infile")
                         .short("i")
                         .long("infile")
                         .value_name("FILE")
                         .help("Sets an input file where a user profile is stored")
                         .required(true)
                         .takes_value(true)))
        .get_matches();

    let verbosity = matches.occurrences_of("v");

    // process subcommand

    // Subcomnad - profile
    if let Some(matches) = matches.subcommand_matches("profile") {

        println!("Press ! key to stop recording keystrokes");

        let mut kstr = KeystrokeLogger::new();
        kstr.set_events_limit(5000);

        // read user keystrokes from Stdin character by character
        let mut buf = [0];
        while let Ok(_) = stdin().read(&mut buf) {
            let ch = buf[0] as char;
            if verbosity >= 1 {
                println!("CHAR {:?}", ch);
            }

            if ch == '!' {
                break;
            } else if ch.is_ascii_alphabetic() {
                kstr.add_keystroke(ch);
            }
        }

        // compute statistics and serialize this
        let stats = kstr.compute_digraph_statistics();
        if verbosity >= 2 {
            for (k, v) in stats.iter() {
                println!("{:?}: mean({}), std({})", k, v.mean, v.std);
            }
        }

        // save a user profile
        let filename = matches.value_of("outfile").unwrap_or("profile.json");
        save_user_profile(&stats, &filename);
    }

    // Subcomnad - auth
    if let Some(matches) = matches.subcommand_matches("auth") {
        let filename = matches.value_of("infile").unwrap();
        let stats_profile = load_user_profile(&filename);
    }
}
