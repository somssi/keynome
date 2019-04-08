use std::io::{stdin, Read};

mod lib;
use lib::KeystrokeLogger;

fn main() {
    println!("Keynome - continuous authentication based on keystroke dynamics");

    let mut kstr = KeystrokeLogger::new();
    kstr.set_events_limit(5000);

    // read a user keystrokes from Stdin char by char
    let mut ch = [0];
    while let Ok(_) = stdin().read(&mut ch) {
        println!("CHAR {:?}", ch[0] as char);
        kstr.add_keystroke(ch[0] as char);

        if ch[0] == 'Q' as u8 {
            break;
        }
    }

    let stats = kstr.compute_digraph_statistics();
    for (k, v) in stats.iter() {
        println!("{:?}: mean({}), std({})", k, v.mean, v.std);
    }
}
