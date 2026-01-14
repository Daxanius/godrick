use clap::Parser;
use parse_frequency::Frequency;
use std::fs;
use std::io::{self, Read};

use godrick::{Error, LimbSpec, Program, Result};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The program / input file to run
    #[arg(value_name = "FILE")]
    file: Option<String>,

    /// The memory size in bytes
    #[arg(short, long, default_value = "1 MiB", value_parser = |s: &str| parse_size::parse_size(s))]
    memory: u64,

    /// The clock frequency of the program
    #[arg(short, long, default_value = "1 MHz")]
    frequency: Frequency,

    /// The limbs to graft (e.g. libfoo@0x1000)
    #[arg(short, long)]
    graft: Vec<LimbSpec>,
}

fn run() -> Result<()> {
    let args = Args::parse();

    let code: String = if let Some(file) = args.file {
        fs::read_to_string(file).map_err(Error::Io)?
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).map_err(Error::Io)?;
        buffer
    };

    let program = Program::new(&code)?;

    let mut engine = godrick::Engine::new(
        program,
        args.memory.try_into().expect("Invalid memory size"),
        args.frequency,
    );
    engine.run();

    println!("\nProgram finished.");
    println!("Program size: {} bytes", engine.get_program().len());
    println!("Memory size: {} bytes", args.memory);
    println!("Clock frequency: {}", args.frequency);
    println!("Grafted limbs: {}", args.graft.len());
    println!(
        "Instruction pointer: {}",
        engine.get_context().instruction_pointer
    );
    println!("Pointer: {}", engine.get_context().pointer);
    // println!("Stack size: {}", engine.get_context().stack.len());
    println!("Commands executed: {}", engine.get_commands_executed());

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
