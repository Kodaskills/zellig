<div align="center">

<img src="web/logo.svg" alt="zellig logo" width="128" height="128" />

# zellig

### CLI translation tool — text, files, and directories — local AI or cloud APIs.

[![Crates.io](https://img.shields.io/crates/v/zellig?style=for-the-badge&logo=rust&color=orange)](https://crates.io/crates/zellig)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)](LICENSE)
[![Last Commit](https://img.shields.io/github/last-commit/kodaskills/zellig/main?style=for-the-badge)](https://github.com/kodaskills/zellig/commits/main)

### Built with:
[![Rust](https://img.shields.io/badge/Rust-2021-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)

</div>

---

## ✨ Features

- **Multiple backends** — local NLLB-200 (offline), LLMs via genai, and 10 cloud APIs
- **File format support** — `.po`, `.xliff`, `.json`, `.toml`, `.yaml`, `.md` with structure preserved
- **Directory translation** — translate all supported files in a folder in one command
- **Interactive TUI** — built with ratatui for a full-screen terminal interface
- **Language detection** — local (whatlang) or AI-powered
- **Async** — tokio-based, concurrent multi-target translation
- **Content-hash caching** — skip unchanged segments automatically
- **Single binary** — no runtime dependencies

---

## 🚀 Installation

**One-liner** (recommended — macOS & Linux, no Rust needed):

```bash
curl -fsSL https://raw.githubusercontent.com/kodaskills/zellig/main/install.sh | sh
```

**Via cargo** (requires Rust 1.74+):

```bash
cargo install zellig
```

**With local model support** (NLLB-200 offline translation):

```bash
cargo install zellig --features local
```

**With GPU (CUDA) support** (single binary — auto-detects GPU at runtime, falls back to CPU):

```bash
cargo install zellig --features cuda
```

**Pre-built binary** — download for your platform from [GitHub Releases](https://github.com/kodaskills/zellig/releases/latest) (Linux, macOS, Windows).

> **Linux runtime requirement**: the `local` backend (NLLB-200) requires `libgomp.so.1` (GCC OpenMP runtime). It is already present on most systems. If you get `error while loading shared libraries: libgomp.so.1`, install it:
> ```bash
> # Debian/Ubuntu
> sudo apt-get install libgomp1
> # RHEL/Fedora
> sudo dnf install libgomp
> # Arch
> sudo pacman -S gcc-libs
> ```

**Build from source:**

```bash
git clone https://github.com/kodaskills/zellig
cd zellig
cargo build --release
./target/release/zellig --help
```

**Docker** — pull from GHCR, no Rust needed:

```bash
# Translate a string
docker run --rm ghcr.io/kodaskills/zellig translate "Hello, world!" --target fr

# Mount a local directory and translate its files
docker run --rm -v "$(pwd):/data" ghcr.io/kodaskills/zellig \
  translate --dir /data/locales --target es --target de

# Pass a config file
docker run --rm \
  -v "$(pwd)/zellig.toml:/config/zellig.toml:ro" \
  -v "$(pwd):/data" \
  ghcr.io/kodaskills/zellig \
  --config /config/zellig.toml translate --dir /data/locales --target fr
```

**Docker Compose** (from the repo — includes a dev mode with hot-reload):

```bash
# Clone the repo first
git clone https://github.com/kodaskills/zellig && cd zellig

# One-off production run
docker compose --profile prod run --rm prod translate "Hello, world!" --target fr

# Development shell — cargo-watch recompiles on every file save
docker compose --profile dev up
```

Available image tags: `ghcr.io/kodaskills/zellig:latest` · `ghcr.io/kodaskills/zellig:0.1.0`

> The prod image includes the `local` NLLB-200 backend. You still need to download a model
> the first time (`zellig models --download <repo>`); mount a volume to persist the cache.

---

## ⚡ Quick Start

```bash
# Translate a string to French
zellig translate "Hello, world!" --target fr

# Translate to multiple targets at once
zellig translate "Hello, world!" --target fr --target ja --target de

# Translate a file (output: README.fr.md)
zellig translate --input README.md --target fr

# Translate a whole directory
zellig translate --dir ./locales --target es --target de

# Detect the language of a string
zellig detect "Bonjour le monde"

# Launch the interactive TUI
zellig tui

# List all supported languages
zellig languages
```

---

## 🔌 Backends

Select a backend with `--mode` or set it in `zellig.toml`.

| Mode | Description | Key required |
|------|-------------|--------------|
| `local` | NLLB-200 via CTranslate2 — fully offline, 200+ languages | No (needs `--features local`) |
| `ai` | LLM via genai (OpenAI, Anthropic, Gemini, Ollama, …) | Depends on provider (Ollama: no) |
| `google` | Google Translate — unofficial free endpoint | No |
| `deepl` | DeepL API | Yes |
| `yandex` | Yandex Translate | Yes |
| `azure` | Azure Translator | Yes |
| `baidu` | Baidu Fanyi | Yes |
| `youdao` | Youdao Fanyi | Yes |
| `qq` | QQ Fanyi | Yes |
| `libretranslate` | LibreTranslate (self-hostable) | Optional |
| `bergamot` | Bergamot (Firefox Translations) | No |
| `lingva` | Lingva Translate (self-hostable) | No |

---

## 📁 Supported File Formats

| Format | Extension | Notes |
|--------|-----------|-------|
| Gettext PO | `.po` | Translates `msgstr` values, keeps structure |
| XLIFF | `.xliff` | Translates `<target>` elements |
| JSON | `.json` | Translates string values, keys preserved |
| TOML | `.toml` | Translates string values |
| YAML | `.yaml` | Translates string values |
| Markdown | `.md` | Translates prose, preserves code blocks |

Output files are written alongside the source with the target language inserted before the extension — e.g. `README.md` → `README.fr.md`.

---

## ⚙️ Configuration

Generate a starter config:

```bash
zellig config --generate
```

Place the output in `zellig.toml` (or pass `--config <path>`). Environment variables override config file values.

### Config reference

```toml
# Default translation backend
mode = "local"   # local | ai | google | deepl | yandex | libretranslate | azure | bergamot | baidu | youdao | qq | lingva

[translation]
default_source_lang = "en"
default_target_langs = ["fr", "de"]
context = "Informal blog post"   # Optional hint passed to AI/LLM backends

[local]
model_repo = "JustFrederik/nllb-200-distilled-1.3B-ct2-int8"
device = "cpu"               # "cuda" requires --features cuda build
compute_type = "INT8"        # INT8 | FLOAT16 | FLOAT32 (FLOAT16 recommended for CUDA)
repetition_penalty = 1.0     # >1.0 penalises repeated phrases
no_repeat_ngram_size = 0     # block n-gram repetitions (0 = disabled, try 3)

[ai]
model = "gpt-4o-mini"   # Any model supported by genai

[deepl]
api_key = "YOUR_DEEPL_KEY"

[google]
api_key = "YOUR_GOOGLE_KEY"

# … one section per cloud backend
```

---

## 📦 Local Model Management

The `local` backend downloads a CTranslate2-format model from HuggingFace Hub.

```bash
# Search available models
zellig models --search nllb

# Download a model
zellig models --download "JustFrederik/nllb-200-distilled-1.3B-ct2-int8"

# List installed models
zellig models --installed

# Remove a model from cache
zellig models --remove "JustFrederik/nllb-200-distilled-1.3B-ct2-int8"
```

> Local backend requires `--features local` at compile time.

---

## 🖥️ Commands

```
zellig translate [TEXT] [OPTIONS]
  TEXT                     Text to translate (omit to use --input or --dir)
  -s, --source <LANG>      Source language [default: en]
  -t, --target <LANG>      Target language(s) — repeat for multiple
  -i, --input <FILE>       Translate a file
  -d, --dir <DIR>          Translate all supported files in a directory
      --mode <MODE>        Override configured backend
      --context <TEXT>     Context hint (AI/LLM backends)

zellig batch-translate [OPTIONS]
  -i, --input <FILE>       Input file (one string per line)
  -o, --output <FILE>      Output file (optional — prints to stdout otherwise)
  -s, --source <LANG>      Source language [default: en]
  -t, --target <LANG>      Target language(s)
      --mode <MODE>        Override configured backend
      --context <TEXT>     Context hint

zellig detect <TEXT> [--mode <MODE>]
  Detect the language of a string.

zellig languages
  List all supported BCP-47 language codes.

zellig config [--generate]
  Show current config or print an example config to stdout.

zellig tui
  Launch the interactive TUI.

zellig models [--search <QUERY>] [--download <REPO>] [--installed] [--remove <REPO>]
  Manage local CTranslate2 models.

Global:
  --config <PATH>          Path to config file
```

---

## ❓ FAQ

### Which backend should I use?

For offline, privacy-first translation: **`local`** (NLLB-200, 200+ languages, no API key). For highest quality on document-length text: **`ai`** with a capable LLM. For high-volume production use: **`deepl`** or **`google`**.

### How do I translate to multiple languages at once?

Repeat `--target`:

```bash
zellig translate --input app.json --target fr --target de --target ja
```

### Does zellig preserve file structure?

Yes. For every supported format, zellig extracts only the translatable strings, passes them through the backend, and reconstructs the file with the same structure, comments, and metadata intact.

### How do I add context for AI translations?

Use `--context`:

```bash
zellig translate "Submit" --target fr --mode ai --context "Button label in a web form"
```

Or set `context` in `zellig.toml` for a project-wide default.

### How does caching work?

Zellig hashes each source string (MD5/SHA-256). On subsequent runs, strings whose hash matches a cached entry are skipped. The cache is per-backend and per-target-language.

### Can I use zellig with self-hosted services?

Yes. **LibreTranslate** and **Lingva** support custom `api_url` in config — point them at your own instance.

---

## 📄 License

MIT — see [LICENSE](LICENSE) for details.

---

<div align="center">

**Maintained with ⚡ by the [Kodaskills](https://github.com/kodaskills) team**

[![Rust](https://img.shields.io/badge/Made%20with-Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)

</div>
