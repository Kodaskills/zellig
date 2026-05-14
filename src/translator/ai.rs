use crate::config::AiConfig;
use crate::error::{Result, ZelligError};
use crate::translator::{PromptBuilder, Translator};
use futures::future::try_join_all;
use std::future::Future;
use std::pin::Pin;

pub struct AiTranslator {
    client: genai::Client,
    model: String,
}

impl AiTranslator {
    pub fn new(config: &AiConfig) -> Result<Self> {
        Ok(Self {
            client: genai::Client::builder().build(),
            model: config.model.clone(),
        })
    }
}

impl Translator for AiTranslator {
    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        let (text, source_lang, target_lang) = (
            text.to_string(),
            source_lang.to_string(),
            target_lang.to_string(),
        );
        let (client, model) = (self.client.clone(), self.model.clone());
        let context = context.map(|c| c.to_string());

        Box::pin(async move {
            let system_msg =
                PromptBuilder::build_system(&source_lang, &target_lang, context.as_deref());
            let response = client
                .exec_chat(
                    &model,
                    genai::chat::ChatRequest::new(vec![
                        genai::chat::ChatMessage::system(&system_msg),
                        genai::chat::ChatMessage::user(&text),
                    ]),
                    None,
                )
                .await
                .map_err(|e| ZelligError::AiError(e.to_string()))?;

            response
                .first_text()
                .map(|s| s.to_string())
                .ok_or_else(|| ZelligError::AiError("Empty response".to_string()))
        })
    }

    fn batch_translate<'a>(
        &'a self,
        texts: &'a [String],
        source_lang: &'a str,
        target_lang: &'a str,
        context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + 'a>> {
        Box::pin(async move {
            try_join_all(
                texts
                    .iter()
                    .map(|t| self.translate(t, source_lang, target_lang, context)),
            )
            .await
        })
    }
}
