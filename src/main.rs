use clap::Parser;
use mytree::{run, Args};

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    run(args)
}
