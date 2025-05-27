use clap_markdown::help_markdown;
use mytree::Args;

fn main() {
    let markdown = help_markdown::<Args>();
    std::fs::create_dir_all("docs").unwrap();
    std::fs::write("docs/CLI.md", markdown).unwrap();
    println!("Generated docs/CLI.md");
}
