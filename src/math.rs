use anyhow::{anyhow, Result};

pub const INT_SIZE_INTEL: u32 = 16;
pub const FRAC_SIZE_INTEL: u32 = 48;

pub const INT_SIZE_AMD: u32 = 8;
pub const FRAC_SIZE_AMD: u32 = 32;

pub const NS_PER_SEC: u32 = 1000000000;

// Returns true if `val` will overflow `int_size + frac_size` bits
fn fixed_point_overflow(val: u128, int_size: u32, frac_size: u32) -> bool {
    let n_unused_bits = 64 + (64 - int_size - frac_size);
    let mask = !(u128::MAX >> n_unused_bits);

    (val & mask) != 0
}

// Returns true if `val` will overflow 6 bits
fn overflow_64(val: u128) -> bool {
    let mask = u128::MAX << 64;

    (val & mask) != 0
}

/// Given as input guest and host frequencies in Hz, outputs a fixed point
/// number representing the ratio of guest/host, with the binary point at the
/// last `frac_size` bits.
pub fn freq_multiplier(
    guest_hz: u64,
    host_hz: u64,
    frac_size: u32,
    int_size: u32,
) -> Result<u64> {
    assert_ne!(guest_hz, 0);
    assert_ne!(host_hz, 0);

    let scaling_factor: u64 = 1 << frac_size;
    let multiplier =
        (scaling_factor as u128 * guest_hz as u128) / host_hz as u128;

    if fixed_point_overflow(multiplier, int_size, frac_size) {
        return Err(anyhow!(
            "frequency ratio too large: guest_hz={}, host_hz={}, {}.{} format",
            guest_hz,
            host_hz,
            int_size,
            frac_size
        ));
    }

    Ok(multiplier as u64)
}

// Helper function to keep from calculating the multiplier twice
// (That is, `multiplier` is assumed to be created by `freq_multiplier`)
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
    frac_size: u32,
    int_size: u32,
) -> Result<i64> {
    let host_tsc_scaled: u128 =
        (initial_host_tsc as u128 * multiplier as u128) >> frac_size;

    if overflow_64(host_tsc_scaled) {
        return Err(anyhow!(
            "cannot scale host TSC: host_tsc={}, multiplier={} ({:#x}), {}.{} format",
            initial_host_tsc,
            multiplier,
            multiplier,
            int_size,
            frac_size
        ));
    }

    let (diff, negate) = if host_tsc_scaled as u64 >= initial_guest_tsc {
        ((host_tsc_scaled as u64 - initial_guest_tsc), true)
    } else {
        ((initial_guest_tsc - host_tsc_scaled as u64), false)
    };

    if (diff & 1u64 << 63) != 0 {
        return Err(anyhow!("negation of host_tsc_scaled={} and initial_guest_tsc={} will overflow (diff={}, negate={})",
            host_tsc_scaled,
            initial_guest_tsc,
            diff,
            negate));
    }

    let res = if negate { -(diff as i64) } else { diff as i64 };

    Ok(res)
}

/// Compute the TSC offset for a guest, with inputs:
/// - `initial_host_tsc`: TSC of the host when the guest started running
/// on this host (either at boot, or following a migration)
/// - `initial_guest_tsc`: TSC of guest when it started running on this host (0
/// for boot)
/// - frequencies of the guest and host (Hz)
/// - specification of the fixed point number format to do ratio calclations
/// with
///
pub fn tsc_offset(
    initial_host_tsc: u64,
    initial_guest_tsc: u64,
    guest_hz: u64,
    host_hz: u64,
    frac_size: u32,
    int_size: u32,
) -> Result<i64> {
    let multiplier = freq_multiplier(guest_hz, host_hz, frac_size, int_size)?;
    calc_tsc_offset(
        initial_host_tsc,
        initial_guest_tsc,
        multiplier,
        frac_size,
        int_size,
    )
}

