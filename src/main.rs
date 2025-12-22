use std::path::Path;

mod parse;
mod bits;

fn main() {
    println!("Hello, world!");
    parse::parse(Path::new("./tracks/R_Kelly_-_I_Believe_I_Can_Fly.mid")).unwrap();
    println!("Goodbye, world!");
}
