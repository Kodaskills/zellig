use crate::config::{Config, ConfigLoader, TranslationMode};
use crate::error::{Result, ZelligError};
use crate::language_detector::LanguageDetector;
use crate::languages;
use crate::manager;
use crate::output;
use crate::translation_service::TranslationService;
use crate::translator::Translator;
use clap::{Parser, Subcommand};
use futures::future::{join_all, try_join_all};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "zellig")]
#[command(about = "Auto Translation CLI tool locally with NLLB or with AI support", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub config: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Translate {
        text: Option<String>,
        #[arg(short, long, default_value = "en")]
        source: String,
        #[arg(short, long)]
        target: Vec<String>,
        #[arg(short = 'i', long)]
        input: Option<String>,
        #[arg(short = 'd', long)]
        dir: Option<String>,
        #[arg(long)]
        context: Option<String>,
        #[arg(
            long,
            value_enum,
            help = "Override config mode (e.g. local, ai, google, deepl)"
        )]
        mode: Option<TranslationMode>,
    },
    BatchTranslate {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, default_value = "en")]
        source: String,
        #[arg(short, long)]
        target: Vec<String>,
        #[arg(long, value_enum, help = "Override config mode")]
        mode: Option<TranslationMode>,
        #[arg(long)]
        context: Option<String>,
    },
    Detect {
        text: String,
        #[arg(long, value_enum, help = "Override config mode")]
        mode: Option<TranslationMode>,
    },
    Config {
        #[arg(long)]
        generate: bool,
    },
    Languages,
    #[command(about = "Launch interactive TUI mode")]
    Tui,
    #[command(about = "Search, download, or manage CTranslate2 models")]
    Models {
        #[arg(short, long, help = "Search query (default: list all)")]
        search: Option<String>,
        #[arg(long, help = "Download a model by HuggingFace repo ID")]
        download: Option<String>,
        #[arg(long, help = "List installed models")]
        installed: bool,
        #[arg(long, help = "Remove an installed model from cache")]
        remove: Option<String>,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.as_deref();
    let config = ConfigLoader::load(config_path)?;

    match cli.command {
        Commands::Translate {
            text,
            source,
            target,
            input,
            dir,
            context,
            mode,
        } => handle_translate(config, text, source, target, input, dir, context, mode).await,
        Commands::BatchTranslate {
            input,
            output,
            source,
            target,
            mode,
            context,
        } => handle_batch_translate(config, input, output, source, target, mode, context).await,
        Commands::Detect { text, mode } => handle_detect(config, text, mode).await,
        Commands::Config { generate } => handle_config(config, generate),
        Commands::Languages => {
            languages::list_languages();
            Ok(())
        }
        Commands::Tui => crate::tui::run(config).await,
        Commands::Models {
            search,
            download,
            installed,
            remove,
        } => handle_models(config, config_path, search, download, installed, remove).await,
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_translate(
    mut config: Config,
    text: Option<String>,
    source: String,
    target: Vec<String>,
    input: Option<String>,
    dir: Option<String>,
    context: Option<String>,
    mode: Option<TranslationMode>,
) -> Result<()> {
    if let Some(m) = mode {
        config.mode = m;
    }
    if let Some(ctx) = context {
        config.translation.context = Some(ctx);
    }
    let targets = if target.is_empty() {
        config.translation.default_target_langs.clone()
    } else {
        target
    };
    let service = TranslationService::new(config).await?;
    let ctx = service.config().translation.context.as_deref();

    if let Some(dir_path) = dir {
        translate_dir(&*service.translator, &dir_path, &source, &targets, ctx).await?;
    } else if let Some(input_file) = input {
        translate_file(&*service.translator, &input_file, &source, &targets, ctx).await?;
    } else if let Some(t) = text {
        let results = join_all(targets.iter().map(|lang| async {
            let start = std::time::Instant::now();
            let lang = lang.clone();
            let result = service.translate_text(&t, &source, &lang, ctx).await;
            (lang, result, start.elapsed())
        }))
        .await;

        for (lang, result, elapsed) in results {
            match result {
                Ok(text) => {
                    println!("{}", text);
                    output::info(format!(
                        "{}  {}",
                        lang,
                        output::elapsed(elapsed.as_secs_f64())
                    ));
                }
                Err(e) => output::err(format!("{}: {}", lang, e)),
            }
        }
    } else {
        output::err("Provide text, --input <file>, or --dir <directory>");
    }
    Ok(())
}

async fn handle_batch_translate(
    mut config: Config,
    input: String,
    output: Option<String>,
    source: String,
    target: Vec<String>,
    mode: Option<TranslationMode>,
    context: Option<String>,
) -> Result<()> {
    if let Some(m) = mode {
        config.mode = m;
    }
    if let Some(ctx) = context {
        config.translation.context = Some(ctx);
    }
    let targets = if target.is_empty() {
        config.translation.default_target_langs.clone()
    } else {
        target
    };
    let service = TranslationService::new(config).await?;
    let ctx = service.config().translation.context.as_deref();

    let texts = std::fs::read_to_string(&input)
        .map_err(ZelligError::IoError)?
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();

    if texts.is_empty() {
        return Err(ZelligError::ConfigError(
            "No texts found in input file".to_string(),
        ));
    }

    output::ok(format!("Loaded {} lines from {}", texts.len(), input));
    eprintln!();

    let source_lang = source.as_str();
    let svc = &service;

    for target_lang in &targets {
        let start = std::time::Instant::now();

        let pb = Arc::new(ProgressBar::new(texts.len() as u64));
        pb.set_style(
            ProgressStyle::with_template(&format!(
                "  {} {} {}  {{bar:32.cyan/blue}} {{pos}}/{{len}}  {{eta}}",
                output::dim("["),
                output::bold(target_lang),
                output::dim("]"),
            ))
            .expect("hardcoded indicatif template")
            .progress_chars("=> "),
        );

        let results = try_join_all(texts.iter().map(|t| {
            let pb = Arc::clone(&pb);
            async move {
                let result = svc.translate_text(t, source_lang, target_lang, ctx).await;
                pb.inc(1);
                result
            }
        }))
        .await?;
        pb.finish_and_clear();

        let elapsed = start.elapsed();

        if let Some(ref out_path) = output {
            let mut output_text = String::new();
            for result in &results {
                output_text.push_str(&format!("[{}] {}\n", target_lang, result));
            }
            std::fs::write(out_path, output_text).map_err(ZelligError::IoError)?;
            output::ok(format!(
                "{} written  {}",
                output::bold(target_lang),
                output::elapsed(elapsed.as_secs_f64())
            ));
            output::info(format!("Saved to {}", out_path));
        } else {
            for result in &results {
                println!("[{}] {}", target_lang, result);
            }
            output::ok(format!(
                "{}  {} strings  {}",
                output::bold(target_lang),
                results.len(),
                output::elapsed(elapsed.as_secs_f64())
            ));
        }

        eprintln!();
    }
    Ok(())
}

async fn handle_detect(
    mut config: Config,
    text: String,
    mode: Option<TranslationMode>,
) -> Result<()> {
    if let Some(m) = mode {
        config.mode = m;
    }
    let use_local = matches!(config.mode, TranslationMode::Local);
    if use_local {
        #[cfg(feature = "local")]
        {
            let detector = crate::language_detector::WhatLangDetector::new();
            match detector.detect_language(&text).await {
                Ok(info) => output::ok(info),
                Err(e) => output::err(format!("Detection failed: {}", e)),
            }
        }
        #[cfg(not(feature = "local"))]
        {
            let _ = (config, text);
            output::err("Local detection not enabled. Compile with --features local");
        }
    } else {
        let detector = crate::language_detector::AiLanguageDetector::new(&config.ai.model);
        match detector.detect_language(&text).await {
            Ok(info) => output::ok(info),
            Err(e) => output::err(format!("Detection failed: {}", e)),
        }
    }
    Ok(())
}

fn handle_config(config: Config, generate: bool) -> Result<()> {
    if generate {
        println!("{}", crate::config::generate_example_config());
        return Ok(());
    }
    eprintln!();
    eprintln!("  {}", output::bold("Current Configuration"));
    eprintln!("  {}", output::dim("─".repeat(42)));
    eprintln!(
        "  {}  {}",
        output::dim("Service:"),
        output::bold(config.mode.display_name())
    );
    eprintln!(
        "  {}  {}",
        output::dim("Source:  "),
        config.translation.default_source_lang
    );
    eprintln!(
        "  {}  {}",
        output::dim("Targets: "),
        config.translation.default_target_langs.join(", ")
    );
    if let Some(ctx) = &config.translation.context {
        eprintln!("  {}  {}", output::dim("Context: "), ctx);
    }
    if let Some(limit) = config.mode.char_limit() {
        eprintln!("  {}  {} chars", output::dim("Char limit:"), limit);
    }
    let section = config.mode.display_section_name();
    if !section.is_empty() {
        eprintln!("  {}", output::dim(section));
        for (label, value) in config.mode.display_settings(&config) {
            eprintln!("    {}  {}", output::dim(label), value);
        }
    }
    eprintln!();
    eprintln!(
        "  {}  zellig config --generate",
        output::dim("Generate example config:")
    );
    Ok(())
}

async fn handle_models(
    config: Config,
    config_path: Option<&str>,
    search: Option<String>,
    download: Option<String>,
    installed: bool,
    remove: Option<String>,
) -> Result<()> {
    if installed {
        manager::print_installed(&config.local.model_repo);
        return Ok(());
    }

    if let Some(repo) = remove {
        #[cfg(feature = "local")]
        {
            manager::uninstall_model_from_cache(&repo)?;
            output::ok(format!("Removed: {}", repo));
        }
        #[cfg(not(feature = "local"))]
        {
            let _ = repo;
            output::err("Remove requires --features local");
        }
        return Ok(());
    }

    if let Some(repo) = download {
        #[cfg(feature = "local")]
        {
            output::info(format!("Downloading: {}", repo));
            manager::download_model(&repo).await?;
            output::ok("Download complete");
        }
        #[cfg(not(feature = "local"))]
        {
            output::err("Download requires --features local");
            output::info(format!(
                r#"Rebuild: cargo run --features local -- models --download "{}""#,
                repo
            ));
        }
        let resolved = manager::resolve_config_path(config_path);
        manager::set_model_in_config(&resolved, &repo)?;
        output::ok(format!("Model set in config: {}", resolved.display()));
        return Ok(());
    }

    manager::search_and_print(search.as_deref(), &config.local.model_repo).await;
    Ok(())
}

async fn translate_dir(
    translator: &dyn Translator,
    dir_path: &str,
    source_lang: &str,
    target_langs: &[String],
    context: Option<&str>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir_path).map_err(ZelligError::IoError)?;
    let mut files: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_file() {
                return None;
            }
            let s = p.to_string_lossy().to_string();
            if crate::formats::detect_format(&s).is_some() {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    files.sort();

    if files.is_empty() {
        output::ok("No supported files found in directory");
        return Ok(());
    }

    output::ok(format!(
        "Found {} supported file(s) in {}",
        files.len(),
        dir_path
    ));
    eprintln!();

    for file_path in &files {
        translate_file(translator, file_path, source_lang, target_langs, context).await?;
    }
    Ok(())
}

fn make_output_path(file_path: &str, target_lang: &str) -> String {
    let p = std::path::Path::new(file_path);
    if let Some(ext) = p.extension() {
        format!(
            "{}.{}.{}",
            p.with_extension("").display(),
            target_lang,
            ext.to_string_lossy()
        )
    } else {
        format!("{}.{}", file_path, target_lang)
    }
}

pub(crate) async fn translate_file_single(
    translator: &dyn Translator,
    file_path: &str,
    source_lang: &str,
    target_lang: &str,
    context: Option<&str>,
) -> Result<(usize, String)> {
    let handler = crate::formats::detect_format(file_path).ok_or_else(|| {
        crate::error::ZelligError::ConfigError(format!("Unsupported file format: {}", file_path))
    })?;
    let content = std::fs::read_to_string(file_path).map_err(crate::error::ZelligError::IoError)?;
    let translatable = handler.extract(&content).await?;
    if translatable.is_empty() {
        return Ok((0, String::new()));
    }
    let texts: Vec<String> = translatable.iter().map(|i| i.text.clone()).collect();
    let ids: Vec<String> = translatable.iter().map(|i| i.id.clone()).collect();
    let results = translator
        .batch_translate(&texts, source_lang, target_lang, context)
        .await?;
    let translations: Vec<crate::formats::TranslatedString> = ids
        .into_iter()
        .zip(results)
        .map(|(id, translated_text)| crate::formats::TranslatedString {
            id,
            translated_text,
        })
        .collect();
    let translated_content = handler.reconstruct(&content, &translations).await?;
    let output_path = make_output_path(file_path, target_lang);
    std::fs::write(&output_path, translated_content).map_err(crate::error::ZelligError::IoError)?;
    Ok((translatable.len(), output_path))
}

pub(crate) async fn translate_file(
    translator: &dyn Translator,
    file_path: &str,
    source_lang: &str,
    target_langs: &[String],
    context: Option<&str>,
) -> Result<()> {
    let handler = crate::formats::detect_format(file_path).ok_or_else(|| {
        crate::error::ZelligError::ConfigError(format!("Unsupported file format: {}", file_path))
    })?;

    let content = std::fs::read_to_string(file_path).map_err(crate::error::ZelligError::IoError)?;

    let extract_start = std::time::Instant::now();
    let translatable = handler.extract(&content).await?;
    let extract_dur = extract_start.elapsed();
    if translatable.is_empty() {
        output::ok("No translatable strings found");
        return Ok(());
    }

    output::ok(format!(
        "Extracted {} strings from {}  {}",
        translatable.len(),
        file_path,
        output::dim(format!("({})", output::elapsed(extract_dur.as_secs_f64()))),
    ));
    eprintln!();

    let texts: Vec<String> = translatable.iter().map(|item| item.text.clone()).collect();
    let ids: Vec<String> = translatable.iter().map(|item| item.id.clone()).collect();
    let n = translatable.len();

    let mp = indicatif::MultiProgress::new();
    let pbs: Vec<ProgressBar> = target_langs
        .iter()
        .map(|target_lang| {
            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::with_template(&format!(
                    "  {} {} {}  {{spinner:.green}} translating...",
                    output::dim("["),
                    output::bold(target_lang),
                    output::dim("]"),
                ))
                .expect("hardcoded indicatif template"),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        })
        .collect();

    let per_lang = try_join_all(target_langs.iter().zip(pbs).map(|(target_lang, pb)| {
        let texts: &[String] = &texts;
        async move {
            let start = std::time::Instant::now();
            let results = translator
                .batch_translate(texts, source_lang, target_lang, context)
                .await;
            let elapsed = start.elapsed();
            pb.finish_and_clear();
            results.map(|r| (target_lang.as_str(), r, elapsed))
        }
    }))
    .await?;

    for (target_lang, results, elapsed) in per_lang {
        eprintln!("{}", output::header(target_lang));
        let avg = elapsed / n as u32;
        let translations: Vec<crate::formats::TranslatedString> = ids
            .iter()
            .cloned()
            .zip(results)
            .map(|(id, translated_text)| crate::formats::TranslatedString {
                id,
                translated_text,
            })
            .collect();

        let translated_content = handler.reconstruct(&content, &translations).await?;
        let output_path = make_output_path(file_path, target_lang);
        std::fs::write(&output_path, translated_content)
            .map_err(crate::error::ZelligError::IoError)?;

        output::ok(format!(
            "{}  {} strings in {}  {}",
            output::bold(target_lang),
            n,
            output::elapsed(elapsed.as_secs_f64()),
            output::dim(format!("(avg {})", output::elapsed(avg.as_secs_f64()))),
        ));
        output::info(format!("Written to {}", output_path));
        eprintln!("{}", output::footer());
        eprintln!();
    }

    Ok(())
}
