use libc::{c_uchar, c_ulonglong};

extern "C" {
    pub fn calc_freq_multiplier(
        guest_hz: c_ulonglong,
        host_hz: c_ulonglong,
        frac_size: c_uchar,
    ) -> c_ulonglong;

    pub fn scale_tsc(tsc: c_ulonglong, multiplier: c_ulonglong, frac_size: c_uchar) -> c_ulonglong;
}
