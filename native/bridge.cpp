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
    const auto input = nlohmann::json::parse(tokens_json).get<std::vector<std::vector<std::string>>>();
    ctranslate2::TranslationOptions options;
    options.beam_size = 4;
    options.num_hypotheses = 1;
    options.length_penalty = 0.2f;
    options.replace_unknowns = true;
    auto results = static_cast<ctranslate2::Translator*>(handle)->translate_batch(
      input, options, 32, ctranslate2::BatchType::Tokens);
    std::vector<std::string> pieces;
    for (const auto& result : results) {
      if (result.hypotheses.empty()) throw std::runtime_error("empty hypothesis");
      pieces.insert(pieces.end(), result.hypotheses[0].begin(), result.hypotheses[0].end());
    }
    *output = copy_string(nlohmann::json(pieces).dump());
    return 0;
  } catch (const std::exception&) { *error = copy_string("原生翻译失败，请检查输入及模型兼容性。"); return -1; }
    catch (...) { *error = copy_string("原生翻译失败。"); return -1; }
}
void jy_model_delete(void* handle) noexcept { delete static_cast<ctranslate2::Translator*>(handle); }
void jy_free(char* text) noexcept { delete[] text; }
}
