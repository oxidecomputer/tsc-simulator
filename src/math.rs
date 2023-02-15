use anyhow::{anyhow, Result};

pub const INT_SIZE_INTEL: u8 = 16;
pub const FRAC_SIZE_INTEL: u8 = 48;

pub const INT_SIZE_AMD: u8 = 8;
pub const FRAC_SIZE_AMD: u8 = 32;

pub const NS_PER_SEC: u32 = 1000000000;

pub const HZ_PER_KHZ: u64 = 1000;

fn fixed_pt_format_64(frac_size: u8, int_size: Option<u8>) -> (u8, u8) {
    assert!(frac_size > 0);
    assert!(frac_size < 64);

    let int_size = if let Some(i) = int_size {
        i
    } else {
        64 - frac_size
    };
    assert!(
        (int_size + frac_size) <= 64,
        "int_size + frac_size must be <= 64"
    );

    (int_size, frac_size)
}

fn overflow(val: u128, int_size: u8, frac_size: u8) -> bool {
    let n_unused_bits = 64 + (64 - int_size - frac_size);
    let mask = !(u128::MAX >> n_unused_bits);

    (val & mask) != 0
}

/// Given as input guest and host frequencies in KHz, outputs a fixed point number
/// representing the ratio of guest/host, with the binary point at the last
/// `frac_size` bits.
///
/// Max frequency KHz value if represented as a u32: 4294.96725 GHz   
pub fn freq_multiplier(
    guest_khz: u32,
    host_khz: u32,
    frac_size: u8,
    int_size: Option<u8>,
) -> Result<u64> {
    assert_ne!(guest_khz, 0);
    assert_ne!(host_khz, 0);

    let (int_size, frac_size) = fixed_pt_format_64(frac_size, int_size);

    let scaling_factor: u64 = 1 << frac_size;
    let multiplier = (scaling_factor as u128 * guest_khz as u128) / host_khz as u128;

    if overflow(multiplier, int_size, frac_size) {
        return Err(anyhow!(
            "frequency ratio too large: guest_khz={guest_khz}, host_khz={host_khz}, {int_size}.{frac_size} format"
        ));
    }

    Ok(multiplier as u64)
}

// Helper function to keep from calculating the multiplier twice
// (That is, assumes that `multiplier` was created by `freq_multiplier`)
//
// `multiplier` is a fixed point number, with `frac_size` fractional bits,
// representing the ratio of guest frequency to host frequency.
//
// XXX: add an example with decimal and binary
//
// TSC offset = - (host_tsc * ratio - guest_tsc)
fn calc_tsc_offset(
    initial_host_tsc: u64,
    initial_guest_tsc: u64,
    multiplier: u64,
    frac_size: u8,
    int_size: Option<u8>,
) -> Result<i64> {
    let (int_size, frac_size) = fixed_pt_format_64(frac_size, int_size);
    let host_tsc_scaled: u128 = (initial_host_tsc as u128 * multiplier as u128) >> frac_size;

    if overflow(host_tsc_scaled, int_size, frac_size) {
        return Err(anyhow!(
            "cannot scale host TSC: host_tsc={initial_host_tsc}, {int_size}.{frac_size} format"
        ));
    }

    /*eprintln!(
    "host_tsc_scaled={}, initial_guest_tsc={}",
    host_tsc_scaled, initial_guest_tsc
    );*/

    let (diff, negate) = if host_tsc_scaled as u64 >= initial_guest_tsc {
        ((host_tsc_scaled as u64 - initial_guest_tsc), true)
    } else {
        ((initial_guest_tsc - host_tsc_scaled as u64), false)
    };

    if diff == u64::MAX {
        return Err(anyhow!("cannot represent TSC offset"));
    }

    let res = if negate { -(diff as i64) } else { diff as i64 };

    Ok(res)
}

pub fn tsc_offset(
    initial_host_tsc: u64,
    initial_guest_tsc: u64,
    guest_khz: u32,
    host_khz: u32,
    frac_size: u8,
    int_size: Option<u8>,
) -> Result<i64> {
    let multiplier = freq_multiplier(guest_khz, host_khz, frac_size, int_size)?;
    calc_tsc_offset(
        initial_host_tsc,
        initial_guest_tsc,
        multiplier,
        frac_size,
        int_size,
    )
}

