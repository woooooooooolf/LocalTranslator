#include <filesystem>
#include <iostream>
#include <string>
extern "C" {
int jy_model_new(const char*, void**, char**) noexcept;
int jy_translate(void*, const char*, char**, char**) noexcept;
void jy_model_delete(void*) noexcept;
void jy_free(char*) noexcept;
}
int wmain(int argc, wchar_t** argv) {
  if (argc != 2) return 2;
  auto path = std::filesystem::path(argv[1]).u8string();
  void* model = nullptr; char* error = nullptr;
  if (jy_model_new(path.c_str(), &model, &error) != 0) { jy_free(error); return 3; }
  std::string line;
  while (std::getline(std::cin, line)) {
    char* result = nullptr;
    if (jy_translate(model, line.c_str(), &result, &error) != 0) { jy_free(error); jy_model_delete(model); return 4; }
    std::cout << result << std::endl;
    jy_free(result);
  }
  jy_model_delete(model);
  return 0;
}
