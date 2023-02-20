use crate::{sys::*, Buffer, BufferSize, Error, Geometry, GeometryKind, Scene};
use std::{
    ffi::CString,
    fmt::{self, Display, Formatter},
    ptr,
};

/// Handle to an Embree device.
#[derive(Debug)]
pub struct Device {
    pub(crate) handle: RTCDevice,
}

impl Clone for Device {
    fn clone(&self) -> Self {
        unsafe { rtcRetainDevice(self.handle) }
        Self {
            handle: self.handle,
        }
    }
}

impl Device {
    pub fn new() -> Result<Device, Error> {
        enable_ftz_and_daz();
        let handle = unsafe { rtcNewDevice(ptr::null()) };
        if handle.is_null() {
            Err(unsafe { rtcGetDeviceError(ptr::null_mut()) })
        } else {
            let device = Device { handle };
            device.set_error_function(default_error_function);
            Ok(device)
        }
    }

    pub fn debug() -> Result<Device, Error> {
        let cfg = CString::new("verbose=4").unwrap();
        enable_ftz_and_daz();
        let handle = unsafe { rtcNewDevice(cfg.as_ptr()) };
        if handle.is_null() {
            Err(unsafe { rtcGetDeviceError(ptr::null_mut()) })
        } else {
            let device = Device { handle };
            device.set_error_function(default_error_function);
            Ok(device)
        }
    }

    pub fn with_config(config: Config) -> Result<Device, Error> {
        enable_ftz_and_daz();
        let cfg = config.to_c_string();
        let handle = unsafe { rtcNewDevice(cfg.as_ptr()) };
        if handle.is_null() {
            Err(unsafe { rtcGetDeviceError(ptr::null_mut()) })
        } else {
            let device = Device { handle };
            device.set_error_function(default_error_function);
            Ok(device)
        }
    }

