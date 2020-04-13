extern crate serde_json;

mod statemap;

use chrono::Utc;

use statemap::*;

fn main() {

    let mut sm = Statemap::new("test".to_owned(), None, None);
    sm.set_state("my_host".to_owned(), "test0".to_owned(), None, Utc::now());
    sm.set_state("other_host".to_owned(), "test0".to_owned(), None, Utc::now());
    sm.set_state("other_host".to_owned(), "test1".to_owned(), None, Utc::now());

    for state in sm {
        println!("{}", state);
    }
}
