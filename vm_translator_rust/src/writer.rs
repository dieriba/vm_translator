use std::{
    collections::HashMap,
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

struct FunctionFrame {
    number_of_args: String,
    call: usize,
}

impl FunctionFrame {
    fn new(number_of_args: String) -> Self {
        FunctionFrame {
            number_of_args,
            call: 0,
        }
    }
}

struct Writer<'a> {
    hack_instruction: String,
    label_count: usize,
    writer: BufWriter<File>,
    reader: BufReader<File>,
    filename_without_extendion: &'a str,
    function_frames: FunctionFrame,
    current_function_executed: String,
}

impl<'a> Writer<'a> {
    fn new(reader: BufReader<File>, writer: BufWriter<File>, filename: &'a str) -> Self {
        Self {
            hack_instruction: String::with_capacity(DEFAULT_CAPACITY),
            label_count: 0,
            writer,
            reader,
            filename_without_extendion: filename,
            function_frames: FunctionFrame {
                number_of_args: String::new(),
                call: 0,
            },
            current_function_executed: String::new(),
        }
    }

    fn write_label(&mut self, label: &str) -> &mut Self {
        let _ = writeln!(self.hack_instruction, "({})", label);
        self
    }

    fn load_address_register(&mut self, ram_address: &str) -> &mut Self {
        let _ = writeln!(self.hack_instruction, "@{}", ram_address);
        self
    }

    fn load_static_in_address_register(&mut self, filename: &str, nb: &str) -> &mut Self {
        let _ = writeln!(self.hack_instruction, "@{}.{}", filename, nb);
        self
    }

    //check handle_return_instruction if you were to modify that function
    fn handle_call_instruction(&mut self, mut splitted_instruction: SplitWhitespace) {
        let function_name = splitted_instruction.next().unwrap();
        let return_address = format!(
            "{}$ret.{}",
            self.current_function_executed, self.function_frames.call
        );
        let number_of_args = splitted_instruction
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap()
            + 5;
        self.write_label(&return_address)
            .push_memory_segment_onto_stack("constant", &return_address)
            .push_memory_segment_onto_stack("argument", "0")
            .push_memory_segment_onto_stack("local", "0")
            .push_memory_segment_onto_stack("this", "0")
            .push_memory_segment_onto_stack("that", "0")
            .load_address_register(&number_of_args.to_string())
            .assign_value_to_selected_register("D", "A")
            .load_address_register("SP")
            .assign_value_to_selected_register("D", "M-D")
            .load_address_register("ARG")
            .assign_value_to_selected_register("M", "D")
            .jump_to_address(function_name);
        self.function_frames.call += 1;
    }

    fn handle_function_instruction(&mut self, mut splitted_instruction: SplitWhitespace) {
        let function_name = splitted_instruction.next().unwrap();
        let number_of_local_variables = splitted_instruction.next().unwrap();
        let loop_label = format!("LOOP_{}", function_name);
        let end_loop_label = format!("END_LOOP_{}", function_name);
        let i = "i";
        self.load_address_register("SP")
            .assign_value_to_selected_register("D", "M")
            .load_address_register("LCL")
            .assign_value_to_selected_register("M", "D")
            .load_address_register(number_of_local_variables)
            .assign_value_to_selected_register("D", "A")
            .load_address_register(&end_loop_label)
            .write_jump_instruction(None, Some("D"), "JEQ")
            .load_address_register(i)
            .assign_value_to_selected_register("M", "D")
            .write_label(&loop_label)
            .load_address_register("SP")
            .assign_value_to_selected_register("A", "M")
            .assign_value_to_selected_register("M", "0")
            .load_and_increment_stack_pointer()
            .load_and_decrement_register_by_one(i)
            .assign_value_to_selected_register("D", "M")
            .load_address_register(&loop_label)
            .write_jump_instruction(None, Some("D"), "JGT")
            .write_label(&end_loop_label);

        if self.current_function_executed != function_name {
            self.function_frames.call = 0;
        }

        self.current_function_executed = function_name.to_string();
    }

    fn restore_pointer(&mut self, memory_segments: &str) -> &mut Self {
        self.load_address_register("LCL")
            .assign_value_to_selected_register("DM", "M-1")
            .load_address_register(memory_segments)
            .assign_value_to_selected_register("M", "D")
    }

    fn handle_return_instruction(&mut self) {
        let return_address = "return_address";
        self.load_address_register("LCL")
            .assign_value_to_selected_register("D", "M")
            .load_address_register("5") //5 because 5 value were push onto the stack before the LCL pointer, and return addres is located at the top
            .assign_value_to_selected_register("A", "D-A")
            .assign_value_to_selected_register("D", "M")
            .load_address_register(return_address)
            .assign_value_to_selected_register("M", "D")
            .load_address_register("SP")
            .assign_value_to_selected_register("A", "M-1")
            .assign_value_to_selected_register("D", "M")
            .load_address_register("ARG")
            .assign_value_to_selected_register("M", "D")
            .assign_value_to_selected_register("D", "A")
            .load_address_register("SP")
            .assign_value_to_selected_register("M", "D+1")
            .restore_pointer("THAT")
            .restore_pointer("THIS")
            .restore_pointer("LCL")
            .restore_pointer("ARG")
            .jump_to_address(return_address);
    }

