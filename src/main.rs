extern crate statemap;
use chrono::Utc;

fn main() {

    let h1 = "host1";
    let h2 = "host2";

    let s1 = "state1";
    let s2 = "state2";
    let s3 = "state3";

    let mut sm = statemap::Statemap::new("test", None, None);
    sm.set_state(h1, s1, None, Utc::now());
    sm.set_state(h2, s1, None, Utc::now());
    sm.set_state(h1, s2, None, Utc::now());
    sm.set_state(h2, s2, None, Utc::now());
    sm.set_state(h1, s3, None, Utc::now());

    for state in sm {
        println!("{}", state);
    }
}
