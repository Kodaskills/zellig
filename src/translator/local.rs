use crate::config::LocalConfig;
use crate::error::{Result, ZelligError};
use crate::languages::iso_to_nllb;
use crate::translator::{LocalBackend, Translator};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::task::spawn_blocking;

pub struct LocalTranslator {
    backend: Arc<dyn LocalBackend>,
}

impl LocalTranslator {
    pub async fn new(config: &LocalConfig) -> Result<Self> {
        match config.model_format.as_str() {
            "ct2" => {
                #[cfg(feature = "ct2rs")]
                {
                    let backend = Arc::new(ct2_backend::Ct2Model::new(config).await?);
                    Ok(Self { backend })
                }
                #[cfg(not(feature = "ct2rs"))]
                {
                    Err(ZelligError::ConfigError(
                        "CT2 format not enabled. Compile with --features ct2rs".to_string(),
                    ))
                }
            }
            _ => Err(ZelligError::ConfigError(format!(
                "Unsupported format: {}",
                config.model_format
            ))),
        }
    }
}

impl Translator for LocalTranslator {
    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        let (text, source_nllb, target_nllb) = (
            text.to_string(),
            iso_to_nllb(source_lang).unwrap_or(source_lang).to_string(),
            iso_to_nllb(target_lang).unwrap_or(target_lang).to_string(),
        );
        let backend = Arc::clone(&self.backend);
        Box::pin(async move {
            spawn_blocking(move || backend.translate(&text, &source_nllb, &target_nllb))
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?
        })
    }

    fn batch_translate<'a>(
        &'a self,
        texts: &'a [String],
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + 'a>> {
        let texts = texts.to_vec();
        let (source_nllb, target_nllb) = (
            iso_to_nllb(source_lang).unwrap_or(source_lang).to_string(),
            iso_to_nllb(target_lang).unwrap_or(target_lang).to_string(),
        );
        let backend = Arc::clone(&self.backend);
        Box::pin(async move {
            spawn_blocking(move || backend.batch_translate(&texts, &source_nllb, &target_nllb))
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?
        })
    }
}

#[cfg(feature = "ct2rs")]
pub(crate) mod ct2_backend {
    use super::*;
    use ct2rs::tokenizers::auto::Tokenizer;
    use ct2rs::{BatchType, ComputeType, Config as Ct2Config, Translator as CT2Translator};
    use std::sync::Arc;

    #[derive(Clone)]
    pub struct Ct2Model {
        pub translator: Arc<CT2Translator<Tokenizer>>,
        beam_size: usize,
        max_decoding_length: usize,
        repetition_penalty: f32,
        no_repeat_ngram_size: usize,
    }

    impl Ct2Model {
        pub async fn new(config: &LocalConfig) -> Result<Self> {
            let model_dir = download_model(config).await?;
            let device = {
                #[cfg(feature = "cuda")]
                {
                    match config.device.to_lowercase().as_str() {
                        "cuda" => ct2rs::Device::CUDA,
                        _ => ct2rs::Device::CPU,
                    }
                }
                #[cfg(not(feature = "cuda"))]
                ct2rs::Device::CPU
            };
            let ct2_config = Ct2Config {
                device,
                compute_type: match config.compute_type.as_str() {
                    "INT8" => ComputeType::INT8,
                    "FLOAT32" => ComputeType::FLOAT32,
                    "FLOAT16" => ComputeType::FLOAT16,
                    _ => ComputeType::INT8,
                },
                num_threads_per_replica: config.num_threads,
                ..Default::default()
            };
            Ok(Self {
                translator: Arc::new(
                    CT2Translator::new(model_dir.to_str().unwrap(), &ct2_config)
                        .map_err(|e| ZelligError::ModelError(e.to_string()))?,
                ),
                beam_size: config.beam_size,
                max_decoding_length: config.max_decoding_length,
                repetition_penalty: config.repetition_penalty,
                no_repeat_ngram_size: config.no_repeat_ngram_size,
            })
        }

        pub fn translate_with_beam(
            &self,
            text: &str,
            source_lang: &str,
            target_lang: &str,
            beam_size: usize,
        ) -> Result<String> {
            let input = format!("{} {}", source_lang, text);
            let target_prefixes = vec![vec![target_lang.to_string()]];
            let max_len =
                compute_max_decoding_length(std::slice::from_ref(&input), self.max_decoding_length);
            let results = self
                .translator
                .translate_batch_with_target_prefix(
                    &[input],
                    &target_prefixes,
                    &ct2rs::TranslationOptions {
                        beam_size,
                        max_decoding_length: max_len,
                        repetition_penalty: self.repetition_penalty,
                        no_repeat_ngram_size: self.no_repeat_ngram_size,
                        ..Default::default()
                    },
                    None,
                )
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;
            Ok(clean_unk(&results[0].0))
        }

