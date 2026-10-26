// Reference harness: Common/MiniJson.cpp (strict mini JSON reader).
//
// Two entry points:
//   * `parse_error`  - `JsonValue::parse` verbatim: the upstream error message
//                      ("JSON error at byte <n>: <reason>") or none when the text parses.
//   * `canonicalise` - parse, then re-emit the value in a canonical, encoding-free form so
//                      that the whole parsed tree is observable:
//                        null              -> `null`
//                        true / false      -> `true` / `false`
//                        number            -> `#` + 16 lowercase hex digits of the IEEE-754
//                                             bit pattern of the double
//                        string            -> `"` + lowercase hex of its bytes + `"`
//                        array             -> `[` items `,`-joined `]`
//                        object            -> `{` key `:` value pairs `,`-joined `}`, the
//                                             keys rendered like strings and ordered the way
//                                             `std::map<std::string>` orders them (ascending
//                                             unsigned byte order)
//                      Hex avoids every escaping question and keeps the result ASCII even
//                      when a `\u` escape produced bytes that are not valid UTF-8.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/MiniJson.h"

#include <cstring>

using sage_harness::Value;

static std::string hex_of(const std::string &text)
{
	return sage_harness::enc_bytes(text.data(), text.size());
}

static void emit(const JsonValue &value, std::string &out)
{
	switch (value.type) {
	case JsonValue::NUL:
		out += "null";
		return;
	case JsonValue::BOOL:
		out += value.boolean ? "true" : "false";
		return;
	case JsonValue::NUMBER: {
		std::uint64_t bits = 0;
		std::memcpy(&bits, &value.number, sizeof(bits));
		char buffer[17];
		std::snprintf(buffer, sizeof(buffer), "%016llx",
		              static_cast<unsigned long long>(bits));
		out.push_back('#');
		out += buffer;
		return;
	}
	case JsonValue::STRING:
		out.push_back('"');
		out += hex_of(value.string);
		out.push_back('"');
		return;
	case JsonValue::ARRAY:
		out.push_back('[');
		for (size_t index = 0; index < value.array.size(); ++index) {
			if (index) {
				out.push_back(',');
			}
			emit(value.array[index], out);
		}
		out.push_back(']');
		return;
	case JsonValue::OBJECT: {
		out.push_back('{');
		bool first = true;
		for (const auto &item : value.object) {
			if (!first) {
				out.push_back(',');
			}
			first = false;
			out.push_back('"');
			out += hex_of(item.first);
			out += "\":";
			emit(item.second, out);
		}
		out.push_back('}');
		return;
	}
	}
	throw std::runtime_error("unreachable JSON value type");
}

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "parse_error") {
		JsonValue value;
		std::string error;
		if (JsonValue::parse(args[0].as_text(), value, &error)) {
			return sage_harness::enc_none();
		}
		return sage_harness::enc_string(error);
	}
	if (fn == "canonicalise") {
		JsonValue value;
		if (!JsonValue::parse(args[0].as_text(), value, nullptr)) {
			return sage_harness::enc_none();
		}
		std::string out;
		emit(value, out);
		return sage_harness::enc_string(out);
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
