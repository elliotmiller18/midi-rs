use std::path::Path;

mod parse;
mod bits;

fn main() {
    let (header, events) = parse::parse(Path::new("./tracks/The_Living_Tombstone_-_Five_Nights_at_Freddy's_2_Song_(It's_Been_so_long).mid")).unwrap();
}
