use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::memory_segments::MEMORY_SEGMENTS;

const INSTRUCTIONS: [&str; 14] = [
    "push", "pop", "add", "sub", "eq", "lt", "gt", "and", "or", "not", "neg", "if-goto", "goto",
    "label",
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
            Ok(_) if line.starts_with('/') => {}
            Ok(_) => {
                let mut is_comment = false;
                let mut splitted_instruction = line.split_whitespace().filter(|str| {
                    if is_comment.eq(&true) {
                        false
                    } else if str.starts_with('/') {
                        is_comment = true;
                        false
                    } else {
                        true
                    }
                });
                if let Some(instruction) = splitted_instruction.next() {
                    match instruction {
                        "label" | "if-goto" => {
                            if splitted_instruction.next().is_none() {
                                return Err(Error::WrongSyntax {
                                    expected: { format!("{} <destination>", instruction) },
                                });
                            }
                        }
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
                }
            }
            Err(e) => return Err(Error::Io(e)),
        }
        line.clear();
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
