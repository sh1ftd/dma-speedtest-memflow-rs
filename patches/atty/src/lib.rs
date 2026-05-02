//! Drop-in `atty` compatibility using [`std::io::IsTerminal`] instead of the crates.io crate.

use std::io::{self, IsTerminal};

/// Stream to test whether it's a TTY.
#[derive(Clone, Copy, Debug)]
pub enum Stream {
    Stdout,
    Stderr,
    Stdin,
}

/// Returns true if `stream` is a terminal.
#[must_use]
pub fn is(stream: Stream) -> bool {
    match stream {
        Stream::Stdout => io::stdout().is_terminal(),
        Stream::Stderr => io::stderr().is_terminal(),
        Stream::Stdin => io::stdin().is_terminal(),
    }
}
