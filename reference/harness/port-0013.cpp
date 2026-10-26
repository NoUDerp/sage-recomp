// Reference harness: Common/Thing/KindOfTokens.cpp (KindOf token <-> name/bit mapping).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.
//
// The retail name table (GameLogic/BitFlagNames.cpp, RW 0xDA0E68, 222 names) is the table in
// force while no mod engine feature is on, which is the state of a freshly loaded process:
// ModEngineFeatures::has() reads zero-initialised atomics, so names() is TheKindOfNames.

#include "common.hpp"

#include "Common/Thing/KindOfTokens.h"

#include <array>

using sage_harness::Value;

static int name_count()
{
	const char *const *names = KindOfTokens::names();
	int count = 0;
	while (names[count]) {
		++count;
	}
	return count;
}

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	const std::uint32_t index = args[0].u32();
	const int count = name_count();
	const bool known = index < static_cast<std::uint32_t>(count);
	// the lowest index whose folded name equals names()[index]: what KindOfTokens::indexOf
	// returns for that token, -1 when the index names nothing
	const int bit = known ? KindOfTokens::indexOf(KindOfTokens::names()[index]) : -1;

	if (fn == "kind_of_name") {
		if (!known) {
			return sage_harness::enc_none();
		}
		return sage_harness::enc_string(KindOfTokens::names()[index]);
	}
	if (fn == "kind_of_canonical_index") {
		return sage_harness::enc_i32(bit);
	}
	if (fn == "kind_of_bit_mask") {
		// the 224 bit KindOf mask a single plain token produces (KindOfTokens::parse: the mask
		// is cleared, then mask[bit >> 5] |= 1 << (bit & 31)); all zero for an unknown name
		std::array<std::uint32_t, 7> mask{};
		mask.fill(0);
		if (bit >= 0) {
			mask[static_cast<std::size_t>(bit) >> 5] |= 1u << (bit & 31);
		}
		return sage_harness::enc_u32list(std::vector<std::uint32_t>(mask.begin(), mask.end()));
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
