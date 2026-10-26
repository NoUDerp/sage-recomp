// Reference harness: Common/Sha256.cpp (FIPS 180-4 SHA-256, RFC 2104 HMAC-SHA256).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.
//
// The three case functions are the module's pure entry points:
//   sha256_hex(data)                -> HexBytes(Sha256::hash(data))
//   sha256_stream_hex(first,second) -> HexBytes of two update() calls then finish()
//   hmac_sha256_hex(key,data)       -> HexBytes(HmacSha256(key, data))
// Keys run past 64 bytes so the HMAC key-shortening path is exercised.
// ConstantTimeEqual is deliberately not a case function: over independent random
// arguments it answers "not equal" for essentially every case, which the mint's
// degeneracy gate rejects (and rightly: such a case group proves nothing).

#include "common.hpp"

#include "Common/Sha256.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "sha256_hex") {
		const std::vector<std::uint8_t> &data = args[0].as_bytes();
		std::array<std::uint8_t, 32> digest = Sha256::hash(data.data(), data.size());
		return sage_harness::enc_string(HexBytes(digest.data(), digest.size()));
	}
	if (fn == "sha256_stream_hex") {
		const std::vector<std::uint8_t> &first = args[0].as_bytes();
		const std::vector<std::uint8_t> &second = args[1].as_bytes();
		Sha256 state;
		state.update(first.data(), first.size());
		state.update(second.data(), second.size());
		std::array<std::uint8_t, 32> digest = state.finish();
		return sage_harness::enc_string(HexBytes(digest.data(), digest.size()));
	}
	if (fn == "hmac_sha256_hex") {
		const std::vector<std::uint8_t> &key = args[0].as_bytes();
		const std::vector<std::uint8_t> &data = args[1].as_bytes();
		std::array<std::uint8_t, 32> mac = HmacSha256(key.data(), key.size(), data.data(), data.size());
		return sage_harness::enc_string(HexBytes(mac.data(), mac.size()));
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
