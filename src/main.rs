use std::path::Path;

mod parse;

fn main() {
    println!("Hello, world!");
    parse::parse(Path::new("/")).unwrap();
}
