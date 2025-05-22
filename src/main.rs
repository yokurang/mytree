use clap::Parser;
use rtree::{run, Args};

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    run(args)
}
