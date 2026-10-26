// Reference harness: Common/MpGameRules.cpp (lobby game rules, lane RULES-1).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.
//
// Every entry point is a free function over constant tables; no GameInfo, no GlobalData (the
// demo mode is a parameter, see MpGameRules::defaults).

#include "common.hpp"

#include "Common/MpGameRules.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "mp_rules_defaults") {
		const GameRules rules = MpGameRules::defaults(args[0].i32(), args[1].i32());
		std::vector<std::uint32_t> out;
		out.reserve(rules.size());
		for (std::int32_t rule : rules) {
			out.push_back(static_cast<std::uint32_t>(rule)); // two's complement: -1 -> 4294967295
		}
		return sage_harness::enc_u32list(out);
	}
	if (fn == "mp_rules_label") {
		const char *label = MpGameRules::ruleLabel(args[0].i32());
		return label ? sage_harness::enc_string(label) : sage_harness::enc_none();
	}
	if (fn == "mp_rules_choice_value") {
		const MpRuleCombo *combo = MpGameRules::findCombo(args[0].i32(), args[1].i32());
		if (!combo) {
			return sage_harness::enc_none();
		}
		const std::uint32_t choice = args[2].u32();
		if (choice >= static_cast<std::uint32_t>(combo->count)) {
			return sage_harness::enc_none();
		}
		return sage_harness::enc_i32(combo->choices[choice].value);
	}
	if (fn == "mp_legendary_damage_factor") {
		// the float constant widened to double; the port must reproduce the same bits
		return sage_harness::enc_f64(
			static_cast<double>(LegendaryHeroes::damageDealtFactor(args[0].i32())));
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
