//! Hybrid プロバイダー実装

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tracing::info;

use crate::api::vision::ImageData;
use crate::error::Result;
use crate::provider::{LlmProvider, ProviderType, RawResponse, VisionBackend};
use crate::types::{ExtractionResult, TextBlock, TextDirection, TokenUsage};

/// Hybrid プロバイダー
///
/// Vision バックエンド（GCP Vision など）でテキストを検出し、
/// LLM で構造化・後処理を行うプロバイダー
pub struct HybridProvider<V: VisionBackend, L: LlmProvider> {
    /// Vision バックエンド
    vision: Arc<V>,
    /// LLM プロバイダー
    llm: Arc<L>,
}

impl<V: VisionBackend, L: LlmProvider> HybridProvider<V, L> {
    /// 新しい Hybrid プロバイダーを作成
    pub fn new(vision: V, llm: L) -> Self {
        Self {
            vision: Arc::new(vision),
            llm: Arc::new(llm),
        }
    }

    /// Arc で包まれたコンポーネントから作成
    pub fn from_arc(vision: Arc<V>, llm: Arc<L>) -> Self {
        Self { vision, llm }
    }

    /// Vision バックエンドへの参照を取得
    pub fn vision(&self) -> &V {
        &self.vision
    }

    /// LLM プロバイダーへの参照を取得
    pub fn llm(&self) -> &L {
        &self.llm
    }
}

#[async_trait]
impl<V: VisionBackend + Send + Sync, L: LlmProvider + Send + Sync> LlmProvider
    for HybridProvider<V, L>
{
    fn provider_id(&self) -> ProviderType {
        ProviderType::Hybrid
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn model_name(&self) -> &str {
        self.llm.model_name()
    }

    async fn extract_text(
        &self,
        image: &ImageData,
        prompt: &str,
        direction: TextDirection,
    ) -> Result<ExtractionResult> {
        let start_time = Instant::now();

        // Step 1: Vision バックエンドでテキスト検出
        let vision_result = self.vision.extract_raw_text(image).await?;

        info!(
            "Vision バックエンド完了: {} 文字検出, 信頼度: {:.2}",
            vision_result.text.len(),
            vision_result.confidence
        );

        // Step 2: 検出したテキストと画像を LLM に渡して構造化
        let llm_prompt = format!(
            "以下は画像から OCR で検出されたテキストです:\n\n```\n{}\n```\n\n{}\n\n画像を参照しながら、テキストを正確に抽出・整形してください。",
            vision_result.text, prompt
        );

        let llm_result = self.llm.extract_text(image, &llm_prompt, direction).await?;

        let processing_time = start_time.elapsed();

        Ok(ExtractionResult {
            text: llm_result.text,
            confidence: vision_result.confidence * llm_result.confidence,
            language: vision_result
                .detected_language
                .unwrap_or_else(|| llm_result.language.clone()),
            direction: llm_result.direction,
            blocks: llm_result.blocks,
            processing_time,
            model: format!("hybrid({}, {})", self.vision.backend_id(), llm_result.model),
            tokens_used: llm_result.tokens_used,
        })
    }

    async fn call_vision_raw(&self, image: &ImageData, prompt: &str) -> Result<RawResponse> {
        let start_time = Instant::now();

        // Vision バックエンドでテキスト検出
        let vision_result = self.vision.extract_raw_text(image).await?;

        // LLM で後処理
        let llm_prompt = format!(
            "以下は画像から OCR で検出されたテキストです:\n\n```\n{}\n```\n\n{}",
            vision_result.text, prompt
        );

        let llm_result = self.llm.call_vision_raw(image, &llm_prompt).await?;
        let latency = start_time.elapsed();

        Ok(RawResponse {
            text: llm_result.text,
            usage: llm_result.usage,
            model: format!("hybrid({}, {})", self.vision.backend_id(), llm_result.model),
            latency,
            finish_reason: llm_result.finish_reason,
        })
    }

    async fn extract_structured(
        &self,
        image: &ImageData,
        prompt: &str,
        schema: &str,
    ) -> Result<String> {
        // Vision バックエンドでテキスト検出
        let vision_result = self.vision.extract_raw_text(image).await?;

        // LLM で構造化
        let llm_prompt = format!(
            "以下は画像から OCR で検出されたテキストです:\n\n```\n{}\n```\n\n{}",
            vision_result.text, prompt
        );

        self.llm.extract_structured(image, &llm_prompt, schema).await
    }

    async fn health_check(&self) -> Result<bool> {
        // 両方のコンポーネントが正常かチェック
        let llm_ok = self.llm.health_check().await?;
        Ok(llm_ok)
    }
}

/// Vision のみモードで使用する場合のラッパー
///
/// LLM を使用せず、Vision バックエンドの結果をそのまま返す
pub struct VisionOnlyProvider<V: VisionBackend> {
    vision: Arc<V>,
}

impl<V: VisionBackend> VisionOnlyProvider<V> {
    /// 新しい Vision のみプロバイダーを作成
    pub fn new(vision: V) -> Self {
        Self {
            vision: Arc::new(vision),
        }
    }
}

#[async_trait]
impl<V: VisionBackend + Send + Sync> LlmProvider for VisionOnlyProvider<V> {
    fn provider_id(&self) -> ProviderType {
        ProviderType::Hybrid
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn model_name(&self) -> &str {
        self.vision.backend_id()
    }

    async fn extract_text(
        &self,
        image: &ImageData,
        _prompt: &str,
        direction: TextDirection,
    ) -> Result<ExtractionResult> {
        let start_time = Instant::now();
        let result = self.vision.extract_raw_text(image).await?;
        let processing_time = start_time.elapsed();

        Ok(ExtractionResult {
            text: result.text.clone(),
            confidence: result.confidence,
            language: result.detected_language.unwrap_or_else(|| "unknown".to_string()),
            direction,
            blocks: vec![TextBlock {
                text: result.text,
                bbox: None,
                confidence: result.confidence,
                direction,
                font_size: None,
            }],
            processing_time,
            model: self.vision.backend_id().to_string(),
            tokens_used: TokenUsage::default(),
        })
    }

    async fn call_vision_raw(&self, image: &ImageData, _prompt: &str) -> Result<RawResponse> {
        let start_time = Instant::now();
        let result = self.vision.extract_raw_text(image).await?;
        let latency = start_time.elapsed();

        Ok(RawResponse {
            text: result.text,
            usage: TokenUsage::default(),
            model: self.vision.backend_id().to_string(),
            latency,
            finish_reason: Some(crate::provider::FinishReason::EndTurn),
        })
    }

    async fn extract_structured(
        &self,
        image: &ImageData,
        _prompt: &str,
        _schema: &str,
    ) -> Result<String> {
        // Vision のみでは構造化できないので、生テキストを返す
        let result = self.vision.extract_raw_text(image).await?;
        Ok(result.text)
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    // Hybrid プロバイダーのテストは統合テストで行う
}
