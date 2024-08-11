use std::{
    fmt::Write,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter},
    num::ParseIntError,
    path::{Path, PathBuf},
    str::SplitWhitespace,
};

use crate::memory_segments::MEMORY_SEGMENTS;

const DEFAULT_CAPACITY: usize = 100usize;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    ParseIntError(ParseIntError),
}

struct Writer<'a> {
    hack_instruction: String,
    label_count: usize,
    writer: BufWriter<File>,
    reader: BufReader<File>,
    filename_without_extendion: &'a str,
}

impl<'a> Writer<'a> {
    fn new(reader: BufReader<File>, writer: BufWriter<File>, filename: &'a str) -> Self {
        Self {
            hack_instruction: String::with_capacity(DEFAULT_CAPACITY),
            label_count: 0,
            writer,
            reader,
            filename_without_extendion: filename,
        }
    }

    fn load_address_register(&mut self, ram_address: &str) -> &mut Self {
        let _ = writeln!(self.hack_instruction, "@{}", ram_address);
        self
    }

    fn load_static_in_address_register(&mut self, filename: &str, nb: &str) -> &mut Self {
        let _ = writeln!(self.hack_instruction, "@{}.{}", filename, nb);
        self
    }

    fn assign_value_to_selected_register(
        &mut self,
        selected_register: &str,
        value_to_assing: &str,
    ) -> &Self {
        let _ = writeln!(
            self.hack_instruction,
            "{}={}",
            selected_register, value_to_assing
        );
        self
    }

    fn load_pointee_address_into_address_register(&mut self) -> &mut Self {
        self.assign_value_to_selected_register("A", "M");
        self
    }

    fn set_register_d_to_value_in_pointee(&mut self) -> &mut Self {
        self.assign_value_to_selected_register("D", "M");
        self
    }

    fn set_pointee_value_to_value_in_register_d(&mut self) -> &mut Self {
        self.assign_value_to_selected_register("M", "D");
        self
    }

    fn decrement_address_register_by_pointee_value_minus_one(&mut self) -> &mut Self {
        self.assign_value_to_selected_register("A", "M-1");
        self
    }

    fn increment_address_register_by_pointee_value_plus_one(&mut self) -> &mut Self {
        self.assign_value_to_selected_register("A", "M+1");
        self
    }

    fn load_pointee_value_into_address_register_and_set_pointee_value_into_register_d(
        &mut self,
    ) -> &mut Self {
        self.load_pointee_address_into_address_register()
            .set_register_d_to_value_in_pointee();
        self
    }

    fn load_pointee_value_into_address_register_and_set_register_d_value_into_pointee(
        &mut self,
    ) -> &mut Self {
        self.load_pointee_address_into_address_register()
            .set_pointee_value_to_value_in_register_d();
        self
    }

    fn load_and_increment_stack_pointer(&mut self) -> &mut Self {
        self.load_address_register("SP")
            .assign_value_to_selected_register("M", "M+1");
        self
    }

    fn load_and_decrement_stack_pointer(&mut self) -> &mut Self {
        self.load_address_register("SP")
            .assign_value_to_selected_register("M", "M-1");
        self
    }

    fn decrement_selected_register(&mut self) -> &mut Self {
        self.assign_value_to_selected_register("M", "M-1");
        self
    }

    fn increment_selected_register(&mut self) -> &mut Self {
        self.assign_value_to_selected_register("M", "M+1");
        self
    }

    fn write_hack_instruction_to_file(&mut self) -> Result<(), Error> {
        std::io::Write::write_all(&mut self.writer, self.hack_instruction.as_bytes())
            .map_err(Error::Io)?;
        self.hack_instruction.clear();
        Ok(())
    }

    fn convert_single_operand_instruction_to_hack_instruction_set(
        &mut self,
        hack_instruction: &str,
    ) {
        self.load_address_register("SP")
            .decrement_address_register_by_pointee_value_minus_one()
            .assign_value_to_selected_register("M", hack_instruction);
    }

    fn convert_double_operand_instruction_to_hack_instruction_set(
        &mut self,
        hack_instruction: &str,
    ) {
        self.load_and_decrement_stack_pointer()
            .load_pointee_value_into_address_register_and_set_pointee_value_into_register_d()
            .load_address_register("SP")
            .decrement_address_register_by_pointee_value_minus_one()
            .assign_value_to_selected_register("M", hack_instruction);
    }

    fn convert_compare_instruction_to_hack_instruction_set(&mut self, hack_instruction: &str) {
        let label_name = format!("LABEL.{}", self.label_count);
        let parenthized_label = format!("({})", label_name);
        let _ = writeln!(
            self.hack_instruction,
            "@SP\nM=M-1\nA=M\nD=M\n@SP\nA=M-1\nD=M-D\nM=-1\n@{}\nD;{}\n@SP\nA=M-1\nM=0\n{}",
            label_name, hack_instruction, parenthized_label
        );
        self.label_count += 1;
    }

