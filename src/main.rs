use std::io::{stdin, Read};
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

extern crate clap;
use clap::{Arg, App, AppSettings, SubCommand};

mod lib;
use lib::KeystrokeLogger;
use lib::{KeynomeAuthenticator, KeynomeAuthenticatorDiffParams, UserProfile};

fn save_user_profile(profile: &UserProfile, filename: &str) {
    let serialized = profile.serialize();
    let path = Path::new(filename);
    let mut file = File::create(&path).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();
    println!("user profile stored in {}.", filename);
}

fn load_user_profile(filename: &str) -> UserProfile {
    let path = Path::new(filename);
    let mut file = File::open(&path).unwrap();

    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    let profile = UserProfile::deserialize(&s);
    profile
}

fn main() {

    // set commandline options
    let matches = App::new("Keynome")
        .author("Minhwan Kim <azurelysium@gmail.com>")
        .about("Continuous authentication using Keystroke Dynamics")
        .version("0.0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("verbosity")
             .short("v")
             .multiple(true)
             .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("profile")
                    .about("generates a user profile")
                    .arg(Arg::with_name("n_profile")
                         .long("n_profile")
                         .value_name("NUMBER")
                         .help("Sets the number of profile keyevents")
                         .default_value("5000")
                         .takes_value(true))
                    .arg(Arg::with_name("n_sample")
                         .long("n_sample")
                         .value_name("NUMBER")
                         .help("Sets the number of sample keyevents")
                         .default_value("1000")
                         .takes_value(true))
                    .arg(Arg::with_name("min_instances")
                         .long("min_instances")
                         .value_name("NUMBER")
                         .help("Sets the minimum number of digraph stats instances")
                         .default_value("2")
                         .takes_value(true))
                    .arg(Arg::with_name("max_comparisons")
                         .long("max_comparisons")
                         .value_name("NUMBER")
                         .help("Sets the maximum number of comparisons")
                         .default_value("100")
                         .takes_value(true))
                    .arg(Arg::with_name("use_dispersion")
                         .long("use_dispersion")
                         .value_name("NUMBER")
                         .help("Sets the flag for using dispersion when diff computed")
                         .default_value("0")
                         .takes_value(true))
                    .arg(Arg::with_name("outfile")
                         .short("o")
                         .long("outfile")
                         .value_name("FILE")
                         .help("Sets an output file where a user profile will be stored")
                         .takes_value(true))
        )
        .subcommand(SubCommand::with_name("auth")
                    .about("authenticates a user using the pre-computed user profile")
                    .arg(Arg::with_name("infile")
                         .short("i")
                         .long("infile")
                         .value_name("FILE")
                         .help("Sets an input file where a user profile is stored")
                         .required(true)
                         .takes_value(true))
        )
        .get_matches();

    let verbosity = matches.occurrences_of("verbosity");

    // process subcommand

    // Subcomnad - profile
    if let Some(matches) = matches.subcommand_matches("profile") {

        let n_profile: u32 = matches.value_of("n_profile").unwrap().parse().unwrap();
        let n_sample: u32 = matches.value_of("n_sample").unwrap().parse().unwrap();
        let min_instances: u32 = matches.value_of("min_instances").unwrap().parse().unwrap();
        let max_comparisons: u32 = matches.value_of("max_comparisons").unwrap().parse().unwrap();
        let use_dispersion: u32 = matches.value_of("use_dispersion").unwrap().parse().unwrap();

        println!("Press ! key to stop recording keystrokes");

        let mut kstr = KeystrokeLogger::new();
        kstr.set_events_limit(n_profile as usize);

        // read user keystrokes from Stdin character by character
        let mut cnt_newline = 0;
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

            // if shelljacked-terminal is closed, newline is typed infinitely
            cnt_newline = if ch == '\n' { cnt_newline + 1 } else { 0 };
            if cnt_newline > 10 {
                break;
            }
        }

        // compute statistics and serialize this
        let stats = kstr.compute_digraph_statistics();
        if verbosity >= 2 {
            for (k, v) in stats.iter() {
                println!("{:?}: mean({}), std({})", k, v.mean, v.std);
            }
        }

        // compute inherent difference level
        let diff_params = KeynomeAuthenticatorDiffParams {
            dispersion: if use_dispersion == 1 { true } else { false },
            min_instances,
            max_comparisons,
        };

        let events = kstr.get_key_events();
        let diff_base = KeynomeAuthenticator::compute_diff_base(events, 12, 6, &diff_params).unwrap();

        // save a user profile
        let profile = UserProfile::new(n_profile, n_sample, diff_base, &diff_params, &stats);
        let filename = matches.value_of("outfile").unwrap_or("profile.json");
        save_user_profile(&profile, &filename);
    }

    // Subcomnad - auth
    if let Some(matches) = matches.subcommand_matches("auth") {
        let filename = matches.value_of("infile").unwrap();
        let profile = load_user_profile(&filename);
        println!("n_profile: {}", profile.n_profile);
        println!("n_sample: {}", profile.n_sample);
        println!("diff_base: {}", profile.diff_base);
        println!("diff_params: {:?}", profile.diff_params);
    }
}