/// Compute the guest TSC at a point in time for a guest, with inputs:
/// - `initial_host_tsc`: TSC of the host when the guest started running
/// on this host (either at boot, or following a migration)
/// - `initial_guest_tsc`: TSC of guest when it started running on this host (0
/// for boot)
/// - frequencies of the guest and host (Hz)
/// - `cur_host_tsc`: the current TSC value of the host (this is what anchors
/// this value to a point in "time")
/// - specification of the fixed point number format to do ratio calclations
/// with
///
pub fn guest_tsc(
    initial_host_tsc: u64,
    initial_guest_tsc: u64,
    host_hz: u64,
    guest_hz: u64,
    cur_host_tsc: u64,
    frac_size: u32,
    int_size: u32,
) -> Result<u64> {
    let freq_multiplier =
        freq_multiplier(guest_hz, host_hz, frac_size, int_size)?;
    let tsc_offset = calc_tsc_offset(
        initial_host_tsc,
        initial_guest_tsc,
        freq_multiplier,
        frac_size,
        int_size,
    )?;

    let host_tsc_scaled: u128 =
        (cur_host_tsc as u128 * freq_multiplier as u128) >> frac_size;
    if overflow_64(host_tsc_scaled) {
        return Err(anyhow!(
            "cannot scale host TSC: host_tsc={} ({:#x}), freq_multiplier={} ({:#x}), {}.{} format",
            cur_host_tsc,
            cur_host_tsc,
            freq_multiplier,
            freq_multiplier,
            int_size,
            frac_size
        ));
    }

    let guest_tsc: i128 = host_tsc_scaled as i128 + tsc_offset as i128;
    if overflow_64(guest_tsc as u128) {
        return Err(anyhow!(
            "offset addition will overflow: host_tsc_scaled={}, tsc_offset={}",
            host_tsc_scaled,
            tsc_offset
        ));
    }

    Ok(guest_tsc as u64)
}

// Outputs the TSC value one second in the future, for a given frequency
pub fn tsc_incr(tsc: u64, freq_hz: u64) -> u64 {
    tsc + freq_hz
}

// For an input TSC and frequency, translate to hrtime
pub fn hrtime(tsc: u64, freq_hz: u64) -> Result<u64> {
    // TODO: edge cases
    Ok((tsc as u64 / freq_hz) * NS_PER_SEC as u64)
}

// For an input hrtime and frequency, translate to a TSC value
pub fn tsc(hrtime: u64, freq_hz: u64) -> Result<u64> {
    // TODO: edge cases
    Ok((hrtime / NS_PER_SEC as u64) * freq_hz)
}

mod tests {
    const MIN_RATIO: u8 = 1;
    const MAX_RATIO: u8 = 15;

    use crate::math::*;
    //use crate::tests::freq_ratio_tests;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    // Ensure that freq_multiplier() doesn't panic, assuming:
    // - guest/host frequencies are > 0
    // - int_size/frac_size are nonzero and fit into 64 bits
    #[quickcheck]
    fn freq_multiplier_panic_check(
        gf: u64,
        hf: u64,
        frac: u32,
        int: u32,
    ) -> TestResult {
        if gf == 0
            || hf == 0
            || frac == 0
            || frac >= 64
            || int == 0
            || int >= 64
            || (int + frac) >= 64
        {
            return TestResult::discard();
        }

        let _ = freq_multiplier(gf, hf, frac, int);

        // if we got here, the function didn't panic, so it passes
        TestResult::from_bool(true)
    }

    // Check that tsc_offset() doesn't panic, assuming:
    // - guest/host frequencies are > 0
    // - int_size/frac_size are nonzero and fit into 64 bits
    #[quickcheck]
    fn tsc_offset_panic_check(
        ihtsc: u64,
        igtsc: u64,
        gf: u64,
        hf: u64,
        frac: u32,
        int: u32,
    ) -> TestResult {
        if gf == 0
            || hf == 0
            || frac == 0
            || frac >= 64
            || int == 0
            || int >= 64
            || (int + frac) >= 64
        {
            return TestResult::discard();
        }

        let _ = tsc_offset(ihtsc, igtsc, gf, hf, frac, int);

        // if we got here, the function didn't panic, so it passes
        TestResult::from_bool(true)
    }

