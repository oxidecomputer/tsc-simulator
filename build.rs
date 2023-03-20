fn main() {
    cc::Build::new()
        .file("src/asm_math.s")
        .compile("asm_math.a");
}
