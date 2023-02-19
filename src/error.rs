use std::fmt::Display;

use crate::sys::RTCError;

impl Display for RTCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RTCError::NONE => write!(f, "No error occurred."),
            RTCError::UNKNOWN => write!(f, "An unknown error has occurred."),
            RTCError::INVALID_ARGUMENT => write!(f, "An invalid argument was specified."),
            RTCError::INVALID_OPERATION => {
                write!(f, "The operation is not allowed for the specified object.")
            }
            RTCError::OUT_OF_MEMORY => write!(
                f,
                "There is not enough memory left to complete the operation."
            ),
            RTCError::UNSUPPORTED_CPU => write!(
                f,
                "The CPU is not supported as it does not support the lowest ISA Embree is \
                 compiled for."
            ),
            RTCError::CANCELLED => write!(
                f,
                "The operation got canceled by a memory monitor callback or progress monitor \
                 callback function."
            ),
        }
    }
}
