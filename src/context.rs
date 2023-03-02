use crate::sys::*;

pub unsafe trait AsIntersectContext {
    type Ext;

    fn as_intersect_context(&self) -> &IntersectContext;
    fn as_intersect_context_mut(&mut self) -> &mut IntersectContext;

    fn as_intersect_context_ptr(&self) -> *const IntersectContext {
        self.as_intersect_context() as *const IntersectContext
    }

    fn as_intersect_context_mut_ptr(&mut self) -> *mut IntersectContext {
        self.as_intersect_context_mut() as *mut IntersectContext
    }

    fn as_intersect_context_ext(&self) -> Option<&Self::Ext>;
    fn as_intersect_context_ext_mut(&mut self) -> Option<&mut Self::Ext>;
}

/// Per ray-query intersection context.
///
/// This is used to configure intersection flags, specify a filter callback
/// function, and specify the chain of IDs of the current instance, and to
/// attach arbitrary user data to the query (e.g. per ray data).
///
/// # Filter Callback
///
/// A filter function can be specified inside the context. This function is
/// invoked as a second filter stage after the per-geometry intersect or
/// occluded filter function is invoked. Only rays that passed the first filter
/// stage are valid in this second filter stage. Having such a per ray-query
/// filter function can be useful to implement modifications of the behavior of
/// the query, such as collecting all hits or accumulating transparencies.
///
/// ## Note
///
/// The support for the context filter function must be enabled for a scene by
/// using the [`RTCSceneFlags::CONTEXT_FILTER_FUNCTION`] flag.
///
/// In case of instancing this feature has to get enabled also for each
/// instantiated scene.
///
/// # Hints
///
/// Best primary ray performance can be obtained by using the ray stream API
/// and setting the intersect context flag to
/// [`RTCIntersectContextFlags::COHERENT`]. For secondary rays, it is typically
/// better to use the [`RTCIntersectContextFlags::INCOHERENT`], unless the rays
/// are known to be coherent(e.g. for primary transparency rays).
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

unsafe impl AsIntersectContext for IntersectContext {
    type Ext = ();

    fn as_intersect_context(&self) -> &IntersectContext { self }

    fn as_intersect_context_mut(&mut self) -> &mut IntersectContext { self }

    fn as_intersect_context_ext(&self) -> Option<&Self::Ext> { None }

    fn as_intersect_context_ext_mut(&mut self) -> Option<&mut Self::Ext> { None }
}

#[repr(C)]
#[derive(Debug)]
pub struct IntersectContextExt<E>
where
    E: Sized,
{
    pub context: IntersectContext,
    pub extra: E,
}

unsafe impl<E> AsIntersectContext for IntersectContextExt<E>
where
    E: Sized,
{
    type Ext = E;

    fn as_intersect_context(&self) -> &IntersectContext { &self.context }

    fn as_intersect_context_mut(&mut self) -> &mut IntersectContext { &mut self.context }

    fn as_intersect_context_ext(&self) -> Option<&Self::Ext> { Some(&self.extra) }

    fn as_intersect_context_ext_mut(&mut self) -> Option<&mut Self::Ext> { Some(&mut self.extra) }
}

impl<E> IntersectContextExt<E>
where
    E: Sized,
{
    pub fn new(flags: RTCIntersectContextFlags, extra: E) -> IntersectContextExt<E> {
        IntersectContextExt {
            context: IntersectContext::new(flags),
            extra,
        }
    }

    pub fn coherent(extra: E) -> IntersectContextExt<E> {
        IntersectContextExt {
            context: IntersectContext::coherent(),
            extra,
        }
    }

    pub fn incoherent(extra: E) -> IntersectContextExt<E> {
        IntersectContextExt {
            context: IntersectContext::incoherent(),
            extra,
        }
    }
}

/// A stack which stores the IDs and instance transformations during a BVH
/// traversal for a point query.
///
/// The transformations are assumed to be affine transformations
/// (3×3 matrix plus translation) and therefore the last column is ignored.
pub type PointQueryContext = RTCPointQueryContext;

// TODO: PointQueryContext::new