pub fn guest_tsc(
    initial_host_tsc: u64,
    initial_guest_tsc: u64,
    host_khz: u32,
    guest_khz: u32,
    cur_host_tsc: u64,
    frac_size: u8,
    int_size: Option<u8>,
) -> Result<u64> {
    let freq_multiplier = freq_multiplier(guest_khz, host_khz, frac_size, int_size)?;
    let tsc_offset = calc_tsc_offset(
        initial_host_tsc,
        initial_guest_tsc,
        freq_multiplier,
        frac_size,
        int_size,
    )?;

    let host_tsc_scaled: u128 = cur_host_tsc as u128 * freq_multiplier as u128;
    let res = ((host_tsc_scaled >> frac_size) as i64) + tsc_offset;
    // TODO: handle edge cases

    Ok(res as u64)
}

pub fn tsc_incr(tsc: u64, freq_khz: u32) -> u64 {
    let freq_hz = freq_khz as u64 * HZ_PER_KHZ;
    tsc + freq_hz
}

pub fn hrtime(tsc: u64, freq_hz: u64) -> Result<u64> {
    let product: u128 = tsc as u128 * NS_PER_SEC as u128;

    // TODO: math edge cases
    Ok(product as u64 / freq_hz)
}

pub fn tsc(hrtime: u64, freq_hz: u64) -> Result<u64> {
    let product: u128 = hrtime as u128 * freq_hz as u128;

    // TODO: math edge cases
    Ok(product as u64 / NS_PER_SEC as u64)
}

mod tests {
    use crate::math::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    // Ensure we can't panic freq_multiplier
    #[quickcheck]
    fn freq_ratio_check(gf: u32, hf: u32, frac: u8, int: Option<u8>) -> TestResult {
        if gf == 0 || hf == 0 || frac == 0 || frac >= 64 {
            return TestResult::discard();
        }
        match int {
            Some(i) if i == 0 || i >= 64 || (i + frac) > 64 => TestResult::discard(),
            Some(_) | None => {
                let _ = freq_multiplier(gf, hf, frac, int);

                TestResult::from_bool(true)
            }
        }
    }

    // Check that tsc_offset() doesn't panic, assuming:
    // - guest/host frequencies are > 0
    // - int_size/frac_size are nonzero and fit into 64 bits
    #[quickcheck]
    fn tsc_offset_check(
        ihtsc: u64,
        igtsc: u64,
        gf: u32,
        hf: u32,
        frac: u8,
        int: Option<u8>,
    ) -> TestResult {
        if gf == 0 || hf == 0 || frac == 0 || frac >= 64 {
            return TestResult::discard();
        }

        match int {
            Some(i) if i == 0 || i >= 64 || (i + frac) > 64 => TestResult::discard(),
            Some(_) | None => {
                tsc_offset(ihtsc, igtsc, gf, hf, frac, None);
                TestResult::from_bool(true)
            }
        }
    }

    #[test]
    fn test_freq_ratio() {
        // 0.5 = 2^-1
        assert!(matches!(freq_multiplier(1000, 2000, 2, None), Ok(0b10)));
        assert!(matches!(
            freq_multiplier(1000, 2000, 8, None),
            Ok(0b10000000)
        ));

        // 1.5 = 2^0 + 2^-1
        assert!(matches!(freq_multiplier(3000, 2000, 2, None), Ok(0b110)));
        assert!(matches!(
            freq_multiplier(3000, 2000, 8, None),
            Ok(0b110000000)
        ));

        // 0.66 = 2^-1 + 2^-3 + 2^-5 + 2^-7
        assert!(matches!(
            freq_multiplier(2000, 3000, 8, None),
            Ok(0b10101010)
        ));

        // Intel: 16.48
        let _n = 1u64 << FRAC_SIZE_INTEL;
        assert!(matches!(
            freq_multiplier(1000, 1000, FRAC_SIZE_INTEL, Some(INT_SIZE_INTEL)),
            Ok(_n)
        ));

        // AMD: 8.32
        let _n = 1u64 << 32;
        assert!(matches!(
            freq_multiplier(1000, 1000, FRAC_SIZE_AMD, Some(INT_SIZE_AMD)),
            Ok(_n)
        ));

        // varied frequency sizes, ratio=1.0
        let _n = 1u64 << 63;
        assert!(matches!(
            freq_multiplier(u32::MAX, u32::MAX, 63, None),
            Ok(_n)
        ));
        assert!(matches!(freq_multiplier(1000, 1000, 63, None), Ok(_n)));
        assert!(matches!(freq_multiplier(1, 1, 63, None), Ok(_n)));
    }

