use libc::{c_ulong, c_ulonglong};

extern "C" {
    pub fn _calc_freq_multiplier(
        guest_hz: c_ulonglong,
        host_hz: c_ulonglong,
        frac_size: c_ulong,
    ) -> c_ulonglong;

    pub fn _scale_tsc(
        tsc: c_ulonglong,
        multiplier: c_ulonglong,
        frac_size: c_ulong,
    ) -> c_ulonglong;
}
