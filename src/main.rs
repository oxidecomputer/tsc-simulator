// A tool for calculating time-related values

use crate::math::*;

use anyhow::anyhow;
use clap::{clap_derive::ArgEnum, Parser, Subcommand};
use clap_num::maybe_hex;

mod asm_math;
mod math;
mod tests;

/// TSC Simulator
#[derive(Debug, Parser)]
struct Opt {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Clone, ArgEnum)]
enum Arch {
    Amd,
    Intel,
}

#[derive(Debug, Copy, Clone, ArgEnum)]
enum MathImpl {
    Asm,
    Rust,
    All,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Calculate a specific value
    Calc {
        #[clap(subcommand)]
        cmd: CalcCommand,
    },

    /// Simulate what TSC values a host and guest have over time
    Simulate {
        /// Duration (seconds)
        #[clap(short = 'd', long, default_value = "20")]
        duration: usize,

        /// Initial Host TSC value
        #[clap(short = 'i', default_value = "1000000000")]
        initial_host_tsc: u64,

        /// Initial Host Frequency (Hz)
        #[clap(short = 'f', default_value = "1000000000")]
        initial_host_hz: u64,

        /// Guest Frequency (Hz)
        #[clap(short = 'g', long, default_value = "1000000000")]
        guest_hz: u64,

        /// Migrate to host at t seconds: "<t> <host_tsc> <host_hz>"
        #[clap(long = "migrate")]
        hosts: Vec<String>,

        /// Architecture of hosts
        #[clap(long, arg_enum, default_value = "amd")]
        arch: Arch,

        /// Calculate related values in assembly, rust, or both
        #[clap(short = 'm', arg_enum, default_value = "rust")]
        math_impl: MathImpl,

        /// Print TSC values as hexadecimal
        #[clap(long, takes_value = false)]
        hex: bool,
    },
}

// Host specification for simulation boot/migration of a guest
#[derive(Debug)]
struct HostDef {
    start: usize,
    host_tsc: u64,
    host_freq: u64,
}

#[derive(Debug, Subcommand)]
enum CalcCommand {
    /// Given a TSC value and a frequency, compute hrtime (nanoseconds)
    Hrtime {
        /// TSC value
        #[clap(short = 't', value_parser=maybe_hex::<u64>)]
        tsc: u64,

        /// Frequency (Hz)
        #[clap(short = 'f', value_parser=maybe_hex::<u64>, default_value = "1000000000")]
        freq_hz: u64,
    },

    /// Given an hrtime and a frequency, compute TSC value
    Tsc {
        /// hrtime (nanoseconds)
        #[clap(short = 't', value_parser=maybe_hex::<u64>)]
        hrtime: u64,

        /// Frequency (Hz)
        #[clap(short = 'f', value_parser=maybe_hex::<u64>, default_value = "1000000000")]
        freq_hz: u64,
    },

    /// Compute a guest's TSC value
    GuestTsc {
        /// Initial Host TSC value (at boot or time of migration)
        #[clap(short = 'i', value_parser=maybe_hex::<u64>)]
        initial_host_tsc: u64,

        /// Initial Guest TSC value
        #[clap(short = 't', value_parser=maybe_hex::<u64>, default_value = "0")]
        initial_guest_tsc: u64,

        /// Current Host TSC value
        host_tsc: u64,

        /// Host Frequency (Hz)
        #[clap(short = 'f', value_parser=maybe_hex::<u64>, default_value = "1000000000")]
        host_hz: u64,

        /// Guest Frequency (Hz)
        #[clap(short = 'g', value_parser=maybe_hex::<u64>, default_value = "1000000000")]
        guest_hz: u64,

        /// Calculate related values in assembly, rust, or both
        #[clap(short = 'm', arg_enum, default_value = "rust")]
        math_impl: MathImpl,

        // AMD defaults
        #[clap(long, default_value = "8")]
        int_size: u8,
        #[clap(long, default_value = "32")]
        frac_size: u8,
    },

