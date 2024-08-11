use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::memory_segments::MEMORY_SEGMENTS;

const INSTRUCTIONS: [&str; 11] = [
    "push", "pop", "add", "sub", "eq", "lt", "gt", "and", "or", "not", "neg",
];

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    UnknownInstruction { instruction: String },
    UnknownMemorySegement { memory_segment: String },
    WrongSyntax { expected: String },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

pub fn parse_file(reader: &mut BufReader<File>) -> Result<(), Error> {
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let mut splitted_instruction = line.split_whitespace();
                if let Some(instruction) = splitted_instruction.next() {
                    match instruction {
                        "push" | "pop" => {
                            if let Some(memory_segment) = splitted_instruction.next() {
                                if memory_segment == "constant" && instruction == "pop" {
                                    return Err(Error::WrongSyntax {
                                        expected: "push constant <i> instead of pop constant <i>"
                                            .to_string(),
                                    });
                                } else if !MEMORY_SEGMENTS.contains_key(memory_segment) {
                                    return Err(Error::UnknownMemorySegement {
                                        memory_segment: memory_segment.to_string(),
                                    });
                                }
                                if splitted_instruction.next().is_none() {
                                    return Err(Error::WrongSyntax {
                                        expected: format!("{} <segments> <i>", instruction),
                                    });
                                }
                            }
                        }
                        instruction if !INSTRUCTIONS.contains(&instruction) => {
                            return Err(Error::UnknownInstruction {
                                instruction: instruction.to_string(),
                            })
                        }
                        _ => {}
                    }
                    line.clear();
                }
            }
            Err(e) => return Err(Error::Io(e)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_exist() -> Result<(), Error> {
        Ok(())
    }

    #[test]
    fn test_unvalid_file_syntax() -> Result<(), Error> {
        Ok(())
    }

    #[test]
    fn test_valid_file_syntax() -> Result<(), Error> {
        Ok(())
    }
}
