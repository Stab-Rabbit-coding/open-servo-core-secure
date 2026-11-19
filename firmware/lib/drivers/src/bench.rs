//! Bench-only probes: counter bodies exist under the `bench` feature; call
//! sites stay unconditional in hot paths and compile to nothing without it
//! (the closure is never called and the empty inline erases).
//!
//! Counters live in `no_mangle` statics so the debug link can dump them by
//! symbol address (`nm` the ELF) on a running chip -- no wire traffic, no
//! table space, and they survive test tails that re-center transport state.

pub use osc_servo_core::budget::probe::TelStep;

use crate::traits::bus::Deadline;

/// Message-plane probe (`docs/security-architecture.md` §2.6, §7.4).
///
/// The architecture's timing claims are **analytical, not measured** -- no
/// Rust toolchain was available when it was written, and this repository's
/// convention is that timing facts come from silicon ([F1]-[F15]). This probe
/// is how that gap gets closed: dump it by symbol address over the debug link
/// after a hot-loop run and the fold cost, the verdict mix and the lockout
/// behaviour all fall out of the counters.
///
/// The number the design hangs on is **fold cost per hot-loop cycle**. The
/// analytical estimate is ~34 µs for a ~70-byte cycle (~42 HalfSipHash rounds
/// at ~26 instructions each, ~1.5 CPI at 48 MHz), which by the transport's
/// measured tick-loss relation (§2 of `osc-servo-transport.md`: loss ~= 1.2-1.4x
/// transport-HIGH duty) should cost ~4-5% additional kernel tick coalescing at
/// a 1 kHz bus cycle. `fold_cycles / folds` against `fold_bytes / folds` is
/// the direct measurement; if it lands materially over the estimate, the
/// documented levers are the split feed, HalfSipHash-1-3, or policy narrowing.
#[repr(C)]
pub struct SecProbe {
    /// Frames folded into the stream digest.
    pub folds: u32,
    /// Bytes folded -- with `folds`, gives bytes/frame for the cost model.
    pub fold_bytes: u32,
    /// Cycle counter accumulated across folds, when the chip supplies one.
    pub fold_cycles: u32,
    /// Verdicts rendered, and their breakdown.
    pub verdicts: u32,
    pub pass: u32,
    pub missing: u32,
    pub bad_tag: u32,
    pub replay: u32,
    pub no_session: u32,
    pub locked_out: u32,
    /// Frames refused because their covered span exceeded the in-place
    /// authentication buffer. Should be zero under the intended hot loop; a
    /// nonzero value means `AUTH_SPAN_MAX` is mis-sized for the deployment.
    pub oversize: u32,
}

impl SecProbe {
    pub const ZERO: Self = Self {
        folds: 0,
        fold_bytes: 0,
        fold_cycles: 0,
        verdicts: 0,
        pass: 0,
        missing: 0,
        bad_tag: 0,
        replay: 0,
        no_session: 0,
        locked_out: 0,
        oversize: 0,
    };
}

#[cfg(feature = "bench")]
static mut TEL_BANKS: u32 = 0;

#[cfg(feature = "bench")]
#[unsafe(no_mangle)]
pub static mut SEC_PROBE: SecProbe = SecProbe::ZERO;

/// Run `f` over the security probe -- a no-op without the `bench` feature.
#[inline(always)]
#[allow(unused_variables)]
pub fn sec_probe(f: impl FnOnce(&mut SecProbe)) {
    // SAFETY: single-hart; every writer runs at the one transport priority
    // (HIGH), so accesses never interleave. The debug link only reads.
    #[cfg(feature = "bench")]
    unsafe {
        f(&mut *core::ptr::addr_of_mut!(SEC_PROBE));
    }
}

/// The TEL encoder banked a batch (kernel tick, the only writer).
#[inline(always)]
pub fn tel_banked() {
    // SAFETY: single writer; readers take one word.
    #[cfg(feature = "bench")]
    unsafe {
        let b = &raw mut TEL_BANKS;
        b.write_volatile(b.read_volatile().wrapping_add(1));
    }
}

