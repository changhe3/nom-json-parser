mod parser;
mod repr;
mod utils;

pub use parser::parse;
pub use repr::{Json, JsonValue};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
