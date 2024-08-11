use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    pub static ref MEMORY_SEGMENTS: HashMap<&'static str, &'static str> = HashMap::from([
        ("sp", "SP"),
        ("local", "LCL"),
        ("argument", "ARG"),
        ("this", "THIS"),
        ("that", "THAT"),
        ("temp", "5"),
        ("static", "STATIC"),
        ("pointer", "POINTER"),
        ("constant", "CONSTANT")
    ]);
}
