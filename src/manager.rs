use crate::error::{Result, ZelligError};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub(crate) struct HFModel {
    pub(crate) id: String,
    pub(crate) downloads: Option<i64>,
    pub(crate) likes: Option<i64>,
    #[allow(dead_code)]
    pipeline_tag: Option<String>,
    #[allow(dead_code)]
    tags: Option<Vec<String>>,
}

async fn fetch_page(client: &reqwest::Client, search: &str, offset: usize) -> Result<Vec<HFModel>> {
    let offset_str = offset.to_string();
    let query_params = [
        ("search", search),
        ("pipeline_tag", "translation"),
        ("sort", "downloads"),
        ("direction", "-1"),
        ("limit", "50"),
        ("offset", offset_str.as_str()),
    ];

    let url = {
        let mut url = reqwest::Url::parse("https://huggingface.co/api/models")
            .map_err(|e| ZelligError::ModelError(format!("Invalid URL: {}", e)))?;
        {
            let mut q = url.query_pairs_mut();
            for (k, v) in &query_params {
                q.append_pair(k, v);
            }
        }
        url
    };

    let resp: reqwest::Response = client
        .get(url)
        .header("User-Agent", "zellig/0.1.0")
        .send()
        .await
        .map_err(|e| ZelligError::ModelError(format!("Failed to fetch models: {}", e)))?;
    let result: Vec<HFModel> = resp
        .json()
        .await
        .map_err(|e| ZelligError::ModelError(format!("Failed to parse response: {}", e)))?;
    Ok(result)
}

pub(crate) async fn fetch_all_models(search: &str) -> Result<Vec<HFModel>> {
    let effective_search = if search.is_empty() { "ct2" } else { search };
    let client = reqwest::Client::new();
    let mut all = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut offset = 0usize;
    loop {
        let models = fetch_page(&client, effective_search, offset).await?;
        let count = models.len();
        for m in models {
            if seen.insert(m.id.clone()) {
                all.push(m);
            }
        }
        if count < 50 {
            break;
        }
        offset += 50;
        if offset >= 500 {
            break;
        }
    }
    all.shrink_to_fit();
    Ok(all)
}

#[cfg(feature = "local")]
fn installed_set() -> std::collections::HashSet<String> {
    list_installed_models().into_iter().collect()
}

#[cfg(not(feature = "local"))]
fn installed_set() -> std::collections::HashSet<String> {
    std::collections::HashSet::new()
}

pub(crate) async fn search_and_print(search: Option<&str>, active_model: &str) {
    let query = search.unwrap_or("");
    crate::output::info("Fetching models from HuggingFace...");
    let models = match fetch_all_models(query).await {
        Ok(m) => m,
        Err(e) => {
            crate::output::err(format!("Failed: {}", e));
            return;
        }
    };
    if models.is_empty() {
        crate::output::info("No models found.");
        return;
    }
    let installed = installed_set();
    eprintln!();
    eprintln!(
        "     {}  {:>8}  {:>5}",
        crate::output::bold(format!("{:<49}", "Model")),
        crate::output::bold("Downloads"),
        crate::output::bold("Likes"),
    );
    eprintln!("  {}", crate::output::dim("─".repeat(70)));
    for m in &models {
        let marker = if m.id == active_model {
            crate::output::green("▶")
        } else if installed.contains(&m.id) {
            crate::output::green("●")
        } else {
            " ".to_string()
        };
        let id = if m.id.len() > 49 {
            format!("{}..", &m.id[..47])
        } else {
            format!("{:<49}", m.id)
        };
        let dl = crate::output::format_downloads(m.downloads.unwrap_or(0));
        let likes = m.likes.unwrap_or(0);
        eprintln!("  {}  {}  {:>8}  {:>5}", marker, id, dl, likes);
    }
    eprintln!();
    crate::output::info(format!(
        "{} models  {} = active  {} = installed",
        models.len(),
        crate::output::green("▶"),
        crate::output::green("●"),
    ));
    crate::output::info("Download: zellig models --download \"<model-id>\"");
    eprintln!();
}

pub(crate) fn print_installed(configured_model: &str) {
    #[cfg(feature = "local")]
    {
        let models = list_installed_models();
        if models.is_empty() {
            crate::output::info(format!(
                "No installed models (cache: {})",
                hf_cache_dir().display()
            ));
            return;
        }
        eprintln!();
        for m in &models {
            if m == configured_model {
                eprintln!(
                    "  {} {}  {}",
                    crate::output::green("▶"),
                    m,
                    crate::output::dim("← active")
                );
            } else {
                eprintln!("    {}", m);
            }
        }
        eprintln!();
        crate::output::info(format!("{} model(s) installed", models.len()));
        crate::output::info("Remove: zellig models --remove \"<model-id>\"");
        eprintln!();
    }
    #[cfg(not(feature = "local"))]
    {
        let _ = configured_model;
        crate::output::err("Local models require --features local");
    }
}

pub(crate) fn resolve_config_path(config_flag: Option<&str>) -> PathBuf {
    if let Some(path) = config_flag {
        return PathBuf::from(path);
    }
    let paths = [PathBuf::from("zellig.toml"), PathBuf::from("zelling.toml")];
    for p in &paths {
        if p.exists() {
            return p.clone();
        }
    }
    PathBuf::from("zellig.toml")
}

