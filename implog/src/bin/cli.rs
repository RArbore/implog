use std::io::{Read, Result, stdin};

use implog::ast::NameInterner;
use implog::grammar::ProgramParser;
use implog::interpret::Environment;

pub fn main() -> Result<()> {
    let mut interner = NameInterner::new();
    let mut program = String::new();
    stdin().read_to_string(&mut program)?;
    let ast = ProgramParser::new().parse(&mut interner, &program).unwrap();

    let mut env = Environment::new();
    env.interpret(&ast);

    Ok(())
}
