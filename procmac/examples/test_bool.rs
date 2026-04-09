use procmac::Boolable;

#[derive(Debug, PartialEq, Boolable)]
enum Switch {
    Off = 0,
    On = 1,
}

#[derive(Debug, PartialEq, Boolable)]
enum Integrity {
    Corrupt = 0,
    Valid = 1,
}

fn main() {
    // Test: bool -> Enum
    let s_on = Switch::from(true);
    let s_off = Switch::from(false);

    assert_eq!(s_on, Switch::On);
    assert_eq!(s_off, Switch::Off);

    // Test: Enum -> bool
    let b_true: bool = Integrity::Valid.into();
    let b_false: bool = Integrity::Corrupt.into();

    assert!(b_true);
    assert!(!b_false);

    println!("All tests passed!");
}
