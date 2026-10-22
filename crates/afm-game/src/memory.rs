//! Handing freed memory back to the system (final build lane 1, memory).
//! A season frees what its days built (game rosters, the year's lists),
//! and the system allocator keeps those pages for itself unless it's asked
//! to let them go. Asking after a load and after each Continue keeps the
//! game's resident memory near what it really holds (about 0.2 GB less in
//! season on the 30-year world). It changes no result.

/// Returns the allocator's free pages to the system.
pub fn hand_back() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        // SAFETY: glibc's malloc_trim only releases free memory.
        unsafe { malloc_trim(0) };
    }
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
        }
        // SAFETY: a null zone asks every malloc zone to release free pages
        // (goal 0: as much as it can); live allocations are untouched.
        unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    }
}