    #[test]
    fn test_freq_ratio_edge_cases() {
        // Overflow conditions for frequency ratio calculation:
        // - `scaling_factor * guest_freq` doesn't fit into 64 bits (>= 2^64)
        // - `scaling_factor * guest_freq` doesn't fit into `int + frac` bits

        /*
         * 1.63 format
         * representable ratios:
         * - int: [0, 1]
         */
        // ratio=0.5
        let _n = (1u64 << 63) & (1u64 << 62);
        assert!(matches!(freq_multiplier(500, 1000, 63, None), Ok(_n)));

        // ratio=0.75
        let _n = (1u64 << 62) & (1u64 << 61);
        assert!(matches!(freq_multiplier(750, 1000, 63, None), Ok(_n)));

        // ratio=1.0
        let _n = 1u64 << 63;
        assert!(matches!(freq_multiplier(1000, 1000, 63, None), Ok(_n)));

        // ratio=1.75
        let _n = (1u64 << 63) & (1u64 << 62) & (1u64 << 61);
        assert!(matches!(freq_multiplier(1750, 1000, 63, None), Ok(_n)));

        // OOB: ratio=2.0
        assert!(matches!(freq_multiplier(2000, 1000, 63, None), Err(_)));

        /*
         * 63.1 format
         * representable ratios:
         * - int: [0, 2^64 - 1]
         * - frac: 1 digit of precision (0.5)
         */

        // frac max precision: 0.5
        // < 0.5 => 0.0
        assert!(matches!(freq_multiplier(1, 1000, 1, None), Ok(0)));
        assert!(matches!(freq_multiplier(499, 1000, 1, None), Ok(0)));
        let _n = 2 ^ 63 & 0b1;
        // = 0.5
        assert!(matches!(freq_multiplier(500, 1000, 1, None), Ok(_n)));
        // > 0.5 => 0.5
        assert!(matches!(freq_multiplier(510, 1000, 1, None), Ok(_n)));
        assert!(matches!(freq_multiplier(999, 1000, 1, None), Ok(_n)));

        /*
         * Intel: 16.48 format
         * representable ratios:
         * - int: [0, 65535]
         * - frac: 48-binary digits of precision
         */

        // int lower bound: 1
        let _n = 1u64 << FRAC_SIZE_INTEL;
        assert!(matches!(
            freq_multiplier(u32::MAX, u32::MAX, FRAC_SIZE_INTEL, Some(INT_SIZE_INTEL)),
            Ok(_n)
        ));

        // int upper bound: 65535
        let _n = 65535u64 << FRAC_SIZE_INTEL;
        assert!(matches!(
            freq_multiplier(65535000, 1000, FRAC_SIZE_INTEL, Some(INT_SIZE_INTEL)),
            Ok(_n)
        ));

        /*
         * AMD: 8.32 format
         */
    }

    /*
    #[test]
    fn test_tsc_offset() {
        // 56.8 format
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 1000, 8, None),
            Ok(180000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 2000, 8, None),
            Ok(90000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 2000, 1000, 8, None),
            Ok(360000000000)
        ));

        // 32.32 format
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 1000, 32, None),
            Ok(180000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 2000, 32, None),
            Ok(90000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 2000, 1000, 32, None),
            Ok(360000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 1500, 1000, 32, None),
            Ok(270000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 1500, 32, None),
            Ok(119999999972)
        ));

        // Intel: 16.48 format
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 1000, 48, None),
            Ok(180000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 2000, 48, None),
            Ok(90000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 2000, 1000, 48, None),
            Ok(360000000000)
        ));

        // AMD: 8.32 format
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 1000, 32, Some(8)),
            Ok(180000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 1000, 2000, 32, Some(8)),
            Ok(90000000000)
        ));
        assert!(matches!(
            tsc_offset(180000000000, 0, 2000, 1000, 32, Some(8)),
            Ok(360000000000)
        ));
    }
    */
}