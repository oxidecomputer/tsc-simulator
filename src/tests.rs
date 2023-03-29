use crate::{FRAC_SIZE_AMD, FRAC_SIZE_INTEL, INT_SIZE_AMD, INT_SIZE_INTEL};

pub(crate) struct Frt {
    pub g: u64,
    pub h: u64,
    pub f: u32,
    pub v: u64,
}

#[rustfmt::skip]
pub(crate) const FREQ_RATIO_TESTS_VALID: &'static [Frt] = &[

    // Smaller frequencies (~KHz)

    // 0.5 = 2^-1
    Frt { g: 1000, h: 2000, f: 2,                v: 1 << 1 },
    Frt { g: 1000, h: 2000, f: 8,                v: 1 << 7 },
    Frt { g: 1000, h: 2000, f: FRAC_SIZE_AMD,    v: 1 << 31 },
    Frt { g: 1000, h: 2000, f: FRAC_SIZE_INTEL,  v: 1 << 47 },
    Frt { g: 1000, h: 2000, f: 63,               v: 1 << 62 },

    // 2/3 = 2^-1 + 2^-3 + 2^-5 + 2^-7 ...
    Frt { g: 2000, h: 3000, f: 2,                v: 0b10 },
    Frt { g: 2000, h: 3000, f: 8,                v: 0b10101010 },
    Frt { g: 2000, h: 3000, f: FRAC_SIZE_AMD,    v: 0xaaaa_aaaa },
    Frt { g: 2000, h: 3000, f: FRAC_SIZE_INTEL,  v: 0xaaaa_aaaa_aaaa },
    Frt { g: 2000, h: 3000, f: 63,               v: 0x5555_5555_5555_5555 },

    // 1.5 = 2^0 + 2^-1
    Frt { g: 3000, h: 2000, f: 2,                v: 1 << 2 | 1 << 1 },
    Frt { g: 3000, h: 2000, f: 8,                v: 1 << 8 | 1 << 7 },
    Frt { g: 3000, h: 2000, f: FRAC_SIZE_AMD,    v: 1 << 32 | 1 << 31 },
    Frt { g: 3000, h: 2000, f: FRAC_SIZE_INTEL,  v: 1 << 48 | 1 << 47 },
    Frt { g: 3000, h: 2000, f: 63,               v: 1 << 63 | 1 << 62 },


    // Larger frequencies (~GHz)

    // 0.5 = 2^-1
    Frt { g: 1000000000, h: 2000000000, f: 2,                v: 1 << 1 },
    Frt { g: 1000000000, h: 2000000000, f: 8,                v: 1 << 7 },
    Frt { g: 1000000000, h: 2000000000, f: FRAC_SIZE_AMD,    v: 1 << 31 },
    Frt { g: 1000000000, h: 2000000000, f: FRAC_SIZE_INTEL,  v: 1 << 47 },
    Frt { g: 1000000000, h: 2000000000, f: 63,               v: 1 << 62 },

    // 2/3 = 2^-1 + 2^-3 + 2^-5 + 2^-7 ...
    Frt { g: 2000000000, h: 3000000000, f: 2,                v: 0b10 },
    Frt { g: 2000000000, h: 3000000000, f: 8,                v: 0b10101010 },
    Frt { g: 2000000000, h: 3000000000, f: FRAC_SIZE_AMD,    v: 0xaaaa_aaaa },
    Frt { g: 2000000000, h: 3000000000, f: FRAC_SIZE_INTEL,  v: 0xaaaa_aaaa_aaaa },
    Frt { g: 2000000000, h: 3000000000, f: 63,               v: 0x5555_5555_5555_5555 },

    // 1.5 = 2^0 + 2^-1
    Frt { g: 3000000000, h: 2000000000, f: 2,                v: 1 << 2 | 1 << 1 },
    Frt { g: 3000000000, h: 2000000000, f: 8,                v: 1 << 8 | 1 << 7 },
    Frt { g: 3000000000, h: 2000000000, f: FRAC_SIZE_AMD,    v: 1 << 32 | 1 << 31 },
    Frt { g: 3000000000, h: 2000000000, f: FRAC_SIZE_INTEL,  v: 1 << 48 | 1 << 47 },
    Frt { g: 3000000000, h: 2000000000, f: 63,               v: 1 << 63 | 1 << 62 },
];

