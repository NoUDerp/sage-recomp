// Reference harness: Common/System/DeterministicLibm.cpp, the EXACT functions
// (floor / ceil / fmod / sqrt; fabs, ldexp and frexp belong to the same group).
//
// Every result is encoded as the 64 bits of the IEEE-754 double, so the Rust
// port must reproduce the sign of a zero and the exact NaN payload:
//   * DetLibm::sqrt(x < 0) and DetLibm::fmod(+-inf, y) / fmod(x, +-0) return the
//     x86 default NaN 0xFFF8000000000000 (not a positive NaN);
//   * floor / ceil return a NaN operand unchanged (payload and sign preserved),
//     ceil of a negative fraction is -0.0, floor of +-0.0 is +-0.0.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/DeterministicLibm.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "det_floor") {
		return sage_harness::enc_f64(DetLibm::floor(args[0].f64()));
	}
	if (fn == "det_ceil") {
		return sage_harness::enc_f64(DetLibm::ceil(args[0].f64()));
	}
	if (fn == "det_fmod") {
		return sage_harness::enc_f64(DetLibm::fmod(args[0].f64(), args[1].f64()));
	}
	if (fn == "det_sqrt") {
		return sage_harness::enc_f64(DetLibm::sqrt(args[0].f64()));
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
