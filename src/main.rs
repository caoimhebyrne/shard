use std::{env, fs};

use crate::config::v1::ShardConfig;

mod config;

fn main() {
    let file_path = env::args().nth(1).expect("expected a file path to be passed as an arg");
    let contents = fs::read_to_string(file_path).unwrap();
    let config = ShardConfig::from_str(&contents).unwrap();
    println!("Config: {config:#?}");
}