pub(crate) struct Frti {
    pub g: u64,
    pub h: u64,
    pub f: u32,
}

#[rustfmt::skip]
pub(crate) const FREQ_RATIO_TESTS_INVALID: &'static [Frti] = &[
    // values that overflow the int portion, generating an error for rust and a
    // #DE for the assembly version

    // can't fit ratio 2.0 in 1-bit integer
    Frti { g: 2000, h: 1000, f: 63, },
    Frti { g: 2000000000, h: 1000000000, f: 63, },

    // can't fit ratio 2^32 in 32-bits
    Frti { g: 4294967296, h: 1, f: FRAC_SIZE_AMD, },

    // can't fit ratio 2^16 in 16-bits
    Frti { g: 65536, h: 1, f: FRAC_SIZE_INTEL, },
];

pub(crate) struct Stt {
    pub t: u64,
    pub m: u64,
    pub f: u32,
    pub v: u64,
}

#[rustfmt::skip]
pub(crate) const SCALE_TSC_TESTS_VALID: &'static [Stt] = &[
    // Ratio = 1.0
    Stt { t: 1, m: 1 << 1, f: 1, v: 1 },
    Stt { t: 1000000000, m: 1 << FRAC_SIZE_AMD,     f: FRAC_SIZE_AMD,   v: 1000000000, },
    Stt { t: 1000000000, m: 1 << FRAC_SIZE_INTEL,   f: FRAC_SIZE_INTEL, v: 1000000000, },
    Stt { t: 5890513020, m: 1 << FRAC_SIZE_AMD,     f: FRAC_SIZE_AMD,   v: 5890513020, },
    Stt { t: 5890513020, m: 1 << FRAC_SIZE_INTEL,   f: FRAC_SIZE_INTEL, v: 5890513020, },

    // Ratio = 0.5
    Stt { t: 1, m: 1 << 0, f: 1, v: 0 },
    Stt { t: 1000000000, m: 1 << 31, f: FRAC_SIZE_AMD,      v: 500000000, },
    Stt { t: 1000000000, m: 1 << 47, f: FRAC_SIZE_INTEL,    v: 500000000, },
    Stt { t: 5890513020, m: 1 << 31, f: FRAC_SIZE_AMD,      v: 5890513020 / 2, },
    Stt { t: 5890513020, m: 1 << 47, f: FRAC_SIZE_INTEL,    v: 5890513020 / 2, },

    // Ratio = 1.5
    Stt { t: 1, m: 1 << 1 | 1 << 0, f: 1, v: 1 },
    Stt { t: 1000000000, m: 1 << 32 | 1 << 31, f: FRAC_SIZE_AMD,      v: 1500000000, },
    Stt { t: 1000000000, m: 1 << 48 | 1 << 47, f: FRAC_SIZE_INTEL,    v: 1500000000, },
    Stt { t: 5890513020, m: 1 << 32 | 1 << 31, f: FRAC_SIZE_AMD,      v: 5890513020 + 5890513020 / 2, },
    Stt { t: 5890513020, m: 1 << 48 | 1 << 47, f: FRAC_SIZE_INTEL,    v: 5890513020 + 5890513020 / 2, },

    // Edge cases
    Stt { t: u64::MAX, m: 1 << 1, f: 1, v: u64::MAX, },
    Stt { t: u64::MAX, m: 1 << 32, f: FRAC_SIZE_AMD, v: u64::MAX, },
    Stt { t: u64::MAX, m: 1 << 48, f: FRAC_SIZE_INTEL, v: u64::MAX, },
];

pub(crate) struct Stti {
    pub t: u64,
    pub m: u64,
    pub f: u32,
}

