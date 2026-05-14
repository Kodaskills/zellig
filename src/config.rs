use crate::error::{Result, ZelligError};
use figment2::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub mode: TranslationMode,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub local: LocalConfig,
    #[serde(default)]
    pub google: GoogleConfig,
    #[serde(default)]
    pub deepl: DeepLConfig,
    #[serde(default)]
    pub yandex: YandexConfig,
    #[serde(default)]
    pub libre_translate: LibreTranslateConfig,
    #[serde(default)]
    pub azure: AzureConfig,
    #[serde(default)]
    pub bergamot: BergamotConfig,
    #[serde(default)]
    pub baidu: BaiduConfig,
    #[serde(default)]
    pub youdao: YoudaoConfig,
    #[serde(default)]
    pub qq: QqConfig,
    #[serde(default)]
    pub lingva: LingvaConfig,
    #[serde(default)]
    pub translation: TranslationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq, Default, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "lower")]
pub enum TranslationMode {
    #[default]
    Local,
    Ai,
    Google,
    #[serde(rename = "deepl")]
    DeepL,
    Yandex,
    LibreTranslate,
    Azure,
    Bergamot,
    Baidu,
    Youdao,
    Qq,
    Lingva,
}

impl TranslationMode {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Local => "Local (CT2)",
            Self::Ai => "AI (LLM)",
            Self::Google => "Google Translate",
            Self::DeepL => "DeepL",
            Self::Yandex => "Yandex Translate",
            Self::LibreTranslate => "LibreTranslate",
            Self::Azure => "Azure Translator",
            Self::Bergamot => "Bergamot",
            Self::Baidu => "Baidu Fanyi",
            Self::Youdao => "Youdao Fanyi",
            Self::Qq => "QQ Fanyi",
            Self::Lingva => "Lingva Translate",
        }
    }

    pub fn needs_key(&self) -> bool {
        matches!(
            self,
            Self::DeepL | Self::Yandex | Self::Azure | Self::Baidu | Self::Youdao | Self::Qq
        )
    }

    pub fn char_limit(&self) -> Option<usize> {
        match self {
            Self::Local | Self::Ai | Self::Bergamot => None,
            Self::Google | Self::LibreTranslate | Self::Youdao | Self::Lingva => Some(5_000),
            Self::DeepL => Some(131_072),
            Self::Yandex | Self::Azure => Some(10_000),
            Self::Baidu | Self::Qq => Some(2_000),
        }
    }

    pub fn display_section_name(&self) -> &'static str {
        match self {
            Self::Ai => "AI Settings",
            Self::Local => "Local Settings",
            Self::DeepL => "DeepL Settings",
            Self::Yandex => "Yandex Settings",
            Self::Azure => "Azure Settings",
            Self::Baidu => "Baidu Settings",
            Self::Youdao => "Youdao Settings",
            Self::Qq => "QQ Settings",
            Self::LibreTranslate => "LibreTranslate Settings",
            Self::Bergamot => "Bergamot Settings",
            Self::Lingva => "Lingva Settings",
            Self::Google => "",
        }
    }

    pub fn display_settings<'a>(&self, config: &'a Config) -> Vec<(&'static str, &'a str)> {
        match self {
            Self::Ai => vec![
                ("Provider:", config.ai.provider.as_str()),
                ("Model:   ", config.ai.model.as_str()),
            ],
            Self::Local => vec![
                ("Model:  ", config.local.model_repo.as_str()),
                ("Compute:", config.local.compute_type.as_str()),
                ("Device: ", config.local.device.as_str()),
            ],
            Self::DeepL => vec![(
                "API Key:",
                if config.deepl.api_key.is_some() {
                    "configured"
                } else {
                    "not set"
                },
            )],
            Self::Yandex => vec![
                (
                    "API Key:   ",
                    if config.yandex.api_key.is_some() {
                        "configured"
                    } else {
                        "not set"
                    },
                ),
                (
                    "Folder ID: ",
                    config.yandex.folder_id.as_deref().unwrap_or("not set"),
                ),
            ],
            Self::Azure => vec![
                (
                    "API Key:",
                    if config.azure.api_key.is_some() {
                        "configured"
                    } else {
                        "not set"
                    },
                ),
                ("Region: ", config.azure.region.as_str()),
            ],
            Self::Baidu => vec![
                (
                    "App ID:    ",
                    config.baidu.app_id.as_deref().unwrap_or("not set"),
                ),
                (
                    "Secret Key:",
                    if config.baidu.secret_key.is_some() {
                        "configured"
                    } else {
                        "not set"
                    },
                ),
            ],
            Self::Youdao => vec![
                (
                    "App Key:   ",
                    config.youdao.app_key.as_deref().unwrap_or("not set"),
                ),
                (
                    "App Secret:",
                    if config.youdao.app_secret.is_some() {
                        "configured"
                    } else {
                        "not set"
                    },
                ),
            ],
            Self::Qq => vec![
                (
                    "Secret ID: ",
                    config.qq.secret_id.as_deref().unwrap_or("not set"),
                ),
                (
                    "Secret Key:",
                    if config.qq.secret_key.is_some() {
                        "configured"
                    } else {
                        "not set"
                    },
                ),
                ("Region:    ", config.qq.region.as_str()),
            ],
            Self::LibreTranslate => vec![
                ("URL:    ", config.libre_translate.url.as_str()),
                (
                    "API Key:",
                    if config.libre_translate.api_key.is_some() {
                        "configured"
                    } else {
                        "none"
                    },
                ),
            ],
            Self::Bergamot => vec![("URL:", config.bergamot.url.as_str())],
            Self::Lingva => vec![("Instance:", config.lingva.instance_url.as_str())],
            Self::Google => vec![],
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Local,
            Self::Ai,
            Self::Google,
            Self::DeepL,
            Self::Yandex,
            Self::LibreTranslate,
            Self::Azure,
            Self::Bergamot,
            Self::Baidu,
            Self::Youdao,
            Self::Qq,
            Self::Lingva,
        ]
    }
}

