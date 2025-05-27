use clap::Parser;
use rtree::{Args, run};

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    run(args)
}
