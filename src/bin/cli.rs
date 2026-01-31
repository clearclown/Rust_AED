//! Rust AED CLI
//!
//! コマンドラインからドキュメントのテキスト抽出を行います。

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use rust_aed::{AedClient, OcrPreset, TextDirection};

/// Rust AED - Claude Vision API を使用した OCR ツール
#[derive(Parser)]
#[command(name = "aed")]
#[command(version, about, long_about = None)]
struct Cli {
    /// 詳細出力モード
    #[arg(short, long)]
    verbose: bool,

    /// 設定ファイルパス
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 画像からテキストを抽出
    Extract {
        /// 入力ファイルパス
        input: PathBuf,

        /// 出力ファイルパス（省略時は標準出力）
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// プリセット
        #[arg(short, long, value_enum, default_value = "general")]
        preset: PresetArg,

        /// テキスト方向
        #[arg(short, long, value_enum, default_value = "auto")]
        direction: DirectionArg,

        /// 言語（ISO 639-1 コード）
        #[arg(short, long)]
        language: Option<String>,
    },

    /// ディレクトリ内の画像を一括処理
    Batch {
        /// 入力ディレクトリ
        input_dir: PathBuf,

        /// 出力ディレクトリ
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// 並列度
        #[arg(short, long, default_value = "5")]
        concurrency: usize,

        /// プリセット
        #[arg(short = 'P', long, value_enum, default_value = "general")]
        preset: PresetArg,
    },

