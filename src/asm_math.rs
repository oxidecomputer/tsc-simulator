use libc::{c_uint, c_ulonglong};

extern "C" {
    pub fn calc_freq_multiplier(
        guest_hz: c_ulonglong,
        host_hz: c_ulonglong,
        frac_size: c_uint,
    ) -> c_ulonglong;

    pub fn scale_tsc(
        tsc: c_ulonglong,
        multiplier: c_ulonglong,
        frac_size: c_uint,
    ) -> c_ulonglong;
}

pub fn calc_tsc_offset(
    initial_host_tsc: u64,
    initial_guest_tsc: u64,
    guest_hz: u64,
    host_hz: u64,
    frac_size: u32,
) -> i64 {
    let mult = unsafe { calc_freq_multiplier(guest_hz, host_hz, frac_size) };
    let host_tsc_scaled =
        unsafe { scale_tsc(initial_host_tsc, mult, frac_size) };

    let (diff, negate) = if host_tsc_scaled >= initial_guest_tsc {
        ((host_tsc_scaled - initial_guest_tsc), true)
    } else {
        ((initial_guest_tsc - host_tsc_scaled), false)
    };

    let res = if negate { -(diff as i64) } else { diff as i64 };

    res
}

pub fn calc_guest_tsc(
    initial_host_tsc: u64,
    initial_guest_tsc: u64,
    host_hz: u64,
    guest_hz: u64,
    cur_host_tsc: u64,
    frac_size: u32,
) -> u64 {
    let mult = unsafe { calc_freq_multiplier(guest_hz, host_hz, frac_size) };
    let offset = calc_tsc_offset(
        initial_host_tsc,
        initial_guest_tsc,
        guest_hz,
        host_hz,
        frac_size,
    );
    let host_tsc_scaled = unsafe { scale_tsc(cur_host_tsc, mult, frac_size) };

    let guest_tsc = host_tsc_scaled as i64 + offset;

    guest_tsc as u64
}
