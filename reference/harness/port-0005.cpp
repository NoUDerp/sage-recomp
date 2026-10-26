// Reference harness: Common/System/PathIO.cpp (install-path validation).
//
// Only `PathIO::checkInstallPath` is both pure and portable: `native`, `environment`,
// `open` and `runMain` either touch the process environment / the filesystem or compile
// to the identity on POSIX. The reference library is built for POSIX, so the `_WIN32`
// half of `checkInstallPath` (the trailing space/dot scan) is not part of the contract.
//
// Owner-only: compiled against libopenbfme_core.a by tools/mint_goldens.py.

#include "common.hpp"

#include "Common/PathIO.h"

using sage_harness::Value;

static std::string run_case(const std::string &fn, const std::vector<Value> &args)
{
	if (fn == "check_install_path") {
		// `as_text()` keeps embedded NUL bytes, which is exactly what the check looks for.
		std::string error;
		if (PathIO::checkInstallPath(args[0].as_text(), error)) {
			return sage_harness::enc_none();
		}
		return sage_harness::enc_string(error);
	}
	throw std::runtime_error("unknown case function " + fn);
}

SAGE_HARNESS_MAIN(run_case)
