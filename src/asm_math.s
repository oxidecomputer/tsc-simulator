.text

.global calc_freq_multiplier
.global scale_tsc

/*
 * calc_freq_multiplier: calculates the ratio of guest_hz / host_hz, with
 * `frac_size` fractional bits.
 *
 * (guest_hz * (1 << FRAC_SIZE)) / host_hz
 *
 * %rdi: uint64_t guest_hz
 * %rsi: uint64_t host_hz
 * %rdx: uint8_t frac_size
 */
calc_freq_multiplier:
	/*
	 * Create scaling factor: 1 << frac_size
	 * Store result in %rax
	 */
	xor %rax, %rax
	movq $1, %rax
	mov %dl, %cl
	shlq %cl, %rax

	/*
	 * Multiply: guest_hz (%rdi) * scaling_factor (%rax)
	 * Result is in RDX:RAX
	 */
	mulq %rdi

	/*
	 * Divide: result by host_hz (%rdi)
	 */
	divq %rsi
	ret


/*
 * scale_tsc: Scales a TSC value based on a frequency multiplier.
 *
 * (tsc * multiplier) >> FRAC_SIZE
 *
 * %rdi: uint64_t tsc
 * %rsi: uint64_t multiplier
 * %rdx: uint8_t  frac_size
 *
 */
scale_tsc:
	/* Save `frac_size` in %cl */
	xor %rcx, %rcx
	mov %dl, %cl

	/*
	 * Multiply tsc (%rdi) * multiplier (%rax)
	 * mulq result is in RDX:RAX
	 */
	movq %rdi, %rax
	mulq %rsi


	/*
	 * Shift the 128-bit product right `frac_size` bits:
	 * - shift lower 64 bits right, `frac_size` bits
	 * - shift upper 64 bits left, (64 - `frac_size`) bits
	 * - bitwise OR upper bits and lower bits
	 */

	/* Shift lower 64 bits right `frac_size` */
	shrq %cl, %rax

	/* Compute 64 - FRAC_SIZE and store result in %cl */
	xor %r9, %r9
	movq %rcx, %r9
	xor %rcx, %rcx
	mov $64, %cl
	subq %r9, %rcx

	/* Shift upper 64 bits right `frac_size` */
	shlq %cl, %rdx

	/* Bitwise OR upper and lower bits */
	orq %rdx, %rax

	ret
