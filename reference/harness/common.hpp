// Shared case runner for sage-recomp reference harnesses.
//
// A harness links against the upstream `libopenbfme_core.a` built by
// `tools/mint_goldens.py`, reads one case per line on stdin
// (`<fn>\t<case>\t<kind>:<payload>...`), and prints `<fn>\t<case>\t<encoding>`.
// The encodings here are the contract the Rust port must reproduce; the same
// encodings are produced by the generated Rust harness in
// `plugins/sage-port/runner.py`.
//
// Owner-only build artefact: never shipped to contributors or verifier guests.

#pragma once

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <functional>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

namespace sage_harness {

enum class Kind { Bytes, Ascii, U32, I32, U64, I64, Usize, F64, Bool, U32List };

struct Value {
    Kind kind = Kind::U32;
    std::vector<uint8_t> bytes;
    std::string text;
    uint64_t unsigned_value = 0;
    int64_t signed_value = 0;
    double float_value = 0.0;
    bool bool_value = false;
    std::vector<uint32_t> list;

    const std::vector<uint8_t>& as_bytes() const { return bytes; }
    const std::string& as_text() const { return text; }
    const char* c_str() const { return text.c_str(); }
    uint32_t u32() const { return static_cast<uint32_t>(unsigned_value); }
    uint64_t u64() const { return unsigned_value; }
    size_t usize() const { return static_cast<size_t>(unsigned_value); }
    int32_t i32() const { return static_cast<int32_t>(signed_value); }
    int64_t i64() const { return signed_value; }
    double f64() const { return float_value; }
    bool boolean() const { return bool_value; }
    const std::vector<uint32_t>& u32list() const { return list; }
};

// ---------------------------------------------------------------- encodings

inline std::string enc_bytes(const void* data, size_t length)
{
    static const char* digits = "0123456789abcdef";
    const uint8_t* raw = static_cast<const uint8_t*>(data);
    std::string out;
    out.reserve(length * 2);
    for (size_t index = 0; index < length; ++index) {
        out.push_back(digits[raw[index] >> 4]);
        out.push_back(digits[raw[index] & 0x0F]);
    }
    return out;
}

inline std::string enc_bytes(const std::vector<uint8_t>& data)
{
    return enc_bytes(data.data(), data.size());
}

inline std::string enc_string(const std::string& text)
{
    return enc_bytes(text.data(), text.size());
}

inline std::string enc_u64(uint64_t value) { return std::to_string(value); }
inline std::string enc_u32(uint32_t value) { return std::to_string(value); }
inline std::string enc_usize(size_t value) { return std::to_string(value); }
inline std::string enc_i64(int64_t value) { return std::to_string(value); }
inline std::string enc_i32(int32_t value) { return std::to_string(value); }
inline std::string enc_bool(bool value) { return value ? "1" : "0"; }

inline std::string enc_f64(double value)
{
    uint64_t bits = 0;
    static_assert(sizeof(bits) == sizeof(value), "double must be 64 bits");
    std::memcpy(&bits, &value, sizeof(bits));
    char buffer[17];
    std::snprintf(buffer, sizeof(buffer), "%016llx",
                  static_cast<unsigned long long>(bits));
    return std::string(buffer);
}

inline std::string enc_u32list(const std::vector<uint32_t>& values)
{
    if (values.empty()) {
        return "-";
    }
    std::string out;
    for (size_t index = 0; index < values.size(); ++index) {
        if (index) {
            out.push_back(',');
        }
        out += std::to_string(values[index]);
    }
    return out;
}

inline std::string enc_none() { return "-"; }
inline std::string enc_error() { return "!"; }

// ------------------------------------------------------------------ parsing

inline std::vector<uint8_t> parse_hex(const std::string& text)
{
    if (text.size() % 2 != 0) {
        throw std::runtime_error("odd-length hex payload");
    }
    std::vector<uint8_t> out;
    out.reserve(text.size() / 2);
    for (size_t index = 0; index < text.size(); index += 2) {
        out.push_back(static_cast<uint8_t>(std::stoul(text.substr(index, 2), nullptr, 16)));
    }
    return out;
}

inline Value parse_token(const std::string& token)
{
    const size_t colon = token.find(':');
    if (colon == std::string::npos) {
        throw std::runtime_error("argument token has no kind: " + token);
    }
    const std::string kind = token.substr(0, colon);
    const std::string payload = token.substr(colon + 1);
    Value value;
    if (kind == "bytes") {
        value.kind = Kind::Bytes;
        value.bytes = parse_hex(payload);
    } else if (kind == "ascii") {
        value.kind = Kind::Ascii;
        const std::vector<uint8_t> raw = parse_hex(payload);
        value.text.assign(raw.begin(), raw.end());
        value.bytes = raw;
    } else if (kind == "u32" || kind == "u64" || kind == "usize") {
        value.kind = kind == "u32" ? Kind::U32 : (kind == "u64" ? Kind::U64 : Kind::Usize);
        value.unsigned_value = std::stoull(payload);
    } else if (kind == "i32" || kind == "i64") {
        value.kind = kind == "i32" ? Kind::I32 : Kind::I64;
        value.signed_value = std::stoll(payload);
    } else if (kind == "f64") {
        value.kind = Kind::F64;
        const uint64_t bits = std::stoull(payload, nullptr, 16);
        std::memcpy(&value.float_value, &bits, sizeof(value.float_value));
    } else if (kind == "bool") {
        value.kind = Kind::Bool;
        value.bool_value = payload == "1";
    } else if (kind == "u32list") {
        value.kind = Kind::U32List;
        if (payload != "-") {
            std::stringstream stream(payload);
            std::string item;
            while (std::getline(stream, item, ',')) {
                value.list.push_back(static_cast<uint32_t>(std::stoul(item)));
            }
        }
    } else {
        throw std::runtime_error("unknown argument kind " + kind);
    }
    return value;
}

using CaseFn = std::function<std::string(const std::string&, const std::vector<Value>&)>;

inline int harness_main(const CaseFn& run)
{
    std::ios::sync_with_stdio(false);
    std::string line;
    std::string out;
    while (std::getline(std::cin, line)) {
        if (line.empty()) {
            continue;
        }
        std::vector<std::string> fields;
        std::string field;
        std::stringstream stream(line);
        while (std::getline(stream, field, '\t')) {
            fields.push_back(field);
        }
        if (fields.size() < 2) {
            std::cerr << "malformed case line\n";
            return 2;
        }
        std::vector<Value> args;
        for (size_t index = 2; index < fields.size(); ++index) {
            args.push_back(parse_token(fields[index]));
        }
        std::string encoded;
        try {
            encoded = run(fields[0], args);
        } catch (const std::exception& error) {
            std::cerr << "case " << fields[0] << '#' << fields[1]
                      << " threw: " << error.what() << '\n';
            return 3;
        }
        out += fields[0];
        out.push_back('\t');
        out += fields[1];
        out.push_back('\t');
        out += encoded;
        out.push_back('\n');
    }
    std::cout << out;
    std::cout.flush();
    return 0;
}

}  // namespace sage_harness

#define SAGE_HARNESS_MAIN(run_case)                                           \
    int main()                                                                \
    {                                                                         \
        return sage_harness::harness_main(run_case);                          \
    }
