pub struct FrTest {
    pub g: u64,
    pub h: u64,
    pub f: u8,
    pub val: u64,
}

pub const _FREQ_RATIO_TESTS: &'static [FrTest] = &[
    // 0.5 = 2^-1
    FrTest {
        g: 1000,
        h: 2000,
        f: 2,
        val: 0b10,
    },
    FrTest {
        g: 1000,
        h: 2000,
        f: 8,
        val: 0b10000000,
    },
    //1.5 = 2^0 + 2^-1
    FrTest {
        g: 3000,
        h: 2000,
        f: 2,
        val: 0b110,
    },
    FrTest {
        g: 3000,
        h: 2000,
        f: 8,
        val: 0b110000000,
    },
];
