//! Independently claimed differential-recovery batches.
//!
//! Each file is one Recomp@Home task output, so concurrent contributors never
//! edit a shared registry. The trusted verifier includes the claimed file alone.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observed {
    pub eax: Option<u32>,
    pub double_bits: Option<u64>,
}
