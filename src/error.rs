use std::fmt::Display;

use crate::sys::RTCError;

impl Display for RTCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RTCError::NONE => write!(f, "No error"),
            RTCError::UNKNOWN => write!(f, "Unknown error"),
            RTCError::INVALID_ARGUMENT => write!(f, "Invalid argument"),
            RTCError::INVALID_OPERATION => {
                write!(f, "Invalid operation.")
            }
            RTCError::OUT_OF_MEMORY => write!(f, "Out of memory"),
            RTCError::UNSUPPORTED_CPU => write!(f, "Unsupported CPU"),
            RTCError::CANCELLED => write!(
                f,
                "Cancelled by a memory monitor callback or progress monitor callback function"
            ),
        }
    }
}