// ── AI ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_provider() -> String {
    "ollama".to_string()
}
fn default_model() -> String {
    "qwen2.5-coder:7b".to_string()
}

impl Default for AiConfig {
    fn default() -> Self {
        AiConfig {
            provider: default_provider(),
            api_key: None,
            base_url: None,
            model: default_model(),
        }
    }
}

// ── Local ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    #[serde(default = "default_model_repo")]
    pub model_repo: String,
    #[serde(default = "default_model_format")]
    pub model_format: String,
    #[serde(default)]
    pub cache_dir: Option<String>,
    #[serde(default = "default_compute_type")]
    pub compute_type: String,
    #[serde(default = "default_device")]
    pub device: String,
    #[serde(default = "default_beam_size")]
    pub beam_size: usize,
    #[serde(default = "default_max_decoding_length")]
    pub max_decoding_length: usize,
    #[serde(default = "default_num_threads")]
    pub num_threads: usize,
    #[serde(default = "default_repetition_penalty")]
    pub repetition_penalty: f32,
    #[serde(default = "default_no_repeat_ngram_size")]
    pub no_repeat_ngram_size: usize,
}

fn default_model_repo() -> String {
    "JustFrederik/nllb-200-distilled-600M-ct2-int8".to_string()
}
fn default_model_format() -> String {
    "ct2".to_string()
}
fn default_compute_type() -> String {
    "INT8".to_string()
}
fn default_device() -> String {
    "cpu".to_string()
}
fn default_beam_size() -> usize {
    4
}
fn default_max_decoding_length() -> usize {
    512
}
fn default_num_threads() -> usize {
    0
}
fn default_repetition_penalty() -> f32 {
    1.0
}
fn default_no_repeat_ngram_size() -> usize {
    0
}

impl Default for LocalConfig {
    fn default() -> Self {
        LocalConfig {
            model_repo: default_model_repo(),
            model_format: default_model_format(),
            cache_dir: None,
            compute_type: default_compute_type(),
            device: default_device(),
            beam_size: default_beam_size(),
            max_decoding_length: default_max_decoding_length(),
            num_threads: default_num_threads(),
            repetition_penalty: default_repetition_penalty(),
            no_repeat_ngram_size: default_no_repeat_ngram_size(),
        }
    }
}

// ── External services ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeepLConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub pro: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YandexConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibreTranslateConfig {
    #[serde(default = "default_libre_url")]
    pub url: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_libre_url() -> String {
    "https://libretranslate.com".to_string()
}

