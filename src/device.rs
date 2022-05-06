use std::ffi::CString;
use std::fmt::{Display, Formatter, Error, format};
use std::ptr;
use std::sync::Arc;

use crate::sys::*;

pub struct Device {
    pub(crate) handle: RTCDevice,
}

impl Device {
    pub fn new() -> Arc<Device> {
        // Set the flush zero and denormals modes from Embrees's perf. recommendations
        // https://embree.github.io/api.html#performance-recommendations
        // Though, in Rust I think we just call the below function to do both
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64;
            x86_64::_MM_SET_FLUSH_ZERO_MODE(x86_64::_MM_FLUSH_ZERO_ON);
        }

        Arc::new(Device {
            handle: unsafe { rtcNewDevice(ptr::null()) },
        })
    }

    pub fn debug() -> Arc<Device> {
        let cfg = CString::new("verbose=4").unwrap();
        Arc::new(Device {
            handle: unsafe { rtcNewDevice(cfg.as_ptr()) },
        })
    }

    pub fn with_config(config: Config) -> Arc<Device> {
        let cfg = config.to_c_string();
        Arc::new(Device {
            handle: unsafe { rtcNewDevice(cfg.as_ptr()) },
        })
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseDevice(self.handle);
        }
    }
}

unsafe impl Sync for Device {}

/// Instruction Set Architecture.
#[derive(Debug, Clone, Copy)]
pub enum Isa {
    Sse2,
    Sse4_2,
    Avx,
    Avx2,
    Avx512,
}

impl Display for Isa {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Isa::Sse2 => write!(f, "sse2"),
            Isa::Sse4_2 => write!(f, "sse4.2"),
            Isa::Avx => write!(f, "avx"),
            Isa::Avx2 => write!(f, "avx2"),
            Isa::Avx512 => write!(f, "avx512"),
        }
    }
}

/// Frequency level of the application.
#[derive(Debug, Clone, Copy)]
pub enum FrequencyLevel {
    /// Run at highest frequency.
    Simd128,

    /// Run at AVX2-heavy frequency level.
    Simd256,

    /// Run at AVX512-heavy frequency level.
    Simd512,
}

impl Display for FrequencyLevel {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            FrequencyLevel::Simd128 => write!(f, "simd128"),
            FrequencyLevel::Simd256 => write!(f, "simd256"),
            FrequencyLevel::Simd512 => write!(f, "simd512"),
        }
    }
}

/// Embree device configuration.
pub struct Config {
    /// The number of build threads. A value of 0 enables all detected hardware
    /// threads. By default all hardware threads are used.
    pub threads: u32,

    /// The number of user threads that can be used to join and participate in a
    /// scene commit using `rtcJoinCommitScene`.
    pub user_threads: u32,

    /// Whether build threads are affinitized to hardware threads. This is disabled
    /// by default on standard CPUs, and enabled by default on Xeon Phi Processors.
    pub set_affinity: bool,

    /// When enabled, the build threads are started upfront. Useful for benchmarking
    /// to exclude thread creation time. This is disabled by default.
    pub start_threads: bool,

    /// ISA selection. By default the ISA is chosen automatically.
    pub isa: Isa,

    /// Configures the automated ISA selection to use maximally the specified ISA.
    pub max_isa: Isa,

    /// Enables or disables usage of huge pages. Enabled by default under Linux but
    /// disabled by default on Windows and macOS.
    pub hugepages: bool,

    /// Enables or disables the SeLockMemoryPrivilege privilege which is required to
    /// use huge pages on Windows. This option has only effect on Windows and is ignored
    /// on other platforms.
    pub enable_selockmemoryprivilege: bool,

    /// Verbosity of the output [0, 1, 2, 3]. No output when set to 0. The higher the
    /// level, the more the output. By default the output is set to 0.
    pub verbose: u32,

    /// Frequency level the application want to run on. See [`FrequencyLevel`].
    /// When some frequency level is specified, Embree will avoid doing optimizations
    /// that may reduce the frequency level below the level specified.
    pub frequency_level: FrequencyLevel,
}

impl Config {
    /// Converts the configuration to a C string.
    pub fn to_c_string(&self) -> CString {
        let formated = format!("threads={},verbose={},set_affinity={},start_threads={},\
        isa={},max_isa={},hugepages={},enable_selockmemoryprivilege={},frequency_level={}",
                               self.threads, self.verbose, self.set_affinity as u32,
                               self.start_threads as u32, self.isa, self.max_isa,
                               self.hugepages as u32, self.enable_selockmemoryprivilege as u32,
                               self.frequency_level).into_bytes();
        unsafe {
            CString::from_vec_unchecked(formated)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            threads: 0,
            user_threads: 0,
            set_affinity: false,
            start_threads: false,
            isa: Isa::Sse2,
            max_isa: Isa::Avx512,
            hugepages: if cfg!(target_os = "linux") {
                true
            } else {
                false
            },
            enable_selockmemoryprivilege: false,
            verbose: 0,
            frequency_level: FrequencyLevel::Simd256,
        }
    }
}
