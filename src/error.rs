use std::fmt::Display;

use crate::sys;

/// Thin wrapper around Embree's `RTCError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Error(sys::RTCError);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            sys::RTCError::NONE => write!(f, "No error"),
            sys::RTCError::UNKNOWN => write!(f, "Unknown error"),
            sys::RTCError::INVALID_ARGUMENT => write!(f, "Invalid argument"),
            sys::RTCError::INVALID_OPERATION => {
                write!(f, "Invalid operation.")
            }
            sys::RTCError::OUT_OF_MEMORY => write!(f, "Out of memory"),
            sys::RTCError::UNSUPPORTED_CPU => write!(f, "Unsupported CPU"),
            sys::RTCError::CANCELLED => write!(
                f,
                "Cancelled by a memory monitor callback or progress monitor callback function"
            ),
        }
    }
}

impl From<sys::RTCError> for Error {
    fn from(err: sys::RTCError) -> Self {
        Self(err)
    }
}
