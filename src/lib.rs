#![allow(dead_code, unused_imports)]

#[macro_export]
macro_rules! assert_numerically_similar {
    ( $eps:expr, $x:expr, $y:expr ) => {
        { assert!(($x - $y).abs() <= $eps); }
    };
}

use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::{thread, time};
use std::collections::{HashMap, VecDeque};
use std::fmt::Display;
use std::hash::Hash;

extern crate statistical;
extern crate serde;
use serde::{Serialize, Serializer, Deserialize};

pub struct KeyEvent {
    timestamp_ms: u128,
    key: char,
}

pub type Digraph = (char, char);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DigraphStats {
    pub size_samples: usize,
    pub mean: f64,
    pub std: f64,
}

pub struct KeystrokeLogger {
    events: VecDeque<KeyEvent>,
    events_limit: Option<usize>,
}

impl KeystrokeLogger {
    pub fn new() -> KeystrokeLogger {
        KeystrokeLogger { events: VecDeque::new(), events_limit: None }
    }

    pub fn add_key_event(&mut self, ev: KeyEvent) {
        self.events.push_back(ev);
        if let Some(limit) = self.events_limit {
            if self.events.len() > limit {
                for _ in 0..self.events.len()-limit {
                    self.events.pop_front();
                }
            }
        }
    }

    pub fn add_keystroke(&mut self, key: char) {
        let now = SystemTime::now();
        let ts = now.duration_since(UNIX_EPOCH).unwrap().as_millis();
        self.add_key_event(KeyEvent { timestamp_ms: ts, key });
    }

    pub fn set_events_limit(&mut self, limit: usize) {
        self.events_limit = Some(limit);
    }

    pub fn get_key_events(&self) -> &VecDeque<KeyEvent> {
        &self.events
    }

    pub fn clear_key_events(&mut self) {
        self.events.clear();
    }

    pub fn compute_digraph_statistics(&self) -> HashMap<Digraph, DigraphStats> {
        let mut samples: HashMap<Digraph, Vec<f64>> = HashMap::new();
        for i in 1..self.events.len() {
            let ev1 = &self.events[i-1];
            let ev2 = &self.events[i];

            let k = (ev1.key, ev2.key);
            let v = (ev2.timestamp_ms - ev1.timestamp_ms) as f64;
            match samples.get_mut(&k) {
                Some(arr) => { arr.push(v); },
                None => { samples.insert(k, vec![v]); },
            }
        }

        let mut stats: HashMap<Digraph, DigraphStats> = HashMap::new();
        for (k, v) in samples.iter() {
            if v.len() >= 2 {
                let mean = statistical::mean(v);
                let std = statistical::standard_deviation(v, Some(mean));
                stats.insert(*k, DigraphStats { size_samples: v.len(), mean, std });
            }
        }

        stats
    }

    pub fn serialize_digraph_statistics(stats: &HashMap<Digraph, DigraphStats>) -> String {
        let mut str_keyed_map: HashMap<String, String> = HashMap::new();
        for (k, v) in stats.iter() {
            let key = format!("{}-{}", k.0, k.1);
            let value = serde_json::to_string(v).unwrap();
            str_keyed_map.insert(key, value);
        }
        serde_json::to_string(&str_keyed_map).unwrap()
    }