fn detect_compute_type(repo: &str) -> Option<&'static str> {
    let lower = repo.to_lowercase();
    if lower.contains("int8_float16") || lower.contains("int8-float16") {
        Some("INT8_FLOAT16")
    } else if lower.contains("int8_bfloat16") || lower.contains("int8-bfloat16") {
        Some("INT8_BFLOAT16")
    } else if lower.contains("float16") {
        Some("FLOAT16")
    } else if lower.contains("bfloat16") {
        Some("BFLOAT16")
    } else if lower.contains("float32") {
        Some("FLOAT32")
    } else if lower.contains("int16") {
        Some("INT16")
    } else if lower.contains("int8") {
        Some("INT8")
    } else {
        None
    }
}

pub(crate) fn set_model_in_config(config_path: &std::path::Path, repo: &str) -> Result<()> {
    use toml_edit::{value, DocumentMut, Item, Table};

    let content = if config_path.exists() {
        std::fs::read_to_string(config_path)
            .map_err(|e| ZelligError::ConfigError(format!("Failed to read config: {}", e)))?
    } else {
        String::new()
    };

    let mut doc: DocumentMut = content.parse().unwrap_or_default();

    doc["mode"] = value("local");

    let defaults = crate::config::LocalConfig::default();

    // Ensure [local] table exists; preserve all existing keys.
    if !doc.contains_table("local") {
        doc["local"] = Item::Table(Table::new());
    }
    let local = doc["local"].as_table_mut().unwrap();

    // Always update model_repo and compute_type — both are coupled to the model.
    local["model_repo"] = value(repo);
    local["compute_type"] =
        value(detect_compute_type(repo).unwrap_or(defaults.compute_type.as_str()));

    // Fill in missing fields with defaults — never overwrite user values.
    macro_rules! set_default {
        ($key:literal, $val:expr) => {
            if !local.contains_key($key) {
                local[$key] = value($val);
            }
        };
    }
    set_default!("model_format", defaults.model_format.as_str());
    set_default!("device", defaults.device.as_str());
    set_default!("beam_size", defaults.beam_size as i64);
    set_default!("max_decoding_length", defaults.max_decoding_length as i64);
    set_default!("num_threads", defaults.num_threads as i64);
    set_default!("repetition_penalty", defaults.repetition_penalty as f64);
    set_default!("no_repeat_ngram_size", defaults.no_repeat_ngram_size as i64);

    let out = doc.to_string();

    std::fs::write(config_path, out)
        .map_err(|e| ZelligError::ConfigError(format!("Failed to write config: {}", e)))?;

    Ok(())
}

#[cfg(feature = "local")]
pub(crate) async fn download_model(repo: &str) -> Result<()> {
    use hf_hub::{api::tokio::ApiBuilder, Repo, RepoType};
    let api = ApiBuilder::new()
        .with_progress(false)
        .build()
        .map_err(|e| ZelligError::DownloadError(e.to_string()))?;
    let hf_repo = api.repo(Repo::new(repo.to_string(), RepoType::Model));

    let info = hf_repo.info().await.map_err(|e| {
        ZelligError::DownloadError(format!("Repo not found or inaccessible: {}", e))
    })?;
    let files: std::collections::HashSet<String> =
        info.siblings.into_iter().map(|s| s.rfilename).collect();

    if !files.contains("model.bin") {
        return Err(ZelligError::DownloadError(format!(
            "'{}' is not a CTranslate2 model (no model.bin). Search for ct2 models in the browser.",
            repo
        )));
    }

    hf_repo
        .get("model.bin")
        .await
        .map_err(|e| ZelligError::DownloadError(format!("model.bin download failed: {}", e)))?;

    for file in &[
        "config.json",
        "tokenizer.json",
        "shared_vocabulary.txt",
        "vocabulary.txt",
        "sentencepiece.bpe.model",
        "vocabulary.json",
    ] {
        if files.contains(*file) {
            let _ = hf_repo.get(file).await;
        }
    }

    Ok(())
}

#[cfg(feature = "local")]
fn hf_cache_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HF_HOME") {
        PathBuf::from(home).join("hub")
    } else if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(cache).join("huggingface/hub")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache/huggingface/hub")
    } else {
        PathBuf::from(".cache/huggingface/hub")
    }
}

#[cfg(feature = "local")]
fn parse_model_dir(dir_name: &str) -> Option<String> {
    let name = dir_name.strip_prefix("models--")?;
    Some(name.replace("--", "/"))
}

#[cfg(feature = "local")]
pub(crate) fn list_installed_models() -> Vec<String> {
    let cache = hf_cache_dir();
    let mut models = Vec::new();
    let Ok(entries) = std::fs::read_dir(&cache) else {
        return models;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("models--") {
            continue;
        }
        let snapshots = path.join("snapshots");
        if !snapshots.is_dir() {
            continue;
        }
        let has_model = std::fs::read_dir(&snapshots)
            .ok()
            .is_some_and(|iter| iter.flatten().any(|s| s.path().join("model.bin").exists()));
        if has_model {
            if let Some(repo) = parse_model_dir(&name) {
                models.push(repo);
            }
        }
    }
    models.sort();
    models
}

#[cfg(feature = "local")]
pub(crate) fn uninstall_model_from_cache(repo: &str) -> Result<()> {
    let cache = hf_cache_dir();
    let dir_name = format!("models--{}", repo.replace("/", "--"));
    let model_dir = cache.join(&dir_name);
    if !model_dir.exists() {
        return Err(ZelligError::ConfigError(format!(
            "Model '{}' not found in cache at {}",
            repo,
            model_dir.display()
        )));
    }
    std::fs::remove_dir_all(&model_dir).map_err(|e| {
        ZelligError::ConfigError(format!("Failed to remove {}: {}", model_dir.display(), e))
    })?;
    Ok(())
}