        pub fn batch_translate_with_beam(
            &self,
            texts: &[String],
            source_lang: &str,
            target_lang: &str,
            beam_size: usize,
        ) -> Result<Vec<String>> {
            let inputs: Vec<String> = texts
                .iter()
                .map(|t| format!("{} {}", source_lang, t))
                .collect();
            let target_prefixes: Vec<Vec<String>> = (0..texts.len())
                .map(|_| vec![target_lang.to_string()])
                .collect();
            let max_len = compute_max_decoding_length(texts, self.max_decoding_length);
            let results = self
                .translator
                .translate_batch_with_target_prefix(
                    &inputs,
                    &target_prefixes,
                    &ct2rs::TranslationOptions {
                        beam_size,
                        max_decoding_length: max_len,
                        max_batch_size: 64,
                        batch_type: BatchType::Examples,
                        repetition_penalty: self.repetition_penalty,
                        no_repeat_ngram_size: self.no_repeat_ngram_size,
                        ..Default::default()
                    },
                    None,
                )
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;
            Ok(results.into_iter().map(|(t, _)| clean_unk(&t)).collect())
        }
    }

    impl LocalBackend for Ct2Model {
        fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String> {
            self.translate_with_beam(text, source_lang, target_lang, self.beam_size)
        }

        fn batch_translate(
            &self,
            texts: &[String],
            source_lang: &str,
            target_lang: &str,
        ) -> Result<Vec<String>> {
            self.batch_translate_with_beam(texts, source_lang, target_lang, self.beam_size)
        }
    }

    fn clean_unk(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut first = true;
        for word in text.split("<unk>").flat_map(|s| s.split_whitespace()) {
            if !first {
                result.push(' ');
            }
            result.push_str(word);
            first = false;
        }
        result
    }

    fn compute_max_decoding_length(texts: &[String], cap: usize) -> usize {
        let max_chars = texts.iter().map(|t| t.len()).max().unwrap_or(0);
        (max_chars / 4 * 2 + 10).min(cap).max(20)
    }

    async fn download_model(config: &LocalConfig) -> Result<std::path::PathBuf> {
        use hf_hub::{api::tokio::ApiBuilder, Repo, RepoType};
        let api = ApiBuilder::new()
            .with_progress(false)
            .build()
            .map_err(|e| ZelligError::DownloadError(e.to_string()))?;
        let repo = api.repo(Repo::new(config.model_repo.clone(), RepoType::Model));
        let model_bin = repo
            .get("model.bin")
            .await
            .map_err(|e| ZelligError::DownloadError(format!("model.bin download failed: {}", e)))?;
        for file in &[
            "config.json",
            "tokenizer.json",
            "shared_vocabulary.txt",
            "sentencepiece.bpe.model",
            "vocabulary.json",
            "vocabulary.txt",
        ] {
            let _ = repo.get(file).await;
        }
        Ok(model_bin.parent().unwrap().to_path_buf())
    }
}

#[cfg(all(test, feature = "ct2rs"))]
mod bench {
    use super::ct2_backend::Ct2Model;
    use crate::config::LocalConfig;
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn compare_beam_sizes() {
        let config = LocalConfig {
            beam_size: 1,
            ..Default::default()
        };
        let model = Arc::new(Ct2Model::new(&config).await.unwrap());

        let texts: Vec<String> = (0..4)
            .map(|i| {
                format!(
                    "The quick brown fox jumps over the lazy dog. This is sentence number {}. ",
                    i
                )
                .repeat(20)
            })
            .collect();

        let _ = model
            .batch_translate_with_beam(&texts, "eng_Latn", "fra_Latn", 1)
            .unwrap();
        let _ = model
            .batch_translate_with_beam(&texts, "eng_Latn", "fra_Latn", 4)
            .unwrap();

        let start = Instant::now();
        for _ in 0..5 {
            let _ = model
                .batch_translate_with_beam(&texts, "eng_Latn", "fra_Latn", 1)
                .unwrap();
        }
        let dur_1 = start.elapsed() / 5;

        let start = Instant::now();
        for _ in 0..5 {
            let _ = model
                .batch_translate_with_beam(&texts, "eng_Latn", "fra_Latn", 4)
                .unwrap();
        }
        let dur_4 = start.elapsed() / 5;

        println!("\n=== BEAM SIZE BENCHMARK ===");
        println!("  texts: {}", texts.len());
        println!("  chars per text: ~{}", texts[0].len());
        println!("  beam_size=1: {:.3}s avg", dur_1.as_secs_f64());
        println!("  beam_size=4: {:.3}s avg", dur_4.as_secs_f64());
        println!(
            "  ratio (1/4): {:.2}x",
            dur_1.as_secs_f64() / dur_4.as_secs_f64()
        );
        println!("============================\n");
    }
}