    // Ensure that we can represent a reasonable range of ratios
    #[quickcheck]
    fn calc_tsc_offset_max_ratio(
        ihtsc: u64,
        igtsc: u64,
        int: u32,
        frac: u32,
        ratio: u8,
    ) -> TestResult {
        if !((int == INT_SIZE_AMD && frac == FRAC_SIZE_AMD)
            || (int == INT_SIZE_INTEL && frac == FRAC_SIZE_AMD))
        {
            return TestResult::discard();
        }

        if ratio < MIN_RATIO || ratio > MAX_RATIO {
            return TestResult::discard();
        }

        // Convert ratio to a multiplier
        let m = (ratio as u64) << frac;

        let offset = calc_tsc_offset(ihtsc, igtsc, m, frac, int);

        // Catch if the TSC will overflow
        //
        // This check only catches values that *could* overflow, but doesn't
        // guarantee that it would. So there are some valid inputs here that
        // fail this check, but won't produce an error in the calculation.
        //
        // For example, if we have values:
        // - initial_host_tsc: 1 << 62
        // - ratio: 1
        // - guest_TSC: 0
        //
        // initial_host_tsc * multiplier  will always be less than 64 bits,
        // since the ratio is 1. and it won't overflow. But if the ratio were
        // 2, calculating the offset would cause an error.
        //
        let htsc_bits = 64 - ihtsc.leading_zeros();
        let gtsc_bits = 64 - igtsc.leading_zeros();
        if htsc_bits > 60 || gtsc_bits > 60 {
            TestResult::discard()
        } else {
            TestResult::from_bool(offset.is_ok())
        }
    }

    // Check that guest_tsc() doesn't panic, assuming:
    // - guest/host frequencies are > 0
    // - int_size/frac_size are nonzero and fit into 64 bits
    // - current host TSC >= initial host TSC
    #[quickcheck]
    fn guest_tsc_panic_check(
        ihtsc: u64,
        igtsc: u64,
        gf: u64,
        hf: u64,
        chtsc: u64,
        frac: u32,
        int: u32,
    ) -> TestResult {
        if gf == 0 || hf == 0 || frac == 0 || frac >= 64 {
            return TestResult::discard();
        }

        if chtsc < ihtsc {
            return TestResult::discard();
        }

        match int {
            i if i == 0 || i >= 64 || (i + frac) > 64 => TestResult::discard(),
            v => {
                let _ = guest_tsc(ihtsc, igtsc, gf, hf, chtsc, frac, v);

                // if we got here, the function didn't panic, so it passes
                TestResult::from_bool(true)
            }
        }
    }

    // Test that a guest sees the same TSC on two different hosts, for the same point in time
    // (analagous to a migration)
    #[quickcheck]
    fn guest_tsc_same_across_migration(
        // boot host (initial guest TSC: 0)
        boot_htsc: u64,
        cur_htsc: u64,
        boot_hfreq: u64,
        guest_freq: u64,

        // migration host
        migrate_htsc: u64,
        migrate_hfreq: u64,

        frac: u32,
        int: u32,
    ) -> TestResult {
        if frac == 0 || int == 0 || frac >= 64 || int >= 64 || (frac + int) > 64
        {
            return TestResult::discard();
        }

        if boot_hfreq == 0 || guest_freq == 0 || migrate_hfreq == 0 {
            return TestResult::discard();
        }

        if cur_htsc < boot_htsc {
            return TestResult::discard();
        }

        // Guest TSC on source host at migration time
        let src_tsc = guest_tsc(
            boot_htsc, 0, boot_hfreq, guest_freq, cur_htsc, frac, int,
        );

        if src_tsc.is_err() {
            return TestResult::from_bool(true);
        }

        // Guest TSC on dest host at migration time
        let gtsc = src_tsc.unwrap();
        let dst_tsc = guest_tsc(
            migrate_htsc,
            gtsc,
            migrate_hfreq,
            guest_freq,
            migrate_htsc,
            frac,
            int,
        );

        if dst_tsc.is_err() {
            return TestResult::from_bool(true);
        }

        TestResult::from_bool(gtsc == dst_tsc.unwrap())
    }

