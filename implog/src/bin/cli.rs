use std::io::{Read, Result, stdin};

use implog::ast::check;
use implog::grammar::ProgramParser;
//use implog::interpret::Environment;

pub fn main() -> Result<()> {
    let mut program = String::new();
    stdin().read_to_string(&mut program)?;
    let ast = ProgramParser::new().parse(&program).unwrap();
    for stmt in &ast {
        assert!(check(stmt));
    }

    //let mut env = Environment::<DNFAssumption>::new(interner);
    //env.interpret(&ast);

    Ok(())
}