    /// Compute a guest's TSC offset
    Offset {
        /// Initial Host TSC value
        #[clap(value_parser=maybe_hex::<u64>)]
        initial_host_tsc: u64,

        /// Initial Guest TSC value
        #[clap(short = 't', value_parser=maybe_hex::<u64>, default_value = "0")]
        initial_guest_tsc: u64,

        /// Guest Frequency (Hz)
        #[clap(short = 'g', value_parser=maybe_hex::<u64>, default_value = "1000000000")]
        guest_hz: u64,

        /// Host Frequency (Hz)
        #[clap(short = 'f', value_parser=maybe_hex::<u64>, default_value = "1000000000")]
        host_hz: u64,

        /// Calculate related values in assembly, rust, or both
        #[clap(short = 'm', arg_enum, default_value = "rust")]
        math_impl: MathImpl,

        // AMD defaults
        #[clap(long, default_value = "8")]
        int_size: u8,
        #[clap(long, default_value = "32")]
        frac_size: u8,
    },

    /// Compute the frequency multiplier for a guest and a host
    Freq {
        /// Host Frequency (Hz)
        #[clap(short = 'f', value_parser=maybe_hex::<u64>)]
        host_hz: u64,

        /// Guest Frequency (Hz)
        #[clap(short = 'g', value_parser=maybe_hex::<u64>)]
        guest_hz: u64,

        /// Number of int bits in multiplier
        #[clap(long, default_value = "8")]
        int_size: u8,

        /// Number of frac bits in multiplier
        #[clap(long, default_value = "32")]
        frac_size: u8,

        /// Calculate related values in assembly, rust, or both
        #[clap(short = 'm', arg_enum, default_value = "rust")]
        math_impl: MathImpl,
    },
}

fn cmd_simulate(
    duration: usize,
    guest_hz: u64,
    hosts: Vec<HostDef>,
    arch: Arch,
    math_impl: MathImpl,
    print_hex: bool,
) {
    assert!(hosts.len() > 0);

    println!(" {:<15} {} {:<30}", "DURATION", duration, "seconds");
    println!(" {:>15} {} {:<30}", "GUEST FREQUENCY", guest_hz, "Hz");
    println!("");
    for (i, h) in hosts.iter().enumerate() {
        println!(" {:<15}", format!("HOST {}", i));
        println!(" {:>15} {} {:<30}", "START TIME", h.start, "seconds");
        println!(" {:>15} {:<30}", "TSC", h.host_tsc);
        println!(" {:>15} {} {:<30}", "FREQUENCY", h.host_freq, "Hz");
        println!("");
    }
    println!("");

    let (int_size, frac_size) = match arch {
        Arch::Amd => (INT_SIZE_AMD, FRAC_SIZE_AMD),
        Arch::Intel => (INT_SIZE_INTEL, FRAC_SIZE_INTEL),
    };
    let num_hosts = hosts.len();
    let mut start_guest_tsc = 0;
    let mut cur_guest_tsc = start_guest_tsc;

    println!("{:<10} {:>16} {:>16}", "TIME", "GUEST_TSC", "HOST_TSC");

    for h in 0..num_hosts {
        let start = hosts[h].start;

        // end time is either: the duration, or the start of the next host
        let end = if num_hosts == (h + 1) {
            duration
        } else {
            hosts[h + 1].start
        };

        let start_host_tsc = hosts[h].host_tsc;
        let host_hz = hosts[h].host_freq;
        let desc = if h == 0 {
            "GUEST_BOOT ".to_string()
        } else {
            format!("MIGRATION {} ", h)
        };

        // print the header for this host
        println!("=== {desc:=<77}");

        let mut cur_host_tsc = start_host_tsc;

        for t in start..=end {
            // find the guest TSC for this point in time
            match guest_tsc(
                start_host_tsc,
                start_guest_tsc,
                host_hz,
                guest_hz,
                cur_host_tsc,
                frac_size,
                int_size,
            ) {
                Ok(tsc) => {
                    cur_guest_tsc = tsc;
                }
                Err(e) => {
                    eprintln!("could not calculate guest tsc: {}", e);
                    return;
                }
            }

            // print the host and guest TSC values
            if print_hex {
                println!("{:<10} {:#16x} {:#16x}", t, cur_guest_tsc, cur_host_tsc,);
            } else {
                println!("{:<10} {:#16} {:#16}", t, cur_guest_tsc, cur_host_tsc,);
            }

            cur_host_tsc = tsc_incr(cur_host_tsc, host_hz);
            cmd_simulate(duration, guest_hz, host_defs, arch, math_impl, hex);
        }
    }
}
