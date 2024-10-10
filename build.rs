use std::env;
use std::path::PathBuf;
use std::fs;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("out path is {}", out_path.display());
    fs::copy("./partitions.csv", out_path.join("partitions.csv")).unwrap();
    
    embuild::espidf::sysenv::output();
}