/// Batches banked so far; 0 without the `bench` feature.
#[inline(always)]
pub fn tel_banks() -> u32 {
    #[cfg(feature = "bench")]
    // SAFETY: one-word volatile read.
    unsafe {
        (&raw const TEL_BANKS).read_volatile()
    }
    #[cfg(not(feature = "bench"))]
    0
}

#[cfg(feature = "bench-telcost")]
#[unsafe(no_mangle)]
pub static mut TEL_COST: osc_servo_core::budget::probe::TelCost =
    osc_servo_core::budget::probe::TelCost::ZERO;

/// A TEL step's opening stamp on `clock`; 0 without `bench-telcost`.
#[inline(always)]
#[allow(unused_variables)]
pub fn tel_stamp(clock: &impl Deadline) -> u32 {
    #[cfg(feature = "bench-telcost")]
    {
        clock.now()
    }
    #[cfg(not(feature = "bench-telcost"))]
    0
}

/// Step `s`, stamped at `from`, ends now.
#[inline(always)]
#[allow(unused_variables)]
pub fn tel_step(clock: &impl Deadline, from: u32, s: TelStep) {
    // SAFETY: the kernel tick is the record's only writer, bringup aside.
    #[cfg(feature = "bench-telcost")]
    unsafe {
        let wall = clock.now().wrapping_sub(from);
        (*core::ptr::addr_of_mut!(TEL_COST)).add(s, wall);
    }
}

/// The TEL hook stamped at `from` ends now: its span joins the running
/// tick's hook time. Every tick, TEL or not.
#[inline(always)]
#[allow(unused_variables)]
pub fn tel_hook(clock: &impl Deadline, from: u32) {
    // SAFETY: the kernel tick is the record's only writer, bringup aside.
    #[cfg(feature = "bench-telcost")]
    unsafe {
        let h = &raw mut TEL_COST.hook;
        h.write(h.read().wrapping_add(clock.now().wrapping_sub(from)));
    }
}

/// Price a back-to-back stamp pair. Call once, before interrupts run.
#[inline(always)]
#[allow(unused_variables)]
pub fn tel_calibrate(clock: &impl Deadline) {
    // SAFETY: pre-IRQ bringup, ahead of the record's only other writer.
    #[cfg(feature = "bench-telcost")]
    unsafe {
        let a = clock.now();
        let b = clock.now();
        let cal = &raw mut TEL_COST.cal;
        cal.write_volatile(b.wrapping_sub(a).min(u16::MAX.into()) as u16);
    }
}

/// Arms picked up so far; 0 without `bench-telcost`.
#[inline(always)]
pub fn tel_pickups() -> u32 {
    #[cfg(feature = "bench-telcost")]
    // SAFETY: one-word volatile read.
    unsafe {
        (&raw const TEL_COST.n[TelStep::Pickup as usize]).read_volatile()
    }
    #[cfg(not(feature = "bench-telcost"))]
    0
}

/// The burst picked up last stamps its steps: every other one does, the
/// rest run unstamped for the kernel's TEL cost.
#[inline(always)]
pub fn tel_stamping() -> bool {
    tel_pickups() & 1 != 0
}

/// The running kernel tick's steps, taken: the tick's end calls it.
#[inline(always)]
pub fn tel_tick_steps() -> u32 {
    #[cfg(feature = "bench-telcost")]
    // SAFETY: the kernel tick's end, the record's only writer's context.
    unsafe {
        let t = &raw mut TEL_COST.tick;
        let steps = t.read();
        t.write(0);
        steps
    }
    #[cfg(not(feature = "bench-telcost"))]
    0
}

/// The running kernel tick's hook time, taken: the tick's end calls it.
#[inline(always)]
pub fn tel_tick_hook() -> u32 {
    #[cfg(feature = "bench-telcost")]
    // SAFETY: as `tel_tick_steps`.
    unsafe {
        let h = &raw mut TEL_COST.hook;
        let hook = h.read();
        h.write(0);
        hook
    }
    #[cfg(not(feature = "bench-telcost"))]
    0
}