    // Test that a guest doesn't lose precision one second into the future
    // following a migration
    // TODO: any ratio not an even power of 2 is going to lose some precision
    //#[quickcheck]
    fn guest_tsc_drift(
        // boot host (initial guest TSC: 0)
        boot_htsc: u64,
        cur_htsc: u64,
        boot_hfreq: u64,
        guest_freq: u64,

        // migration host
        migrate_htsc: u64,
        migrate_hfreq: u64,

        frac: u32,
        int: u32,
    ) -> TestResult {
        if frac == 0 || int == 0 || frac >= 64 || int >= 64 || (frac + int) > 64
        {
            return TestResult::discard();
        }

        if boot_hfreq == 0 || guest_freq == 0 || migrate_hfreq == 0 {
            return TestResult::discard();
        }

        if cur_htsc < boot_htsc {
            return TestResult::discard();
        }

        // Guest TSC on source host at migration time
        let src_tsc = guest_tsc(
            boot_htsc, 0, boot_hfreq, guest_freq, cur_htsc, frac, int,
        );
        if src_tsc.is_err() {
            return TestResult::from_bool(true);
        }

        // Guest TSC on dest host at migration time
        let gtsc = src_tsc.unwrap();
        let dst_tsc = guest_tsc(
            migrate_htsc,
            gtsc,
            migrate_hfreq,
            guest_freq,
            migrate_htsc,
            frac,
            int,
        );
        if dst_tsc.is_err() {
            return TestResult::from_bool(true);
        }
        let dst_tsc = dst_tsc.unwrap();

        // Host and Guest TSCs, one second into the future
        let htsc_future = tsc_incr(migrate_htsc, migrate_hfreq);
        let gtsc_future = guest_tsc(
            migrate_htsc,
            dst_tsc,
            migrate_hfreq,
            guest_freq,
            htsc_future,
            frac,
            int,
        );
        if gtsc_future.is_err() {
            return TestResult::from_bool(false);
        }

        // Should have incremented guest frequency in hz
        TestResult::from_bool(
            (gtsc_future.unwrap() - dst_tsc) == (guest_freq * 1000),
        )
    }

    /*
        #[test]
        fn test_freq_ratio() {
            for args in freq_ratio_tests.iter() {
                assert!(matches!(freq_multiplier(args.g, args.h, args.f, 64 - args.f), Ok(val)));
            }

            // 0.5 = 2^-1
            assert!(matches!(freq_multiplier(1000, 2000, 2, 62), Ok(0b10)));
            assert!(matches!(
                freq_multiplier(1000, 2000, 8, 56),
                Ok(0b10000000)
            ));

            // 1.5 = 2^0 + 2^-1
            assert!(matches!(freq_multiplier(3000, 2000, 2, 62), Ok(0b110)));
            assert!(matches!(
                freq_multiplier(3000, 2000, 8, 56),
                Ok(0b110000000)
            ));

            // 0.66 = 2^-1 + 2^-3 + 2^-5 + 2^-7
            assert!(matches!(
                freq_multiplier(2000, 3000, 8, 56),
                Ok(0b10101010)
            ));

            // Intel: 16.48
            let _n = 1u64 << FRAC_SIZE_INTEL;
            assert!(matches!(
                freq_multiplier(1000, 1000, FRAC_SIZE_INTEL, INT_SIZE_INTEL),
                Ok(_n)
            ));

            // AMD: 8.32
            let _n = 1u64 << 32;
            assert!(matches!(
                freq_multiplier(1000, 1000, FRAC_SIZE_AMD, INT_SIZE_AMD),
                Ok(_n)
            ));

            // varied frequency sizes, ratio=1.0
            let _n = 1u64 << 63;
            assert!(matches!(
                freq_multiplier(u64::MAX, u64::MAX, 63, 1),
                Ok(_n)
            ));
            assert!(matches!(freq_multiplier(1000, 1000, 63, 1), Ok(_n)));
            assert!(matches!(freq_multiplier(1, 1, 63, 1), Ok(_n)));
        }

        /*#[test]
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
                freq_multiplier(u64::MAX, u64::MAX, FRAC_SIZE_INTEL, INT_SIZE_INTEL),
                Ok(_n)
            ));

            // int upper bound: 65535
            let _n = 65535u64 << FRAC_SIZE_INTEL;
            assert!(matches!(
                freq_multiplier(65535000, 1000, FRAC_SIZE_INTEL, INT_SIZE_INTEL),
                Ok(_n)
            ));

            /*
             * AMD: 8.32 format
             */
        }*/
    */

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
