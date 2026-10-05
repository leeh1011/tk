pub mod error;
pub use error::{Error,Result};
pub mod store;
pub use store::Store;
pub mod note;
pub use note::Note;

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn print_test(){
        println!("{}", Error::EmptyNote);
        println!("{:?}", Error::EmptyNote);
    }
}