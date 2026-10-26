// Reference harness: Common/Audio/Mp3Tables.cpp (Layer III constant tables).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.
//
// Pure constant data: the ISO 11172-3 Table B.7 Huffman tables (code length / code, row-major
// [x * n + y]), the two count1 quadruple tables and the Table B.3 synthesis window.

#include "common.hpp"

#include "Common/Audio/Mp3Tables.h"

using sage_harness::Value;
using namespace AudioDecode::mp3tab;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "mp3_huffman_shape") {
		const std::uint32_t table = args[0].u32();
		if (table >= static_cast<std::uint32_t>(kHuffmanTableCount)) {
			return sage_harness::enc_u32list({});
		}
		const HuffmanSource &source = kHuffmanSources[table];
		return sage_harness::enc_u32list({ static_cast<std::uint32_t>(source.tableNumber),
		                                   static_cast<std::uint32_t>(source.n) });
	}
	if (fn == "mp3_huffman_len_code") {
		const std::uint32_t table = args[0].u32();
		const std::uint32_t index = args[1].u32();
		if (table >= static_cast<std::uint32_t>(kHuffmanTableCount)) {
			return sage_harness::enc_u32list({});
		}
		const HuffmanSource &source = kHuffmanSources[table];
		if (index >= static_cast<std::uint32_t>(source.n * source.n)) {
			return sage_harness::enc_u32list({});
		}
		return sage_harness::enc_u32list({ static_cast<std::uint32_t>(source.len[index]),
		                                   source.code[index] });
	}
	if (fn == "mp3_quad_len_code") {
		const std::uint32_t select = args[0].u32();
		const std::uint32_t index = args[1].u32();
		if (select >= 2u || index >= 16u) {
			return sage_harness::enc_u32list({});
		}
		const QuadSource &source = kQuadSources[select];
		return sage_harness::enc_u32list({ static_cast<std::uint32_t>(source.len[index]),
		                                   static_cast<std::uint32_t>(source.code[index]) });
	}
	if (fn == "mp3_synth_window") {
		const std::uint32_t index = args[0].u32();
		if (index >= 257u) {
			return sage_harness::enc_none();
		}
		return sage_harness::enc_i32(kSynthWindow257[index]);
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