    fn assign_value_to_selected_register(
        &mut self,
        selected_register: &str,
        value_to_assing: &str,
    ) -> &mut Self {
        let _ = writeln!(
            self.hack_instruction,
            "{}={}",
            selected_register, value_to_assing
        );
        self
    }

    fn write_jump_instruction(
        &mut self,
        dest: Option<&str>,
        value: Option<&str>,
        instruction: &str,
    ) -> &mut Self {
        if let Some(dest) = dest {
            let _ = write!(self.hack_instruction, "{}=", dest);
        };
        if let Some(value) = value {
            let _ = write!(self.hack_instruction, "{}", value);
        }
        let _ = writeln!(self.hack_instruction, ";{}", instruction);
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

    fn load_and_decrement_register_by_one(&mut self, address: &str) -> &mut Self {
        self.load_address_register(address)
            .assign_value_to_selected_register("M", "M-1");
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

    fn write_hack_instruction_to_file(&mut self) -> Result<(), Error> {
        std::io::Write::write_all(&mut self.writer, self.hack_instruction.as_bytes())
            .map_err(Error::Io)?;
        self.hack_instruction.clear();
        Ok(())
    }

    fn jump_to_address(&mut self, address: &str) -> &mut Self {
        self.load_address_register(address)
            .write_jump_instruction(None, Some("0"), "JMP")
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
        self.load_and_decrement_stack_pointer()
            .load_pointee_value_into_address_register_and_set_pointee_value_into_register_d()
            .load_address_register("SP")
            .decrement_address_register_by_pointee_value_minus_one()
            .assign_value_to_selected_register("D", "M-D")
            .assign_value_to_selected_register("M", "-1")
            .load_address_register(&label_name)
            .write_jump_instruction(None, Some("D"), hack_instruction)
            .load_address_register("SP")
            .decrement_address_register_by_pointee_value_minus_one()
            .assign_value_to_selected_register("M", "0")
            .write_label(&label_name);
        self.label_count += 1;
    }

    fn push_memory_segment_onto_stack(&mut self, memory_segments: &str, offset: &str) -> &mut Self {
        let remaning_instruction = |writer: &mut Self| {
            writer
                .load_address_register("SP")
                .load_pointee_value_into_address_register_and_set_register_d_value_into_pointee()
                .load_and_increment_stack_pointer();
        };

        match memory_segments {
            "local" | "argument" | "this" | "that" => {
                let addr = MEMORY_SEGMENTS.get(memory_segments).unwrap();
                self.load_address_register(offset)
                    .assign_value_to_selected_register("D", "A")
                    .load_address_register(addr)
                    .assign_value_to_selected_register("A", "D+M")
                    .assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            "temp" => {
                let addr = MEMORY_SEGMENTS.get(memory_segments).unwrap();
                self.load_address_register(offset)
                    .assign_value_to_selected_register("D", "A")
                    .load_address_register(addr)
                    .assign_value_to_selected_register("A", "D+A")
                    .assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            "constant" => {
                self.load_address_register(offset)
                    .assign_value_to_selected_register("D", "A");
                remaning_instruction(self);
            }
            "pointer" => {
                let instruction = if offset == "0" { "THIS" } else { "THAT" };
                self.load_address_register(instruction)
                    .assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            "static" => {
                self.load_static_in_address_register(self.filename_without_extendion, offset)
                    .assign_value_to_selected_register("D", "M");
                remaning_instruction(self);
            }
            _ => unreachable!(),
        };
        self
    }

    fn pop_off_memory_segment_of_stack(&mut self, mut instruction_arguments: SplitWhitespace) {
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
                    .assign_value_to_selected_register("D", "M")
                    .load_address_register(ram_address)
                    .assign_value_to_selected_register("D", "D+A");
                remaining_instruction(self);
            }
            "temp" => {
                let addr = MEMORY_SEGMENTS.get(memory_segments).unwrap();
                self.load_address_register(addr)
                    .assign_value_to_selected_register("D", "A")
                    .load_address_register(ram_address)
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
                Ok(_) if line.starts_with('/') => {}
                Ok(_) => {
                    let mut splitted_instruction = line.split_whitespace();
                    if let Some(instruction) = splitted_instruction.next() {
                        match instruction {
                            "push" => {
                                self.push_memory_segment_onto_stack(
                                    splitted_instruction.next().unwrap(),
                                    splitted_instruction.next().unwrap(),
                                );
                            }
                            "pop" => self.pop_off_memory_segment_of_stack(splitted_instruction),
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
                            "label" => {
                                self.write_label(splitted_instruction.next().unwrap());
                            }
                            "if-goto" => {
                                let address = splitted_instruction.next().unwrap();
                                self.load_and_decrement_stack_pointer()
                                    .assign_value_to_selected_register("A", "M")
                                    .assign_value_to_selected_register("D", "M")
                                    .load_address_register(address)
                                    .write_jump_instruction(None, Some("D"), "JNE");
                            }
                            "goto" => {
                                self.jump_to_address(splitted_instruction.next().unwrap());
                            }
                            "call" => self.handle_call_instruction(splitted_instruction),
                            "function" => self.handle_function_instruction(splitted_instruction),
                            "return" => self.handle_return_instruction(),
                            _ => unreachable!(),
                        };
                        self.write_hack_instruction_to_file()?;
                    }
                }
                Err(e) => return Err(Error::Io(e)),
            }
            line.clear();
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
    path.set_extension("asm");
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
