use lalrpop_util::lalrpop_mod;

pub mod assumption;
pub mod ast;
pub mod interpret;

lalrpop_mod!(pub grammar);
