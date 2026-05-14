// Zellig - Auto Translation CLI Tool
// Follows SOLID principles.

pub mod cli;
pub mod config;
pub mod error;
pub mod formats;
pub mod language_detector;
pub mod languages;
pub mod manager;
pub mod output;
pub mod stats;
pub mod translation_service;
pub mod translator;
pub mod tui;

pub use config::Config;
pub use config::ConfigLoader;
pub use error::Result;
pub use language_detector::LanguageDetector;
pub use translation_service::TranslationService;
pub use translator::AiTranslator;
pub use translator::AzureTranslator;
pub use translator::BaiduTranslator;
pub use translator::BergamotTranslator;
pub use translator::DeepLTranslator;
pub use translator::GoogleTranslator;
pub use translator::LibreTranslator;
pub use translator::LingvaTranslator;
pub use translator::LocalTranslator;
pub use translator::QqTranslator;
pub use translator::Translator;
pub use translator::TranslatorFactory;
pub use translator::YandexTranslator;
pub use translator::YoudaoTranslator;
