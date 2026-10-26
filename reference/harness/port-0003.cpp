// Reference harness: Common/System/XferCRC.cpp (the save-game CRC).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.
//
// XferCRC accumulates every transfer into Common/StateHash.h's StateHasher
// (`crc = rotl32(crc, 1) + word`, zero-initialised): whole little-endian words
// first, then one rotate-add per trailing byte, and the word grouping RESTARTS
// at every transfer. The three case functions are exactly that contract:
//   xfer_crc_bytes(data)          one transfer of the whole buffer
//   xfer_crc_chunks(first,second) two transfers (regrouping at the boundary)
//   xfer_crc_words(values)        one 4-byte transfer per u32, little-endian
// Only Xfer::xferUser (a raw-byte transfer, inline in Common/Xfer.h) is used, so
// the port stays inside XferCRC.cpp + StateHash.h.

#include "common.hpp"

#include "Common/XferCRC.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "xfer_crc_bytes") {
		std::vector<std::uint8_t> data = args[0].as_bytes();
		XferCRC crc;
		crc.xferUser(data.data(), data.size());
		return sage_harness::enc_u32(crc.getCRC());
	}
	if (fn == "xfer_crc_chunks") {
		std::vector<std::uint8_t> first = args[0].as_bytes();
		std::vector<std::uint8_t> second = args[1].as_bytes();
		XferCRC crc;
		crc.xferUser(first.data(), first.size());
		crc.xferUser(second.data(), second.size());
		return sage_harness::enc_u32(crc.getCRC());
	}
	if (fn == "xfer_crc_words") {
		const std::vector<std::uint32_t> &values = args[0].u32list();
		XferCRC crc;
		for (std::uint32_t value : values) {
			std::uint8_t little[4] = {
				(std::uint8_t)(value & 0xFFu),
				(std::uint8_t)((value >> 8) & 0xFFu),
				(std::uint8_t)((value >> 16) & 0xFFu),
				(std::uint8_t)((value >> 24) & 0xFFu),
			};
			crc.xferUser(little, sizeof little);
		}
		return sage_harness::enc_u32(crc.getCRC());
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
