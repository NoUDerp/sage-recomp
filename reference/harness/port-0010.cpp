// Reference harness: Common/System/DeterministicLibm.cpp, the TRANSCENDENTALS
// (sin / cos / atan2 / exp; tan, asin, acos, atan, log, log10 and pow are the
// same group and the same tables, DeterministicLibmTables.inc).
//
// These reproduce glibc 2.39's dbl-64 results with every fused multiply-add
// spelled as DetLibm::fma (one rounding, integer arithmetic), so the Rust port
// must reproduce the bits exactly; the encoding is the 64-bit pattern.
// NaN / signed-zero notes for the contributor:
//   * sin(+-inf) / cos(+-inf) and atan2 with a NaN operand give a NaN; the
//     operand NaN is returned quietened, an invalid operation gives the x86
//     default NaN 0xFFF8000000000000.
//   * exp overflows to +inf (mathOflow) and underflows to +0.0 (mathUflow);
//     sin(-0.0) is -0.0, atan2(+-0.0, x) keeps the sign of y.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/DeterministicLibm.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "det_sin") {
		return sage_harness::enc_f64(DetLibm::sin(args[0].f64()));
	}
	if (fn == "det_cos") {
		return sage_harness::enc_f64(DetLibm::cos(args[0].f64()));
	}
	if (fn == "det_atan2") {
		return sage_harness::enc_f64(DetLibm::atan2(args[0].f64(), args[1].f64()));
	}
	if (fn == "det_exp") {
		return sage_harness::enc_f64(DetLibm::exp(args[0].f64()));
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
