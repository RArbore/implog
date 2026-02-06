use lalrpop_util::lalrpop_mod;

pub mod assumption;
pub mod ast;
pub mod interner;
pub mod interpret;
pub mod table;

lalrpop_mod!(pub grammar);
