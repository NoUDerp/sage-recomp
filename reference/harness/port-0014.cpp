// Reference harness: Common/CommandPoints.cpp (command point arithmetic, lane ECON-1).
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.
//
// Every entry point builds a CommandPoints from the case arguments alone: the records carry no
// filter (a null filter is never `valid`, so every record counts and getLimit needs no world).

#include "common.hpp"

#include "Common/CommandPoints.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "cp_limit") {
		CommandPoints points;
		points.reset();
		points.setFromScript(args[0].i32(), args[2].i32()); // base, cap
		points.addBonus(args[1].i32());
		const std::vector<std::uint32_t> &values = args[3].u32list();
		for (std::size_t i = 0; i < values.size(); ++i) {
			points.addRecord(static_cast<std::int32_t>(values[i]),
			                 static_cast<std::uint32_t>(i), nullptr);
		}
		const int limit = points.getLimit([](const ObjectFilter &) { return false; });
		return sage_harness::enc_i32(limit);
	}
	if (fn == "cp_can_afford") {
		CommandPoints points;
		points.reset();
		points.addUsage(args[0].i32());
		return sage_harness::enc_bool(points.canAfford(args[1].i32(), args[2].boolean(),
		                                               args[3].i32()));
	}
	if (fn == "cp_init_cap") {
		CommandPoints points;
		points.reset();
		if (args[1].boolean()) {
			points.setFromScript(0, args[0].i32());
		}
		CommandPointsSource source;
		source.start = 0;
		source.cap = args[2].i32();
		source.lobbyPercent = args[3].i32();
		source.applyLobbyPercent = args[4].boolean();
		points.init(0, source);
		return sage_harness::enc_i32(points.getCap());
	}
	if (fn == "cp_init_base") {
		CommandPoints points;
		points.reset();
		if (args[1].boolean()) {
			points.setFromScript(args[0].i32(), 0);
		}
		CommandPointsSource source;
		source.start = args[2].i32();
		source.territoryGoodTerm = args[3].i32();
		source.territoryEvilTerm = args[4].i32();
		source.cap = 0;
		points.init(0, source);
		return sage_harness::enc_i32(points.getBase());
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