#[rustfmt::skip]
pub(crate) const SCALE_TSC_TESTS_INVALID: &'static [Stti] = &[
    // values that overflow: (tsc * multiplier) >> frac
    Stti { t: u64::MAX, m: 1 << 1 | 1 << 0, f: 1 },
    Stti { t: u64::MAX, m: 1 << 32 | 1 << 31, f: FRAC_SIZE_AMD },
    Stti { t: u64::MAX, m: 1 << 48 | 1 << 47, f: FRAC_SIZE_INTEL },
];

#[cfg(test)]
mod tests {
    use super::{
        FREQ_RATIO_TESTS_INVALID, FREQ_RATIO_TESTS_VALID,
        SCALE_TSC_TESTS_INVALID, SCALE_TSC_TESTS_VALID,
    };
    use crate::asm_math;
    use crate::math;

    #[test]
    fn test_freq_ratio() {
        for i in 0..FREQ_RATIO_TESTS_VALID.len() {
            let t = &FREQ_RATIO_TESTS_VALID[i];

            let msg = format!(
                "guest_freq={}, host_freq={}, frac_size={}, expected_val={}",
                t.g, t.h, t.f, t.v
            );

            // Check rust implementation
            let rs_res = math::freq_multiplier(t.g, t.h, t.f, 64 - t.f);
            match rs_res {
                Ok(v) => {
                    assert_eq!(v, t.v, "rust impl failure: {}", msg);
                }
                Err(e) => {
                    panic!(
                        "rust impl failure, got err {} instead of value: {}",
                        e, msg
                    );
                }
            }

            // Check asm implementation
            assert_eq!(
                unsafe { asm_math::calc_freq_multiplier(t.g, t.h, t.f) },
                t.v,
                "asm impl failure: {}",
                msg
            );
        }
    }

    #[test]
    fn test_freq_ratio_invalid() {
        for i in 0..FREQ_RATIO_TESTS_INVALID.len() {
            let t = &FREQ_RATIO_TESTS_INVALID[i];

            let msg = format!(
                "guest_freq={}, host_freq={}, frac_size={}",
                t.g, t.h, t.f
            );

            // Check that rust implementation throws an error
            let rs_res = math::freq_multiplier(t.g, t.h, t.f, 64 - t.f);
            assert!(
                rs_res.is_err(),
                "rust impl failure, got value {} instead of error: {}",
                rs_res.unwrap(),
                msg
            );

            // asm implementation will get a SIGFPE for these tests
        }
    }

    #[test]
    fn test_scale_tsc() {
        for i in 0..SCALE_TSC_TESTS_VALID.len() {
            let t = &SCALE_TSC_TESTS_VALID[i];

            let msg = format!(
                "tsc={}, mult={}, frac_size={}, expected_val={}",
                t.t, t.m, t.f, t.v
            );

            // Check rust implementation
            let rs_res = math::scale_tsc(t.t, t.m, t.f);
            match rs_res {
                Ok(v) => {
                    assert_eq!(v, t.v, "rust impl failure: {}", msg);
                }
                Err(e) => {
                    panic!(
                        "rust impl failure, got err {} instead of value: {}",
                        e, msg
                    );
                }
            }

            // Check asm implementation
            assert_eq!(
                unsafe { asm_math::scale_tsc(t.t, t.m, t.f) },
                t.v,
                "asm impl failure: {}",
                msg
            );
        }
    }

    #[test]
    fn test_scale_tsc_invalid() {
        for i in 0..SCALE_TSC_TESTS_INVALID.len() {
            let t = &SCALE_TSC_TESTS_INVALID[i];

            let msg = format!("tsc={}, mult={}, frac_size={}", t.t, t.m, t.f);

            // Check that rust implementation throws an error
            let rs_res = math::scale_tsc(t.t, t.m, t.f);
            assert!(
                rs_res.is_err(),
                "rust impl failure, got value {} instead of error: {}",
                rs_res.unwrap(),
                msg
            );

            // call the asm implementation to make sure we don't panic
            unsafe { asm_math::scale_tsc(t.t, t.m, t.f) };
        }
    }
}
