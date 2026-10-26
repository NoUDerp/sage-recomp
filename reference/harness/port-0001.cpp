// Reference harness: Common/MD5.cpp (RFC 1321 MD5).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/MD5.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "md5_hex") {
		const std::vector<std::uint8_t> &data = args[0].as_bytes();
		return sage_harness::enc_string(MD5::ofBytes(data.data(), data.size()));
	}
	if (fn == "md5_stream_hex") {
		MD5 digest;
		const std::vector<std::uint8_t> &first = args[0].as_bytes();
		const std::vector<std::uint8_t> &second = args[1].as_bytes();
		digest.update(first.data(), first.size());
		digest.update(second.data(), second.size());
		return sage_harness::enc_string(digest.finalHex());
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
