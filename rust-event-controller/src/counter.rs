use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

// Record the counters to restart from the correct place.
pub fn record_counters(counter_file_name: &String) -> HashMap<String, i32> {
    let mut counter_map: HashMap<String, i32> = HashMap::new();

    // If the counter file doesn't exist yet (e.g. first run), return an empty map.
    if !Path::new(counter_file_name).exists() {
        println!(
            "Counter file '{}' not found. Starting fresh with empty counters.",
            counter_file_name
        );
        return counter_map;
    }

    // for each counter file, read each line and update counter map based on file name
    let file = match File::open(counter_file_name) {
        Ok(f) => f,
        Err(e) => {
            println!(
                "Warning: Could not open counter file '{}': {}. Starting with empty counters.",
                counter_file_name, e
            );
            return counter_map;
        }
    };

    let file_reader = BufReader::new(file);

    for file_line in file_reader.lines() {
        let line = match file_line {
            Ok(l) => l,
            Err(e) => {
                println!("Warning: Could not read line from counter file: {}", e);
                continue;
            }
        };

        let parts: Vec<&str> = line.split(" - ").collect();
        if parts.len() < 2 {
            println!("Warning: Skipping malformed counter line: '{}'", line);
            continue;
        }

        let f_counter = match parts[1].trim().parse::<i32>() {
            Ok(c) => c,
            Err(e) => {
                println!(
                    "Warning: Could not parse counter value '{}': {}. Skipping.",
                    parts[1], e
                );
                continue;
            }
        };

        let key = parts[0].to_string();
        if counter_map.contains_key(&key) {
            if counter_map.get(&key).unwrap() < &f_counter {
                counter_map.insert(key, f_counter);
            }
        } else {
            counter_map.insert(key, f_counter);
        }
    }

    counter_map
}

// Fetch location from config hashmap
pub fn fetch_file_counter(counter_map: &HashMap<String, i32>, f_name: &String) -> i32 {
    if counter_map.contains_key(f_name) {
        return *counter_map.get(f_name).unwrap();
    } else {
        return 0;
    }
}

