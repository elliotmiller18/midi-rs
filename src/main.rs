use std::path::Path;

mod parse;
mod bits;

fn main() {
    println!("Hello, world!");
    parse::parse(Path::new("/")).unwrap();
}