impl Default for LibreTranslateConfig {
    fn default() -> Self {
        Self {
            url: default_libre_url(),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_azure_region")]
    pub region: String,
    #[serde(default)]
    pub endpoint: Option<String>,
}

fn default_azure_region() -> String {
    "eastus".to_string()
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            region: default_azure_region(),
            endpoint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BergamotConfig {
    #[serde(default = "default_bergamot_url")]
    pub url: String,
}

fn default_bergamot_url() -> String {
    "http://localhost:8080".to_string()
}

impl Default for BergamotConfig {
    fn default() -> Self {
        Self {
            url: default_bergamot_url(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaiduConfig {
    #[serde(default)]
    pub app_id: Option<String>,
    #[serde(default)]
    pub secret_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YoudaoConfig {
    #[serde(default)]
    pub app_key: Option<String>,
    #[serde(default)]
    pub app_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QqConfig {
    #[serde(default)]
    pub secret_id: Option<String>,
    #[serde(default)]
    pub secret_key: Option<String>,
    #[serde(default = "default_qq_region")]
    pub region: String,
}

fn default_qq_region() -> String {
    "ap-beijing".to_string()
}

impl Default for QqConfig {
    fn default() -> Self {
        Self {
            secret_id: None,
            secret_key: None,
            region: default_qq_region(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LingvaConfig {
    #[serde(default = "default_lingva_url")]
    pub instance_url: String,
}

fn default_lingva_url() -> String {
    "https://lingva.ml".to_string()
}

impl Default for LingvaConfig {
    fn default() -> Self {
        Self {
            instance_url: default_lingva_url(),
        }
    }
}

// ── Translation defaults ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    #[serde(default = "default_source_lang", alias = "source_lang")]
    pub default_source_lang: String,
    #[serde(default, alias = "target_langs")]
    pub default_target_langs: Vec<String>,
    #[serde(default)]
    pub context: Option<String>,
}

fn default_source_lang() -> String {
    "en".to_string()
}

impl Default for TranslationConfig {
    fn default() -> Self {
        TranslationConfig {
            default_source_lang: default_source_lang(),
            default_target_langs: vec!["fr".to_string(), "de".to_string()],
            context: None,
        }
    }
}

// ── Loader ───────────────────────────────────────────────

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(config_path: Option<&str>) -> Result<Config> {
        let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

        if let Some(path) = config_path {
            figment = figment.merge(Toml::file(path));
        }

        figment = figment.merge(Env::prefixed("ZELLIG_").split("_"));

        let config: Config = figment
            .extract()
            .map_err(|e| ZelligError::ConfigError(e.to_string()))?;

        Ok(config)
    }
}

pub fn generate_example_config() -> String {
    "# Example Zellig configuration\n\
    # Copy to zellig.toml and customize\n\
    \n\
    # Translation service: local, ai, google, deepl, yandex,\n\
    #   libre_translate, azure, bergamot, baidu, youdao, qq, lingva\n\
    mode = \"local\"\n\
    \n\
    [ai]\n\
    provider = \"openai\"\n\
    model = \"gpt-4o-mini\"\n\
    # api_key = \"sk-...\"\n\
    \n\
    [deepl]\n\
    # api_key = \"...\"\n\
    # pro = false\n\
    \n\
    [yandex]\n\
    # api_key = \"...\"\n\
    # folder_id = \"...\"\n\
    \n\
    [libre_translate]\n\
    # url = \"https://libretranslate.com\"\n\
    # api_key = \"...\"\n\
    \n\
    [azure]\n\
    # api_key = \"...\"\n\
    # region = \"eastus\"\n\
    \n\
    [bergamot]\n\
    # url = \"http://localhost:8080\"\n\
    \n\
    [baidu]\n\
    # app_id = \"...\"\n\
    # secret_key = \"...\"\n\
    \n\
    [youdao]\n\
    # app_key = \"...\"\n\
    # app_secret = \"...\"\n\
    \n\
    [qq]\n\
    # secret_id = \"...\"\n\
    # secret_key = \"...\"\n\
    # region = \"ap-beijing\"\n\
    \n\
    [lingva]\n\
    # instance_url = \"https://lingva.ml\"\n\
    \n\
    [translation]\n\
    default_source_lang = \"en\"\n\
    default_target_langs = [\"fr\", \"de\"]\n"
        .to_string()
}