    /// 構造化データを抽出（JSON 出力）
    Structured {
        /// 入力ファイルパス
        input: PathBuf,

        /// JSON Schema ファイル（省略時は自動推論）
        #[arg(short, long)]
        schema: Option<PathBuf>,

        /// 出力ファイルパス（省略時は標準出力）
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// キャッシュを管理
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// キャッシュをクリア
    Clear,
    /// キャッシュ情報を表示
    Info,
    /// 期限切れキャッシュを削除
    Cleanup,
}

#[derive(Clone, ValueEnum)]
enum PresetArg {
    General,
    JapaneseBook,
    Manga,
    Invoice,
    BusinessCard,
    Receipt,
    Form,
    Table,
}

impl From<PresetArg> for OcrPreset {
    fn from(arg: PresetArg) -> Self {
        match arg {
            PresetArg::General => OcrPreset::General,
            PresetArg::JapaneseBook => OcrPreset::JapaneseBook,
            PresetArg::Manga => OcrPreset::Manga,
            PresetArg::Invoice => OcrPreset::Invoice,
            PresetArg::BusinessCard => OcrPreset::BusinessCard,
            PresetArg::Receipt => OcrPreset::Receipt,
            PresetArg::Form => OcrPreset::Form,
            PresetArg::Table => OcrPreset::Table,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum DirectionArg {
    Auto,
    Vertical,
    Horizontal,
}

impl From<DirectionArg> for TextDirection {
    fn from(arg: DirectionArg) -> Self {
        match arg {
            DirectionArg::Auto => TextDirection::Auto,
            DirectionArg::Vertical => TextDirection::Vertical,
            DirectionArg::Horizontal => TextDirection::Horizontal,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // ログ設定
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("rust_aed=debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("rust_aed=warn")
            .init();
    }

    // クライアント初期化
    let client = if let Some(config_path) = &cli.config {
        AedClient::from_config_file(config_path)?
    } else {
        AedClient::from_env()?
    };

    match cli.command {
        Commands::Extract {
            input,
            output,
            preset,
            direction,
            language,
        } => {
            run_extract(&client, input, output, preset, direction, language).await?;
        }

        Commands::Batch {
            input_dir,
            output_dir,
            concurrency,
            preset,
        } => {
            run_batch(&client, input_dir, output_dir, concurrency, preset).await?;
        }

        Commands::Structured {
            input,
            schema: _,
            output,
        } => {
            run_structured(&client, input, output).await?;
        }

        Commands::Cache { action } => {
            run_cache(action)?;
        }
    }

    Ok(())
}

async fn run_extract(
    client: &AedClient,
    input: PathBuf,
    output: Option<PathBuf>,
    preset: PresetArg,
    direction: DirectionArg,
    language: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("抽出中: {}", input.display());

    let mut builder = client
        .extract_text(&input)
        .preset(preset.into())
        .direction(direction.into());

    if let Some(lang) = language {
        builder = builder.language(&lang);
    }

    let result = builder.await?;

    // 出力
    if let Some(output_path) = output {
        std::fs::write(&output_path, &result.text)?;
        eprintln!("出力: {}", output_path.display());
    } else {
        println!("{}", result.text);
    }

    eprintln!(
        "完了: {}トークン使用, {:?}",
        result.tokens_used.total(),
        result.processing_time
    );

    Ok(())
}

async fn run_batch(
    client: &AedClient,
    input_dir: PathBuf,
    output_dir: Option<PathBuf>,
    concurrency: usize,
    preset: PresetArg,
) -> Result<(), Box<dyn std::error::Error>> {
    use rust_aed::extraction::batch::BatchExtractBuilder;

    eprintln!("バッチ処理開始: {}", input_dir.display());

    // 画像ファイルを収集
    let supported_extensions = ["png", "jpg", "jpeg", "gif", "webp"];
    let mut paths = Vec::new();

    for entry in std::fs::read_dir(&input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && supported_extensions.contains(&ext.to_lowercase().as_str())
        {
            paths.push(path);
        }
    }

    paths.sort();
    eprintln!("ファイル数: {}", paths.len());

    let result = BatchExtractBuilder::new(client, paths)
        .concurrency(concurrency)
        .preset(preset.into())
        .execute()
        .await;

    // 結果出力
    if let Some(out_dir) = output_dir {
        std::fs::create_dir_all(&out_dir)?;
        for (i, extraction) in result.successful.iter().enumerate() {
            let out_path = out_dir.join(format!("{:04}.txt", i));
            std::fs::write(&out_path, &extraction.text)?;
        }
        eprintln!("出力ディレクトリ: {}", out_dir.display());
    } else {
        for extraction in &result.successful {
            println!("---");
            println!("{}", extraction.text);
        }
    }

    eprintln!(
        "完了: 成功 {}/{}, 所要時間 {:?}",
        result.successful.len(),
        result.successful.len() + result.failed.len(),
        result.total_time
    );

    if !result.failed.is_empty() {
        eprintln!("失敗したファイル:");
        for (path, err) in &result.failed {
            eprintln!("  {}: {}", path.display(), err);
        }
    }

    Ok(())
}

async fn run_structured(
    client: &AedClient,
    input: PathBuf,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("構造化抽出中: {}", input.display());

    // 汎用的な JSON 抽出
    let result = client
        .extract_text(&input)
        .preset(OcrPreset::Invoice)
        .await?;

    let json_output = result.text;

    if let Some(output_path) = output {
        std::fs::write(&output_path, &json_output)?;
        eprintln!("出力: {}", output_path.display());
    } else {
        println!("{}", json_output);
    }

    Ok(())
}

fn run_cache(action: CacheAction) -> Result<(), Box<dyn std::error::Error>> {
    use rust_aed::cache::ResponseCache;
    use rust_aed::config::CacheConfig;

    let config = CacheConfig::default();
    let cache = ResponseCache::new(&config)?;

    match action {
        CacheAction::Clear => {
            cache.clear()?;
            eprintln!("キャッシュをクリアしました");
        }
        CacheAction::Info => {
            let size = cache.size()?;
            eprintln!("キャッシュディレクトリ: {}", config.directory);
            eprintln!("キャッシュサイズ: {} バイト", size);
            eprintln!("TTL: {} 時間", config.ttl_hours);
        }
        CacheAction::Cleanup => {
            let stats = cache.cleanup()?;
            eprintln!(
                "クリーンアップ完了: {} ファイル中 {} ファイルを削除",
                stats.total_files, stats.removed_files
            );
        }
    }

    Ok(())
}
