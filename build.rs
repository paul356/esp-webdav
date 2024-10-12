use std::env;
use std::path::PathBuf;
use embuild::cmd::Cmd;

fn main() {
    embuild::espidf::sysenv::output();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let src_path = PathBuf::from(env::var("PWD").unwrap());
    let mut cp_cmd = Cmd::new("/usr/bin/cp");
    cp_cmd.arg(src_path.join("partitions.csv").to_str().unwrap()).arg(out_path.join("partitions.csv").to_str().unwrap());
    cp_cmd.run().unwrap();
}
