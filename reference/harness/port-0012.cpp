// Reference harness: Common/System/NumericState.cpp, the x87 PC24 emulation
// (the three rounding kernels kept WIDE in the register plus the narrowing
// `fstp dword` store).
//
// pc24AddW / pc24MulW / pc24DivW round one operation to a 24-bit significand
// and keep the result as a binary64-bounded carrier (upstream stop S-233: the
// exponent range is the double range, not the x87 register's 15 bits, so
// (2^600 * 2^600) / 2^600 is +inf here). fstpDword rounds a register value ONCE
// to binary32 (nearest even, subnormals included, overflow to infinity).
//
// Encoding notes for the contributor:
//   * the *W results are encoded as the 64 bits of the double. A NaN result is
//     the emulation's canonical quiet NaN 0x7FF8000000000000, with the sign bit
//     set when the x87 default NaN applies (inf - inf, 0 * inf, 0 / 0, inf/inf
//     -> 0xFFF8000000000000); zeros keep their sign.
//   * fstp_dword_bits returns the raw binary32 BIT PATTERN as a u32 (not a
//     float), so no float -> double NaN widening can hide a payload mismatch.
//     A NaN there is the emulation's 0xFFC00000-style quiet NaN.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/NumericState.h"

#include <cstring>

using sage_harness::Value;

static void ensure_canonical_environment()
{
	// Retail setFPMode: the canonical MXCSR / x87 control word the fast path of
	// the emulation requires. Both paths produce the same bits, but minting runs
	// in the environment upstream documents.
	static const bool once = [] {
		NumericState::setFPMode();
		return true;
	}();
	(void)once;
}

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	ensure_canonical_environment();
	if (fn == "pc24_add_w") {
		return sage_harness::enc_f64(NumericState::pc24AddW(args[0].f64(), args[1].f64()));
	}
	if (fn == "pc24_mul_w") {
		return sage_harness::enc_f64(NumericState::pc24MulW(args[0].f64(), args[1].f64()));
	}
	if (fn == "pc24_div_w") {
		return sage_harness::enc_f64(NumericState::pc24DivW(args[0].f64(), args[1].f64()));
	}
	if (fn == "fstp_dword_bits") {
		const float stored = NumericState::fstpDword(args[0].f64());
		std::uint32_t bits = 0;
		std::memcpy(&bits, &stored, sizeof(bits));
		return sage_harness::enc_u32(bits);
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
