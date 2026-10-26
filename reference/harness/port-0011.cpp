// Reference harness: Common/RandomValue.cpp (the game-logic RNG).
//
// The upstream class is instance based and carries no state that the cases
// could observe, so each case builds a fresh generator, seeds it with
// `seedRandom(seed)` and draws `draws` times: a pure seed -> sequence mapping.
// Both algorithms are exercised, because upstream refuses to pick one (stop
// S-080): ZH_CarryChain (the six-word add-with-carry chain) and
// RotWK_GameDat_LCG (seed[0] * 0x08088405 + 1, returned xor the high word).
//
// The range entry points call getValue(lo, hi) after `draws` discarded raw
// draws. Note the wrapping arithmetic the retail code uses: delta is
// (uint32)hi - (uint32)lo + 1, a reversed range (hi < lo) therefore produces a
// huge delta, delta == 0 (hi == lo - 1) returns `hi` WITHOUT drawing, and the
// result is (raw % delta) + (uint32)lo computed in 32 bits and reinterpreted as
// a signed int.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/RandomValue.h"

using sage_harness::Value;

static std::uint32_t next_u32(RandomAlgorithm algorithm, std::uint32_t seed, std::uint32_t draws)
{
	GameLogicRandom generator(algorithm);
	generator.seedRandom(seed);
	std::uint32_t value = 0;
	for (std::uint32_t index = 0; index < draws; ++index) {
		value = generator.randomValue();
	}
	return value;
}

static int value_in_range(RandomAlgorithm algorithm, std::uint32_t seed, std::uint32_t draws,
                          int lo, int hi)
{
	GameLogicRandom generator(algorithm);
	generator.seedRandom(seed);
	for (std::uint32_t index = 0; index < draws; ++index) {
		(void)generator.randomValue();
	}
	return generator.getValue(lo, hi, "harness", 0);
}

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "zh_next_u32") {
		return sage_harness::enc_u32(
			next_u32(RandomAlgorithm::ZH_CarryChain, args[0].u32(), args[1].u32()));
	}
	if (fn == "lcg_next_u32") {
		return sage_harness::enc_u32(
			next_u32(RandomAlgorithm::RotWK_GameDat_LCG, args[0].u32(), args[1].u32()));
	}
	if (fn == "zh_value_in_range") {
		return sage_harness::enc_i32(
			value_in_range(RandomAlgorithm::ZH_CarryChain, args[0].u32(), args[1].u32(),
			               args[2].i32(), args[3].i32()));
	}
	if (fn == "lcg_value_in_range") {
		return sage_harness::enc_i32(
			value_in_range(RandomAlgorithm::RotWK_GameDat_LCG, args[0].u32(), args[1].u32(),
			               args[2].i32(), args[3].i32()));
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
