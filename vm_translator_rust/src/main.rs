use std::io::{BufReader, Seek};

mod memory_segments;
mod parser;
mod writer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        return Err("cargo run <filename>".into());
    }

    let filename = &args[1];

    let file = std::fs::File::open(filename)?;
    let mut reader = BufReader::new(file);
    let num_of_call_instruction = parser::parse_file(&mut reader)?;
    reader.rewind()?;
    writer::write_hack_instruction_from_jvm_instruction_into_file(
        num_of_call_instruction,
        reader,
        filename,
    )?;
    Ok(())
}