    /// Register a callback function to be called when an error occurs.
    ///
    /// Only a single callback function can be registered per device,
    /// and further invocations overwrite the previously registered callback.
    ///
    /// The error code is also set if an error callback function is registered.
    ///
    /// Unregister with [`Device::unset_error_function`].
    ///
    /// # Arguments
    ///
    /// * `error_fn` - A callback function that takes an error code and a
    ///   message.
    ///
    /// When the callback function is invoked, it gets the error code of the
    /// occurred error, as well as a message of type `&'static str` that
    /// further describes the error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use embree::Device;
    /// let device = Device::new().unwrap();
    /// device.set_error_function(|error, msg| {
    ///     println!("Error: {:?} {}", error, msg);
    /// });
    /// ```
    pub fn set_error_function<F>(&self, error_fn: F)
    where
        F: FnMut(RTCError, &'static str),
    {
        let mut closure = error_fn;
        unsafe {
            rtcSetDeviceErrorFunction(
                self.handle,
                crate::callback::error_function_helper(&mut closure),
                &mut closure as *mut _ as *mut ::std::os::raw::c_void,
            );
        }
    }

    /// Disable the registered error callback function.
    pub fn unset_error_function(&self) {
        unsafe {
            rtcSetDeviceErrorFunction(self.handle, None, ptr::null_mut());
        }
    }

    /// Register a callback function to track memory consumption of the device.
    ///
    /// Only a single callback function can be registered per device, and
    /// further invocations overwrite the previously registered callback.
    ///
    /// Once registered, the Embree device will invoke the callback function
    /// before or after it allocates or frees important memory blocks. The
    /// callback function might get called from multiple threads
    /// concurrently.
    ///
    /// Unregister with [`Device::unset_memory_monitor_function`].
    ///
    /// # Arguments
    /// * `monitor_fn` - A callback function that takes two arguments:
    ///    * `bytes: isize` - The number of bytes allocated or deallocated
    /// (> 0 for allocations and < 0 for deallocations). The Embree `Device`
    ///   atomically accumulating `bytes` input parameter.
    ///    * `post: bool` - Whether the callback is invoked after the allocation
    ///      or deallocation took place.
    ///
    /// Embree will continue its operation normally when the callback function
    /// returns `true`. If `false` returned, Embree will cancel the current
    /// operation with `RTC_ERROR_OUT_OF_MEMORY` error code.
    /// Issuing multiple cancel requests from different threads is allowed.
    /// Cancelling will only happen when the callback was called for
    /// allocations (bytes > 0), otherwise the cancel request will be ignored.
    ///
    /// If a callback to cancel was invoked before the allocation happens (`post
    /// == false`), then the `bytes` parameter should not be accumulated, as
    /// the allocation will never happen. If the callback to cancel was
    /// invoked after the allocation happened (`post == true`), then
    /// the `bytes` parameter should be accumulated, as the allocation properly
    /// happened and a deallocation will later free that data block.
    ///
    /// # Example
    /// ```no_run
    /// use embree::Device;
    /// let device = Device::new().unwrap();
    /// device.set_memory_monitor_function(|bytes, post| {
    ///     if bytes > 0 {
    ///         println!("allocated {} bytes", bytes);
    ///     } else {
    ///         println!("deallocated {} bytes", -bytes);
    ///     };
    ///     true
    /// });
    /// ```
    pub fn set_memory_monitor_function<F>(&self, monitor_fn: F)
    where
        F: FnMut(isize, bool) -> bool,
    {
        let mut closure = monitor_fn;
        unsafe {
            rtcSetDeviceMemoryMonitorFunction(
                self.handle,
                crate::callback::memory_monitor_function_helper(&mut closure),
                &mut closure as *mut _ as *mut ::std::os::raw::c_void,
            );
        }
    }

    /// Disable the registered memory monitor callback function.
    pub fn unset_memory_monitor_function(&self) {
        unsafe {
            rtcSetDeviceMemoryMonitorFunction(self.handle, None, ptr::null_mut());
        }
    }

    /// Query properties of the device.
    ///
    /// # Arguments
    ///
    /// * `prop` - The property to query. See `RTCDeviceProp` for possible
    ///   values.
    ///
    /// # Returns
    ///
    /// An integer of type `isize`.
    pub fn get_property(&self, prop: RTCDeviceProperty) -> Result<isize, Error> {
        let ret = unsafe { rtcGetDeviceProperty(self.handle, prop) };
        if ret == 0 {
            Err(self.get_error())
        } else {
            Ok(ret)
        }
    }

    /// Query the error code of the device.
    ///
    /// Each thread has its own error code per device. If an error occurs when
    /// calling an API function, this error code is set to the occurred
    /// error if it stores no previous error. The [`Device::get_error`] function
    /// reads and returns the currently stored error and clears the error
    /// code. This assures that the returned error code is always the first
    /// error occurred since the last invocation of [`Device::get_error`].
    ///
    /// # Returns
    ///
    /// Error code encoded as [`Error`].
    pub fn get_error(&self) -> RTCError { unsafe { rtcGetDeviceError(self.handle) } }

    /// Creates a new scene bound to the device.
    pub fn create_scene(&self) -> Result<Scene, Error> { Scene::new(self.clone()) }

    /// Creates a new scene bound to the device with the given configuration.
    /// It's the same as calling [`Device::create_scene`] and then
    /// [`Scene::set_flags`].
    /// See [`SceneConfig`] for possible values.
    pub fn create_scene_with_flags(&self, flags: RTCSceneFlags) -> Result<Scene, Error> {
        Scene::new_with_flags(self.clone(), flags)
    }

    /// Creates a new data buffer.
    /// The created buffer is always aligned to 16 bytes.
    pub fn create_buffer(&self, size: usize) -> Result<Buffer, Error> {
        Buffer::new(self, BufferSize::new(size).unwrap())
    }

    /// Creates a [`Geometry`] object bound to the device without any
    /// buffers attached.
    pub fn create_geometry(&self, kind: GeometryKind) -> Result<Geometry<'static>, Error> {
        Geometry::new(self, kind)
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
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
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
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
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

    /// Whether build threads are affinitized to hardware threads. This is
    /// disabled by default on standard CPUs, and enabled by default on Xeon
    /// Phi Processors.
    pub set_affinity: bool,

    /// When enabled, the build threads are started upfront. Useful for
    /// benchmarking to exclude thread creation time. This is disabled by
    /// default.
    pub start_threads: bool,

    /// ISA selection. By default the ISA is chosen automatically.
    pub isa: Option<Isa>,

    /// Configures the automated ISA selection to use maximally the specified
    /// ISA.
    pub max_isa: Isa,

    /// Enables or disables usage of huge pages. Enabled by default under Linux
    /// but disabled by default on Windows and macOS.
    pub hugepages: bool,

    /// Enables or disables the SeLockMemoryPrivilege privilege which is
    /// required to use huge pages on Windows. This option has only effect
    /// on Windows and is ignored on other platforms.
    pub enable_selockmemoryprivilege: bool,

    /// Verbosity of the output [0, 1, 2, 3]. No output when set to 0. The
    /// higher the level, the more the output. By default the output is set
    /// to 0.
    pub verbose: u32,

    /// Frequency level the application want to run on. See [`FrequencyLevel`].
    /// When some frequency level is specified, Embree will avoid doing
    /// optimizations that may reduce the frequency level below the level
    /// specified.
    pub frequency_level: Option<FrequencyLevel>,
}

impl Config {
    /// Converts the configuration to a C string.
    pub fn to_c_string(&self) -> CString {
        let isa = self
            .isa
            .map(|isa| format!("isa={},", isa))
            .unwrap_or_default();
        let frequency_level = self
            .frequency_level
            .map(|frequency_level| format!("frequency_level={}", frequency_level))
            .unwrap_or_default();
        let formated = format!(
            "threads={},verbose={},set_affinity={},start_threads={},max_isa={},hugepages={},\
             enable_selockmemoryprivilege={},{}{}",
            self.threads,
            self.verbose,
            self.set_affinity as u32,
            self.start_threads as u32,
            self.max_isa,
            self.hugepages as u32,
            self.enable_selockmemoryprivilege as u32,
            isa,
            frequency_level
        )
        .into_bytes();
        unsafe { CString::from_vec_unchecked(formated) }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            threads: 0,
            user_threads: 0,
            set_affinity: false,
            start_threads: false,
            isa: None,
            max_isa: Isa::Avx512,
            hugepages: cfg!(target_os = "linux"),
            enable_selockmemoryprivilege: false,
            verbose: 0,
            frequency_level: None,
        }
    }
}

/// Set the flush zero and denormals modes from Embrees's perf. recommendations
/// https://embree.github.io/api.html#performance-recommendations
pub fn enable_ftz_and_daz() {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{_mm_getcsr, _mm_setcsr, _MM_FLUSH_ZERO_MASK};
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{_mm_getcsr, _mm_setcsr, _MM_FLUSH_ZERO_MASK};

        let flag = _MM_FLUSH_ZERO_MASK | 0x0040;
        unsafe {
            let csr = (_mm_getcsr() & !flag) | flag;
            _mm_setcsr(csr);
        }
    }
}

fn default_error_function(error: Error, msg: &str) {
    eprintln!("Embree error {:?} - {}", error, msg);
}
