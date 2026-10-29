#include <ctranslate2/translator.h>
#include <ctranslate2/logging.h>
#include <filesystem>
#include <fstream>
#include <cstring>
#include <memory>

// Keep the ABI limited to opaque handles and UTF-8 buffers. All allocations
// crossing this boundary are released by jy_free in the same C++ runtime.
namespace {
char* copy_string(const std::string& text) {
  auto out = new char[text.size() + 1];
  std::memcpy(out, text.c_str(), text.size() + 1);
  return out;
}
class UnicodeReader final : public ctranslate2::models::ModelReader {
  std::filesystem::path root;
  std::string id;
public:
  explicit UnicodeReader(const std::string& path) : root(std::filesystem::u8path(path)), id(path) {}
  std::string get_model_id() const override { return id; }
  std::unique_ptr<std::istream> get_file(const std::string& filename, bool binary) override {
    auto file = std::make_unique<std::ifstream>(root / std::filesystem::u8path(filename),
      std::ios::in | (binary ? std::ios::binary : std::ios::openmode(0)));
    if (!*file) return nullptr;
    return file;
  }
};
}
extern "C" {
int jy_model_new(const char* path, void** handle, char** error) noexcept {
  *handle = nullptr; *error = nullptr;
  try {
    ctranslate2::set_log_level(ctranslate2::LogLevel::Off);
    UnicodeReader reader(path);
    auto model = ctranslate2::models::Model::load(reader, ctranslate2::Device::CPU);
    ctranslate2::ReplicaPoolConfig config;
    config.num_threads_per_replica = 4;
    *handle = new ctranslate2::Translator(model, config);
    return 0;
  } catch (const std::exception&) { *error = copy_string("无法加载本地翻译模型，请检查模型文件。"); return -1; }
    catch (...) { *error = copy_string("原生翻译模型初始化失败。"); return -1; }
}
int jy_translate(void* handle, const char* tokens_json, char** output, char** error) noexcept {
  *output = nullptr; *error = nullptr;
  try {
    const auto request = nlohmann::json::parse(tokens_json);
    const auto input = (request.is_array() ? request : request.at("tokens")).get<std::vector<std::vector<std::string>>>();
    const auto prefix = request.is_object() && request.contains("target_prefix") && request["target_prefix"].is_string()
      ? request["target_prefix"].get<std::string>() : std::string();
    ctranslate2::TranslationOptions options;
    options.beam_size = 4;
    options.num_hypotheses = 1;
    options.length_penalty = prefix.empty() ? 0.2f : 1.0f;
    options.replace_unknowns = prefix.empty();
    options.max_decoding_length = prefix.empty() ? 256 : 512;
    // Multilingual inputs are bounded in Rust; never silently truncate them.
    if (!prefix.empty()) options.max_input_length = 0;
    auto* translator = static_cast<ctranslate2::Translator*>(handle);
    auto results = prefix.empty()
      ? translator->translate_batch(input, options, 32, ctranslate2::BatchType::Tokens)
      : translator->translate_batch(input, std::vector<std::vector<std::string>>(input.size(), {prefix}), options, 32, ctranslate2::BatchType::Tokens);
    std::vector<std::string> pieces;
    for (const auto& result : results) {
      if (result.hypotheses.empty()) throw std::runtime_error("empty hypothesis");
      auto begin = result.hypotheses[0].begin();
      if (!prefix.empty()) {
        if (begin == result.hypotheses[0].end() || *begin != prefix || result.hypotheses[0].size() >= options.max_decoding_length)
          throw std::runtime_error("invalid multilingual result");
        ++begin;
      }
      pieces.insert(pieces.end(), begin, result.hypotheses[0].end());
    }
    *output = copy_string(nlohmann::json(pieces).dump());
    return 0;
  } catch (const std::exception&) { *error = copy_string("原生翻译失败，请检查输入及模型兼容性。"); return -1; }
    catch (...) { *error = copy_string("原生翻译失败。"); return -1; }
}
void jy_model_delete(void* handle) noexcept { delete static_cast<ctranslate2::Translator*>(handle); }
void jy_free(char* text) noexcept { delete[] text; }
}
