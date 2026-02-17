use lalrpop_util::lalrpop_mod;

pub mod ast;
pub mod interpret;
pub mod representation;

lalrpop_mod!(pub grammar);
