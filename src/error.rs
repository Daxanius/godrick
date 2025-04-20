use std::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Limb(String),
    Program(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error:\n {err}"),
            Error::Limb(err) => write!(f, "Limb error:\n {err}"),
            Error::Program(err) => write!(f, "Program error:\n {err}"),
        }
    }
}

impl std::error::Error for Error {}
