mod repr;
mod parser;
mod utils;

pub use repr::{Json, JsonValue};
pub use parser::parse;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
