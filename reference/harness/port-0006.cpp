// Reference harness: Common/NameKeyGenerator.cpp (RotWK string -> name key).
//
// `hash` and `socketOf` are pure statics. `name_keys` drives a *local* generator (never
// the engine singleton): the names are the `;`-separated parts of the input, fed in order
// to `nameToKey` after `init()`, and the row is the key each part received. That covers
// the chain lookup, the case-sensitive compare and the `strcmp`/`c_str()` truncation at an
// embedded NUL without any global state.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/NameKeyGenerator.h"

using sage_harness::Value;

static std::vector<std::string> split_semicolons(const std::string &text)
{
	// One more part than there are separators; empty parts are kept.
	std::vector<std::string> parts;
	std::string current;
	for (char c : text) {
		if (c == ';') {
			parts.push_back(current);
			current.clear();
		} else {
			current.push_back(c);
		}
	}
	parts.push_back(current);
	return parts;
}

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "name_key_hash") {
		return sage_harness::enc_u32(NameKeyGenerator::hash(args[0].c_str()));
	}
	if (fn == "name_key_socket") {
		return sage_harness::enc_u32(NameKeyGenerator::socketOf(args[0].c_str()));
	}
	if (fn == "name_keys") {
		NameKeyGenerator generator;
		generator.init(); // next id := 1, as GameEngine::init leaves it
		std::vector<std::uint32_t> keys;
		for (const std::string &part : split_semicolons(args[0].as_text())) {
			keys.push_back(static_cast<std::uint32_t>(generator.nameToKey(part)));
		}
		return sage_harness::enc_u32list(keys);
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
