// Reference harness: Common/INI/INIMacro.cpp (the #define macro table).
//
// Two entry points:
//   * `macro_hash`  - `INIMacroTable::hash` verbatim (lower-case A-Z, h = h * 5 + (signed
//                     char)c, 32-bit wrap-around).
//   * `macro_value` - the table's observable behaviour without an INI file: `defines` is split
//                     on `;` into one more record than there are separators, each record is
//                     split at its first `=` into a name and a value (a record without `=` is
//                     ignored), every record is handed to `define(name, value, false)` in order
//                     and its verdict discarded (so the first definition of a name wins, as in
//                     the non-reload load types), and the row is `find(name)`'s stored value.
//                     That covers the case-insensitive key and the duplicate rule.
//
// The scanners (`INI::scanInt`/`scanReal`/`parseMath*`) need a live `INI` instance with a text
// stack, so they are out of scope for a pure case runner.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/INI.h"

using sage_harness::Value;

static std::vector<std::string> split_semicolons(const std::string &text)
{
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
	if (fn == "macro_hash") {
		return sage_harness::enc_u32(INIMacroTable::hash(args[0].c_str()));
	}
	if (fn == "macro_value") {
		INIMacroTable table;
		for (const std::string &record : split_semicolons(args[0].as_text())) {
			const size_t split = record.find('=');
			if (split == std::string::npos) {
				continue;
			}
			table.define(record.substr(0, split), record.substr(split + 1), false);
		}
		const INIMacroTable::Entry *entry = table.find(args[1].c_str());
		if (entry == nullptr) {
			return sage_harness::enc_none();
		}
		return sage_harness::enc_string(entry->value);
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
