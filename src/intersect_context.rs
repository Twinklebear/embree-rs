use crate::sys::*;

/// Per ray-query intersection context.
///
/// This is used to configure intersection flags, specify a filter callback function,
/// and specify the chain of IDs of the current instance, and to attach arbitrary user
/// data to the query (e.g. per ray data).
///
/// # Filter Callback
///
/// A filter function can be specified inside the context. This function is invoked as
/// a second filter stage after the per-geometry intersect or occluded filter function
/// is invoked. Only rays that passed the first filter stage are valid in this second
/// filter stage. Having such a per ray-query filter function can be useful
/// to implement modifications of the behavior of the query, such as collecting all
/// hits or accumulating transparencies.
///
/// ## Note
///
/// The support for the context filter function must be enabled for a scene by using
/// the [`RTCSceneFlags::CONTEXT_FILTER_FUNCTION`] flag.
///
/// In case of instancing this feature has to get enabled also for each instantiated scene.
///
/// # Hints
///
/// Best primary ray performance can be obtained by using the ray stream API
/// and setting the intersect context flag to [`RTCIntersectContextFlags::COHERENT`].
/// For secondary rays, it is typically better to use the [`RTCIntersectContextFlags::INCOHERENT`],
/// unless the rays are known to be coherent(e.g. for primary transparency rays).
pub type IntersectContext = RTCIntersectContext;

impl IntersectContext {
    /// Shortcut to create a IntersectContext with coherent flag set.
    pub fn coherent() -> IntersectContext {
        IntersectContext::new(RTCIntersectContextFlags::COHERENT)
    }

    /// Shortcut to create a IntersectContext with incoherent flag set.
    pub fn incoherent() -> IntersectContext {
        IntersectContext::new(RTCIntersectContextFlags::INCOHERENT)
    }

    pub fn new(flags: RTCIntersectContextFlags) -> IntersectContext {
        RTCIntersectContext {
            flags,
            filter: None,
            instID: [u32::MAX; 1],
        }
    }
}
