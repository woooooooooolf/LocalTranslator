"""Private JSON-lines worker. stdout is protocol only; never opens a server."""
import json
import os
from pathlib import Path
import sys
import time
import tempfile
import shutil
import threading

def watch_parent():
    """Also release models if the desktop process crashes or is terminated."""
    if os.name != 'nt' or not os.environ.get('JINGYI_PARENT_PID'):
        return
    import ctypes
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.argtypes = [ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
    kernel.OpenProcess.restype = ctypes.c_void_p
    kernel.WaitForSingleObject.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
    kernel.CloseHandle.argtypes = [ctypes.c_void_p]
    parent = kernel.OpenProcess(0x00100000, False, int(os.environ['JINGYI_PARENT_PID']))
    if parent:
        kernel.WaitForSingleObject(parent, 0xFFFFFFFF)
        kernel.CloseHandle(parent)
        standalone = globals().get('SESSION')
        if standalone is not None:
            standalone.cleanup()
        os._exit(0)

threading.Thread(target=watch_parent, daemon=True).start()

ROOT = Path(__file__).resolve().parent.parent
SESSION = tempfile.TemporaryDirectory(prefix='jingyi-worker-') if not os.environ.get('JINGYI_SESSION_DIR') else None
DATA = Path(os.environ['JINGYI_SESSION_DIR']) if os.environ.get('JINGYI_SESSION_DIR') else Path(SESSION.name)
for name, relative in {
    "USERPROFILE": "home", "APPDATA": "appdata", "LOCALAPPDATA": "appdata/local",
    "XDG_CONFIG_HOME": "config", "XDG_CACHE_HOME": "cache", "TEMP": "temp", "TMP": "temp",
}.items():
    path = DATA / relative
    path.mkdir(parents=True, exist_ok=True)
    os.environ[name] = str(path)
os.environ.update({
    "XDG_DATA_HOME": str(ROOT / "runtime/assets"),
    "ARGOS_PACKAGES_DIR": str(ROOT / "runtime/packages"),
    "ARGOS_DEVICE_TYPE": "cpu", "ARGOS_MODEL_PROVIDER": "OPENNMT",
    "ARGOS_CHUNK_TYPE": "MINISBD", "ARGOS_DEBUG": "0",
    "HF_HUB_OFFLINE": "1", "TRANSFORMERS_OFFLINE": "1",
    "PYTHONDONTWRITEBYTECODE": "1",
})
tempfile.tempdir = str(DATA / "temp")

def offline_guard(event, args):
    if event in {"socket.connect", "socket.getaddrinfo", "socket.sendto"}:
        raise RuntimeError("离线模式禁止网络连接；请检查本地模型文件是否齐全。")

sys.addaudithook(offline_guard)
protocol = sys.stdout
sys.stdout = sys.stderr
from argostranslate import package, translate  # noqa: E402

FORMATS = ["txt", "docx", "pptx", "odt", "odp", "epub", "html", "srt", "pdf"]

def translation(request):
    source, target = request["source"], request["target"]
    if source == target:
        raise ValueError("请选择不同的源语言和目标语言。")
    languages = {lang.code: lang for lang in translate.get_installed_languages()}
    if source not in languages or target not in languages:
        raise ValueError("对应的本地语言模型尚未安装。")
    result = languages[source].get_translation(languages[target])
    if result is None:
        raise ValueError("没有可用的翻译方向。")
    return result

def handle(request):
    operation = request.get("op")
    if operation == "info":
        languages = translate.get_installed_languages()
        return {
            "languages": [{"code": lang.code, "name": lang.name,
                           "targets": [t.to_lang.code for t in lang.translations_from if t.to_lang.code != lang.code]}
                          for lang in languages],
            "packages": [{"from": p.from_code, "to": p.to_code, "version": p.package_version}
                         for p in package.get_installed_packages()],
            "formats": FORMATS,
        }
    if operation == "text":
        text = request.get("text", "")
        if not text.strip() or len(text) > 20000:
            raise ValueError("请输入 1 至 20,000 字文字。")
        return {"text": translation(request).translate(text)}
    if operation == "file":
        source = Path(request["path"]).resolve(strict=True)
        destination = Path(request["output"]).resolve()
        if source == destination:
            raise ValueError("译文必须另存为，不能覆盖原文件。")
        if source.suffix.lower().lstrip(".") not in FORMATS:
            raise ValueError("不支持此文档格式。")
        if destination.suffix.lower() != source.suffix.lower():
            raise ValueError("请保留原文档扩展名。")
        if source.stat().st_size > 50 * 1024 * 1024:
            raise ValueError("单份文档最大支持 50 MB。")
        translator = translation(request)
        # Stage both source and output: third-party format handlers may write beside input.
        with tempfile.TemporaryDirectory(prefix="document-") as folder:
            staged = Path(folder) / source.name
            shutil.copyfile(source, staged)
            if source.suffix.lower() == ".txt":
                try:
                    text = staged.read_text(encoding="utf-8-sig")
                except UnicodeDecodeError:
                    text = staged.read_text(encoding="gb18030")
                result = Path(folder) / "translated.txt"
                result.write_text(translator.translate(text), encoding="utf-8")
            else:
                if source.suffix.lower() == '.pdf':
                    import pymupdf
                    with pymupdf.open(staged) as document:
                        if not any(page.get_text().strip() for page in document):
                            raise ValueError('PDF 没有可翻译的文字层，请先进行 OCR。')
                import argostranslatefiles
                result = argostranslatefiles.translate_file(translator, str(staged))
                if not result:
                    raise ValueError("此文档无法翻译。")
            # Atomic replacement only after successful translation, on output volume.
            fd, pending = tempfile.mkstemp(prefix=".jingyi-", dir=destination.parent)
            try:
                with os.fdopen(fd, "wb") as output, open(result, "rb") as translated:
                    shutil.copyfileobj(translated, output)
                os.replace(pending, destination)
            finally:
                if os.path.exists(pending):
                    os.unlink(pending)
        return {"path": str(destination)}
    raise ValueError("不支持的请求。")

for line in sys.stdin:
    started = time.monotonic()
    try:
        response = {"ok": True, "data": handle(json.loads(line))}
    except Exception as error:
        response = {"ok": False, "error": str(error)}
    response["elapsed"] = round(time.monotonic() - started, 2)
    protocol.write(json.dumps(response, ensure_ascii=False) + "\n")
    protocol.flush()
