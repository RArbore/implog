use std::io::{Read, Result, stdin};

use implog::assumption::DNFAssumption;
use implog::ast::{NameInterner, check_range_restricted};
use implog::grammar::ProgramParser;
use implog::interpret::Environment;

pub fn main() -> Result<()> {
    let mut interner = NameInterner::new();
    let mut program = String::new();
    stdin().read_to_string(&mut program)?;
    let ast = ProgramParser::new().parse(&mut interner, &program).unwrap();
    for stmt in &ast {
        assert!(check_range_restricted(stmt));
    }

    let mut env = Environment::<DNFAssumption>::new(interner);
    env.interpret(&ast);

    Ok(())
}