    pub fn deserialize_digraph_statistics(serialized: &str) -> HashMap<Digraph, DigraphStats> {
        let mut stats: HashMap<Digraph, DigraphStats> = HashMap::new();
        let str_keyed_map: HashMap<String, String> = serde_json::from_str(serialized).unwrap();

        for (k, v) in str_keyed_map.iter() {
            let vec_digraph: Vec<&str> = k.split('-').collect();
            let vec_k1: Vec<char> = vec_digraph[0].chars().collect();
            let vec_k2: Vec<char> = vec_digraph[1].chars().collect();

            let digraph = (vec_k1[0], vec_k2[0]);
            let digraph_stats: DigraphStats = serde_json::from_str(v).unwrap();

            stats.insert(digraph, digraph_stats);
        }
        stats
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeynomeAuthenticatorDiffParams {
    pub dispersion: bool,
    pub min_instances: u32,
    pub max_comparisons: u32,
}

pub struct KeynomeAuthenticator<'a, 'b> {
    pub diff_base: f64,
    pub multiplier: f64,
    pub diff_params: &'a KeynomeAuthenticatorDiffParams,
    pub stats: &'b HashMap<Digraph, DigraphStats>,
}

impl<'a, 'b> KeynomeAuthenticator<'a, 'b> {
    pub fn new(diff_base: f64, multiplier: f64, diff_params: &'a KeynomeAuthenticatorDiffParams,
               stats: &'b HashMap<Digraph, DigraphStats>) -> KeynomeAuthenticator<'a, 'b> {
        KeynomeAuthenticator { diff_base, multiplier, diff_params, stats }
    }

    pub fn compute_diff(
        stats_profile: &HashMap<Digraph, DigraphStats>, stats_sample: &HashMap<Digraph, DigraphStats>,
        diff_params: &KeynomeAuthenticatorDiffParams) -> f64 {
        
        let mut diff: f64 = 0.0;
        let mut n_comparisons: u32 = 0;

        for (k, v) in stats_profile.iter() {
            if v.size_samples < diff_params.min_instances as usize {
                continue;
            }

            if let Some(vs) = stats_sample.get(k) {
                diff = diff + ((v.mean - vs.mean).abs() / (if diff_params.dispersion { 0.001 + v.std } else { 1.0 }));
                n_comparisons = n_comparisons + 1;
            }

            if n_comparisons >= diff_params.max_comparisons {
                break;
            }
        }
        diff
    }

    pub fn compute_diff_base(
        events: &VecDeque<KeyEvent>, n_profile: usize, n_sample: usize,
        diff_params: &KeynomeAuthenticatorDiffParams) -> Option<f64> {

        if n_profile > events.len() {
            return None;
        } else if n_profile % n_sample != 0 {
            return None;
        }

        let vec_events: Vec<&KeyEvent> = events.into_iter().collect();
        let mut kstr = KeystrokeLogger::new();
        for ev in &vec_events[(events.len()-n_profile)..] {
            kstr.add_key_event(KeyEvent { timestamp_ms: ev.timestamp_ms, key: ev.key });
        }
        let stats = kstr.compute_digraph_statistics();

        let mut diff_base: f64 = 0.0;
        for i in 0..n_profile/n_sample {
            let idx_start = (events.len() - n_profile) + n_sample * i;
            let idx_end = (events.len() - n_profile) + n_sample * (i + 1);

            let mut kstr_sample = KeystrokeLogger::new();
            for ev in &vec_events[idx_start..idx_end] {
                kstr_sample.add_key_event(KeyEvent { timestamp_ms: ev.timestamp_ms, key: ev.key });
            }

            let stats_sample = kstr_sample.compute_digraph_statistics();
            diff_base = diff_base + KeynomeAuthenticator::compute_diff(&stats, &stats_sample, diff_params);
        }

        diff_base = diff_base / (n_profile / n_sample) as f64;
        Some(diff_base)
    }

    pub fn authenticate(&self, events: &VecDeque<KeyEvent>) -> bool {
        true
    }
}

pub struct UserProfile {
    pub n_profile: u32,
    pub n_sample: u32,
    pub diff_base: f64,
    pub diff_params: KeynomeAuthenticatorDiffParams,
    pub stats: HashMap<Digraph, DigraphStats>,
}

impl UserProfile {
    pub fn new(n_profile: u32, n_sample: u32, diff_base: f64,
           _diff_params: &KeynomeAuthenticatorDiffParams,
           _stats: &HashMap<Digraph, DigraphStats>) -> UserProfile {

        let diff_params = (*_diff_params).clone();
        let mut stats: HashMap<Digraph, DigraphStats> = HashMap::new();
        for (k, v) in _stats.iter() {
            stats.insert(*k, (*v).clone());
        }
        UserProfile { n_profile, n_sample, diff_base, diff_params, stats }
    }

    pub fn serialize(&self) -> String {
        let mut obj = HashMap::new();

        obj.insert("n_profile", format!("{}", self.n_profile));
        obj.insert("n_sample", format!("{}", self.n_sample));
        obj.insert("diff_base", format!("{}", self.diff_base));
        obj.insert("diff_params", serde_json::to_string(&self.diff_params).unwrap());
        obj.insert("stats", KeystrokeLogger::serialize_digraph_statistics(&self.stats));

        serde_json::to_string(&obj).unwrap()
    }

    pub fn deserialize(serialized: &str) -> UserProfile {
        let str_keyed_map: HashMap<String, String> = serde_json::from_str(serialized).unwrap();

        let n_profile: u32 = str_keyed_map.get("n_profile").unwrap().parse().unwrap();
        let n_sample: u32 = str_keyed_map.get("n_sample").unwrap().parse().unwrap();
        let diff_base: f64 = str_keyed_map.get("diff_base").unwrap().parse().unwrap();
        let diff_params: KeynomeAuthenticatorDiffParams = serde_json::from_str(
            &str_keyed_map.get("diff_params").unwrap()).unwrap();
        let stats: HashMap<Digraph, DigraphStats> = KeystrokeLogger::deserialize_digraph_statistics(
            &str_keyed_map.get("stats").unwrap());

        UserProfile { n_profile, n_sample, diff_base, diff_params, stats }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_logger_instantiation() {
        let _ = KeystrokeLogger::new();
    }

    #[test]
    fn keystroke_logger_preserving_keystroke_history() {
        let mut kstr = KeystrokeLogger::new();
        kstr.add_keystroke('a');
        kstr.add_keystroke('b');
        kstr.add_keystroke('c');

        let events = kstr.get_key_events();
        let keystrokes: Vec<char> = events.into_iter().map(|e| e.key).collect();
        assert_eq!(keystrokes, vec!['a', 'b', 'c']);
    }

    #[test]
    fn keystroke_logger_events_limit() {
        let mut kstr = KeystrokeLogger::new();
        kstr.set_events_limit(123);

        for _ in 0..200 {
            kstr.add_keystroke('a');
        }
        assert_eq!(kstr.get_key_events().len(), 123);
    }

    #[test]
    fn keystroke_logger_time_difference() {
        let delays: Vec<u64> = vec![12, 34, 56];

        let mut kstr = KeystrokeLogger::new();
        kstr.add_keystroke('a');
        thread::sleep(Duration::from_millis(delays[0]));
        kstr.add_keystroke('b');
        thread::sleep(Duration::from_millis(delays[1]));
        kstr.add_keystroke('c');
        thread::sleep(Duration::from_millis(delays[2]));
        kstr.add_keystroke('d');

        let events = kstr.get_key_events();
        assert_numerically_similar!(1.0, (events[1].timestamp_ms - events[0].timestamp_ms) as f64, delays[0] as f64);
        assert_numerically_similar!(1.0, (events[2].timestamp_ms - events[1].timestamp_ms) as f64, delays[1] as f64);
        assert_numerically_similar!(1.0, (events[3].timestamp_ms - events[2].timestamp_ms) as f64, delays[2] as f64);
    }

    #[test]
    fn keystroke_logger_digraph_statistics() {
        let mut kstr = KeystrokeLogger::new();

        // a-b digraphs, diffs = [1000, 2000, 3000], mean = 2000.0, std = 1000.0
        // b-a digraphs, diffs = [1000, 1000], mean = 1000.0, std = 0.0
        kstr.add_key_event(KeyEvent { timestamp_ms: 10000, key: 'a' });
        kstr.add_key_event(KeyEvent { timestamp_ms: 11000, key: 'b' });

        kstr.add_key_event(KeyEvent { timestamp_ms: 12000, key: 'a' });
        kstr.add_key_event(KeyEvent { timestamp_ms: 14000, key: 'b' });

        kstr.add_key_event(KeyEvent { timestamp_ms: 15000, key: 'a' });
        kstr.add_key_event(KeyEvent { timestamp_ms: 18000, key: 'b' });

        // e-f digraphs, diffs = [500, 1000, 1500], mean = 1000.0, std = 500.0
        // f-e digraphs, diffs = [500, 2000], mean = 1250.0, std = 1060.66
        kstr.add_key_event(KeyEvent { timestamp_ms: 20000, key: 'e' });
        kstr.add_key_event(KeyEvent { timestamp_ms: 20500, key: 'f' });

        kstr.add_key_event(KeyEvent { timestamp_ms: 21000, key: 'e' });
        kstr.add_key_event(KeyEvent { timestamp_ms: 22000, key: 'f' });

        kstr.add_key_event(KeyEvent { timestamp_ms: 24000, key: 'e' });
        kstr.add_key_event(KeyEvent { timestamp_ms: 25500, key: 'f' });

        let stats = kstr.compute_digraph_statistics();

        assert_numerically_similar!(0.01, stats[&('a', 'b')].mean, 2000.0);
        assert_numerically_similar!(0.01, stats[&('a', 'b')].std, 1000.0);

        assert_numerically_similar!(0.01, stats[&('b', 'a')].mean, 1000.0);
        assert_numerically_similar!(0.01, stats[&('b', 'a')].std, 0.0);

        assert_numerically_similar!(0.01, stats[&('e', 'f')].mean, 1000.0);
        assert_numerically_similar!(0.01, stats[&('e', 'f')].std, 500.0);

        assert_numerically_similar!(0.01, stats[&('f', 'e')].mean, 1250.0);
        assert_numerically_similar!(0.01, stats[&('f', 'e')].std, 1060.66);
    }

    #[test]
    fn keystroke_logger_digraph_statistics_serialization() {
        let keystrokes = "The most likely way for the world to be destroyed, \
                          most experts agree, is by accident.  That's where we come in \
                          we're computer professionals.  We cause accidents. \
                          - Nathaniel Borenstein";

        let mut kstr = KeystrokeLogger::new();
        for c in keystrokes.chars() {
            kstr.add_keystroke(c);
        }

        let stats = kstr.compute_digraph_statistics();

        let serialized = KeystrokeLogger::serialize_digraph_statistics(&stats);
        let deserialized = KeystrokeLogger::deserialize_digraph_statistics(&serialized);

        let mut keys_orig: Vec<String> = stats.keys().map(|d| format!("{}-{}", d.0, d.1)).collect();
        let mut keys_new: Vec<String> = deserialized.keys().map(|d| format!("{}-{}", d.0, d.1)).collect();

        keys_orig.sort();
        keys_new.sort();
        assert_eq!(keys_orig, keys_new);
    }

    #[test]
    fn keynome_authenticator_compute_diff_base() {
        let mut events = VecDeque::new();

        // (Profile) For all 12 instances,

        // a-b digraphs
        // diffs = [1000, 2000, 3000, 500, 1000, 1500]
        // mean = 1500.0, std = 894.4272

        // b-a digraphs
        // diffs = [1000, 1000, 2000, 500, 2000]
        // mean = 1300.0, std = 670.8204

        // (Sample 1) For first 6 instances,

        // a-b digraphs
        // diffs = [1000, 2000, 3000]
        // mean = 2000.0, std = 1000.0

        // b-a digraphs
        // diffs = [1000, 1000]
        // mean = 1000.0, std = 0

        // (Sample 2) For last 6 instances,

        // a-b digraphs
        // diffs = [500, 1000, 1500]
        // mean = 1000.0, std = 500.0

        // b-a digraphs
        // diffs = [500, 2000]
        // mean = 1250.0, std = 1060.6602

        events.push_back(KeyEvent { timestamp_ms: 10000, key: 'a' });
        events.push_back(KeyEvent { timestamp_ms: 11000, key: 'b' });

        events.push_back(KeyEvent { timestamp_ms: 12000, key: 'a' });
        events.push_back(KeyEvent { timestamp_ms: 14000, key: 'b' });

        events.push_back(KeyEvent { timestamp_ms: 15000, key: 'a' });
        events.push_back(KeyEvent { timestamp_ms: 18000, key: 'b' });

        events.push_back(KeyEvent { timestamp_ms: 20000, key: 'a' });
        events.push_back(KeyEvent { timestamp_ms: 20500, key: 'b' });

        events.push_back(KeyEvent { timestamp_ms: 21000, key: 'a' });
        events.push_back(KeyEvent { timestamp_ms: 22000, key: 'b' });

        events.push_back(KeyEvent { timestamp_ms: 24000, key: 'a' });
        events.push_back(KeyEvent { timestamp_ms: 25500, key: 'b' });

        let diff_params = KeynomeAuthenticatorDiffParams {
            dispersion: false,
            min_instances: 1,
            max_comparisons: 100,
        };

        // Profile <=> Sample 1
        // (a-b) abs(1500 - 2000) = 500
        // (b-a) abs(1300 - 1000) = 300
        // diff = 800

        // Profile <=> Sample 2
        // (a-b) abs(1500 - 1000) = 500
        // (b-a) abs(1300 - 1250) = 50
        // diff = 550

        // diff_base = (800 + 550) / 2 = 675

        let diff_base = KeynomeAuthenticator::compute_diff_base(&events, 12, 6, &diff_params).unwrap();
        assert_eq!(diff_base, 675.0);
    }
}
