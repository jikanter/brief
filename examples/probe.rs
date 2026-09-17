//! Temporary probe: which plain scalars does serde-saphyr refuse as String?
use std::collections::BTreeMap;

fn main() {
    for y in [
        "stack: [no]",
        "stack: [yes]",
        "stack: [on]",
        "stack: [off]",
        "stack: [true]",
        "stack: [false]",
        "stack: [null]",
        "stack: [~]",
        "stack: [Null]",
        "stack: [NULL]",
        "stack: [3.12]",
        "stack: [2026-04-11]",
        "stack: ['null']",
    ] {
        let r: Result<BTreeMap<String, Vec<String>>, _> = serde_saphyr::from_str(y);
        match r {
            Ok(m) => println!("{:24} OK  {:?}", y, m.get("stack").unwrap()),
            Err(e) => println!(
                "{:24} ERR {}",
                y,
                e.to_string().lines().next().unwrap_or("")
            ),
        }
    }
}
