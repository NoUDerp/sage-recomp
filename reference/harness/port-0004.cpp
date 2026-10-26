// Reference harness: Common/System/Dict.cpp (the small ordered key/value dictionary).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.
//
// Dict is a container, so each case function is value-in/value-out: it builds one
// Dict from the case arguments alone (no shared or global state) and renders the
// result as text. The build program is read from `program`, three bytes per
// instruction, trailing bytes (fewer than three) ignored:
//
//   byte 0 `op`      op % 5 selects the setter
//   byte 1 `key`     the key is "k" + ('0' + key % 5), so k0..k4 collide often
//   byte 2 `payload` the value
//
//     op % 5 == 0  setBool(key, (payload & 1) != 0)
//     op % 5 == 1  setInt(key, (int32)payload - 128)
//     op % 5 == 2  setReal(key, (float)payload * 0.5f)
//     op % 5 == 3  setAsciiString(key, { 'a' + payload % 26, 'a' + (payload / 26) % 26 })
//     op % 5 == 4  setUnicodeString(key, { 0x0041 + payload, 0x4000 + payload })
//
// Repeated keys therefore keep their first position and take the last value and
// the last type, which is exactly the upstream slot() behaviour under test.
//
//   dict_dump(program)            the pairs in order, ";"-joined "<key>=<type>:<value>"
//   dict_lookup(program, key)     the five typed getters for key "k<key>", key 0..7,
//                                 so keys that were never set (and k5..k7, which the
//                                 build program cannot reach) exercise the missing-key
//                                 and wrong-type paths.
//
// Value rendering, identical on both sides:
//   bool     "1" / "0"
//   int      decimal, signed
//   real     8 lowercase hex digits of the float's IEEE-754 bit pattern
//   ascii    lowercase hex of the bytes
//   unicode  4 lowercase hex digits per UTF-16 code unit
//   type     the Dict::DataType code as decimal (0 bool, 1 int, 2 real, 3 ascii, 4 unicode)

#include "common.hpp"

#include "Common/Dict.h"

using sage_harness::Value;

namespace {

std::string hex_u32(std::uint32_t value, int digits)
{
	static const char *kDigits = "0123456789abcdef";
	std::string out;
	for (int shift = (digits - 1) * 4; shift >= 0; shift -= 4) {
		out.push_back(kDigits[(value >> shift) & 0xFu]);
	}
	return out;
}

std::string hex_utf16(const std::u16string &text)
{
	std::string out;
	for (char16_t unit : text) {
		out += hex_u32((std::uint32_t)unit, 4);
	}
	return out;
}

std::string float_bits(float value)
{
	std::uint32_t bits = 0;
	std::memcpy(&bits, &value, sizeof bits);
	return hex_u32(bits, 8);
}

std::string value_text(const Dict::Pair &pair)
{
	switch (pair.type) {
	case Dict::DICT_BOOL:
		return pair.b ? "1" : "0";
	case Dict::DICT_INT:
		return std::to_string(pair.i);
	case Dict::DICT_REAL:
		return float_bits(pair.r);
	case Dict::DICT_ASCIISTRING:
		return sage_harness::enc_bytes(pair.s.data(), pair.s.size());
	case Dict::DICT_UNICODESTRING:
		return hex_utf16(pair.u);
	default:
		return std::string();
	}
}

Dict build(const std::vector<std::uint8_t> &program)
{
	Dict dict;
	for (size_t index = 0; index + 3 <= program.size(); index += 3) {
		const std::uint8_t op = program[index];
		const std::uint8_t keyByte = program[index + 1];
		const std::uint8_t payload = program[index + 2];
		const std::string key = std::string("k") + (char)('0' + (keyByte % 5));
		switch (op % 5) {
		case 0:
			dict.setBool(key, (payload & 1) != 0);
			break;
		case 1:
			dict.setInt(key, (std::int32_t)payload - 128);
			break;
		case 2:
			dict.setReal(key, (float)payload * 0.5f);
			break;
		case 3: {
			std::string text;
			text.push_back((char)('a' + payload % 26));
			text.push_back((char)('a' + (payload / 26) % 26));
			dict.setAsciiString(key, text);
			break;
		}
		default: {
			std::u16string text;
			text.push_back((char16_t)(0x0041 + payload));
			text.push_back((char16_t)(0x4000 + payload));
			dict.setUnicodeString(key, text);
			break;
		}
		}
	}
	return dict;
}

} // namespace

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "dict_dump") {
		const Dict dict = build(args[0].as_bytes());
		std::string out;
		for (const Dict::Pair &pair : dict.pairs()) {
			if (!out.empty()) {
				out.push_back(';');
			}
			out += pair.key;
			out.push_back('=');
			out += std::to_string((int)pair.type);
			out.push_back(':');
			out += value_text(pair);
		}
		return sage_harness::enc_string(out);
	}
	if (fn == "dict_lookup") {
		const Dict dict = build(args[0].as_bytes());
		const std::string key = std::string("k") + (char)('0' + (args[1].u32() % 10));
		bool exists = false;
		std::string out;

		const bool b = dict.getBool(key, &exists);
		out += "b";
		out += exists ? "1" : "0";
		out += b ? "1" : "0";

		const std::int32_t i = dict.getInt(key, &exists);
		out += "|i";
		out += exists ? "1" : "0";
		out += std::to_string(i);

		const float r = dict.getReal(key, &exists);
		out += "|r";
		out += exists ? "1" : "0";
		out += float_bits(r);

		const std::string s = dict.getAsciiString(key, &exists);
		out += "|s";
		out += exists ? "1" : "0";
		out += sage_harness::enc_bytes(s.data(), s.size());

		const std::u16string u = dict.getUnicodeString(key, &exists);
		out += "|u";
		out += exists ? "1" : "0";
		out += hex_utf16(u);

		out += "|n";
		out += std::to_string(dict.getPairCount());
		return sage_harness::enc_string(out);
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