    fn convert_push_instruction_to_hack_instruction_set(
        &mut self,
        mut instruction_arguments: SplitWhitespace,
    ) {
        let memory_segments = instruction_arguments.next().unwrap();
        let offset = instruction_arguments.next().unwrap();

        let remaning_instruction = |writer: &mut Self| {
            writer
                .load_address_register("SP")
                .load_pointee_value_into_address_register_and_set_register_d_value_into_pointee()
                .load_and_increment_stack_pointer();
        };

        match memory_segments {
            "local" | "argument" | "this" | "that" => {
                let addr = MEMORY_SEGMENTS.get(memory_segments).unwrap();
                self.load_address_register(offset);
                self.assign_value_to_selected_register("D", "A");
                self.load_address_register(addr);
                self.assign_value_to_selected_register("A", "D+M");
                self.assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            "temp" => {
                let addr = MEMORY_SEGMENTS.get(memory_segments).unwrap();
                self.load_address_register(offset);
                self.assign_value_to_selected_register("D", "A");
                self.load_address_register(addr);
                self.assign_value_to_selected_register("A", "D+A");
                self.assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            "constant" => {
                self.load_address_register(offset)
                    .assign_value_to_selected_register("D", "A");
                remaning_instruction(self);
            }
            "pointer" => {
                if offset == "0" {
                    self.load_address_register("THIS");
                } else {
                    self.load_address_register("THAT");
                };
                self.assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            "static" => {
                self.load_static_in_address_register(self.filename_without_extendion, offset);
                self.assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            _ => unreachable!(),
        };
    }

    fn convert_pop_instruction_to_hack_instruction_set(
        &mut self,
        mut instruction_arguments: SplitWhitespace,
    ) {
        let memory_segments = instruction_arguments.next().unwrap();
        let ram_address = instruction_arguments.next().unwrap();

        let remaining_instruction = |writer: &mut Self| {
            writer
                .load_address_register("SP")
                .load_pointee_value_into_address_register_and_set_register_d_value_into_pointee()
                .load_and_decrement_stack_pointer()
                .load_pointee_value_into_address_register_and_set_pointee_value_into_register_d()
                .load_address_register("SP")
                .increment_address_register_by_pointee_value_plus_one()
                .load_pointee_value_into_address_register_and_set_register_d_value_into_pointee();
        };

        match memory_segments {
            "local" | "argument" | "this" | "that" => {
                let addr = MEMORY_SEGMENTS.get(memory_segments).unwrap();
                self.load_address_register(addr)
                    .assign_value_to_selected_register("D", "M");
                self.load_address_register(ram_address)
                    .assign_value_to_selected_register("D", "D+A");
                remaining_instruction(self);
            }
            "temp" => {
                let addr = MEMORY_SEGMENTS.get(memory_segments).unwrap();
                self.load_address_register(addr)
                    .assign_value_to_selected_register("D", "A");
                self.load_address_register(ram_address)
                    .assign_value_to_selected_register("D", "D+A");
                remaining_instruction(self);
            }
            "pointer" => {
                self.load_and_decrement_stack_pointer()
                    .load_pointee_value_into_address_register_and_set_pointee_value_into_register_d(
                    );
                let addr = if ram_address == "0" { "THIS" } else { "THAT" };
                self.load_address_register(addr)
                    .assign_value_to_selected_register("M", "D");
            }
            "static" => {
                self.load_static_in_address_register(self.filename_without_extendion, ram_address)
                    .assign_value_to_selected_register("D", "A");
                remaining_instruction(self);
            }
            _ => unreachable!(),
        };
    }

    fn execution(&mut self) -> Result<(), Error> {
        let mut line = String::new();
        loop {
            match self.reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let mut splitted_instruction = line.split_whitespace();
                    if let Some(instruction) = splitted_instruction.next() {
                        match instruction {
                            "push" => self.convert_push_instruction_to_hack_instruction_set(
                                splitted_instruction,
                            ),
                            "pop" => self.convert_pop_instruction_to_hack_instruction_set(
                                splitted_instruction,
                            ),
                            "add" => self
                                .convert_double_operand_instruction_to_hack_instruction_set("D+M"),
                            "sub" => self
                                .convert_double_operand_instruction_to_hack_instruction_set("M-D"),
                            "eq" => self.convert_compare_instruction_to_hack_instruction_set("JEQ"),
                            "lt" => self.convert_compare_instruction_to_hack_instruction_set("JLT"),
                            "gt" => self.convert_compare_instruction_to_hack_instruction_set("JGT"),
                            "and" => self
                                .convert_double_operand_instruction_to_hack_instruction_set("D&M"),
                            "or" => self
                                .convert_double_operand_instruction_to_hack_instruction_set("D|M"),
                            "neg" => self
                                .convert_single_operand_instruction_to_hack_instruction_set("-M"),
                            "not" => self
                                .convert_single_operand_instruction_to_hack_instruction_set("!M"),
                            _ => unreachable!(),
                        };
                        self.write_hack_instruction_to_file()?;
                        line.clear();
                    }
                }
                Err(e) => return Err(Error::Io(e)),
            }
        }

        Ok(())
    }
}

impl<'a> Drop for Writer<'a> {
    fn drop(&mut self) {
        let _ = std::io::Write::flush(&mut self.writer);
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

fn open_file(new_file_name: &str) -> Result<File, Error> {
    let mut path = PathBuf::from(new_file_name);
    path.set_extension("hack");
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(Error::Io)
}

pub fn write_hack_instruction_from_jvm_instruction_into_file(
    reader: BufReader<File>,
    filename: &str,
) -> Result<(), Error> {
    let new_file = open_file(filename)?;
    let filename_without_extension = Path::new(filename).file_stem().unwrap().to_str().unwrap();
    let mut stack_write = Writer::new(reader, BufWriter::new(new_file), filename_without_extension);
    stack_write.execution()?;
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_push_instruction() {}

    #[test]
    fn test_pop_instruction() {}

    #[test]
    fn test_eq_instruction() {}

    #[test]
    fn test_lt_instruction() {}

    #[test]
    fn test_gt_instruction() {}

    #[test]
    fn test_gte_instruction() {}

    #[test]
    fn test_lte_instruction() {}

    #[test]
    fn test_and_instruction() {}

    #[test]
    fn test_or_instruction() {}
}
