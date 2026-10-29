use std::fmt;

/// Everything a command can fail with. `Display` is the one line the user sees.
#[derive(Debug)]
pub enum Error {
    /// The server could not be reached at all (refused, DNS, timeout, TLS...).
    Unreachable { server: String, cause: String },
    /// The server answered `401`: no session, or it is invalid or expired.
    Unauthorized,
    /// The server answered with an unexpected status; `message` is its `error` field, if any.
    Server { status: u16, message: String },
    /// Any other failure, already phrased for the user.
    Msg(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreachable { server, cause } => {
                write!(f, "cannot reach server at {server}: {cause}")
            }
            Error::Unauthorized => write!(f, "session missing or expired; run polykov login"),
            Error::Server { status, message } if message.is_empty() => {
                write!(f, "server returned {status}")
            }
            Error::Server { status, message } => write!(f, "server returned {status}: {message}"),
            Error::Msg(msg) => f.write_str(msg),
        }
    }
}
