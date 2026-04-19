use crate::models::{Node, RootJson};
use crate::error::{AppError, Result};
use crate::models::Components;
use gliner::model::{params::Parameters as GlinerParams, pipeline::span::SpanMode, GLiNER};
use gte::{params::Parameters as GteParams, rerank::pipeline::RerankingPipeline};
use orp::model::Model;
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

const DEFAULT_GTE_MODEL_URL: &str =
    "https://huggingface.co/Alibaba-NLP/gte-reranker-modernbert-base/resolve/main/onnx/model.onnx";
const DEFAULT_GTE_TOKENIZER_URL: &str =
    "https://huggingface.co/Alibaba-NLP/gte-modernbert-base/resolve/main/tokenizer.json";
const DEFAULT_GLINER_MODEL_URL: &str =
    "https://huggingface.co/onnx-community/gliner_small-v2.1/resolve/main/onnx/model.onnx";
const DEFAULT_GLINER_TOKENIZER_URL: &str =
    "https://huggingface.co/onnx-community/gliner_small-v2.1/resolve/main/tokenizer.json";
const DEFAULT_CONFIG_PATH: &str = "data.json";

#[derive(Clone)]
pub struct ModelAssetPaths {
    pub gte_model: String,
    pub gte_tokenizer: String,
    pub gliner_model: String,
    pub gliner_tokenizer: String,
}

pub fn config_path() -> String {
    std::env::var("EFFICEINTNLP_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string())
}

pub fn model_asset_paths() -> ModelAssetPaths {
    let model_dir = std::env::var("EFFICEINTNLP_MODEL_DIR").unwrap_or_else(|_| "models".to_string());

    let default_path = |relative: &str| -> String {
        PathBuf::from(&model_dir)
            .join(relative)
            .to_string_lossy()
            .to_string()
    };

    ModelAssetPaths {
        gte_model: std::env::var("EFFICEINTNLP_GTE_MODEL_PATH")
            .unwrap_or_else(|_| default_path("gte/model.onnx")),
        gte_tokenizer: std::env::var("EFFICEINTNLP_GTE_TOKENIZER_PATH")
            .unwrap_or_else(|_| default_path("gte/tokenizer.json")),
        gliner_model: std::env::var("EFFICEINTNLP_GLINER_MODEL_PATH")
            .unwrap_or_else(|_| default_path("gliner/model.onnx")),
        gliner_tokenizer: std::env::var("EFFICEINTNLP_GLINER_TOKENIZER_PATH")
            .unwrap_or_else(|_| default_path("gliner/tokenizer.json")),
    }
}

pub struct ModelResources {
    pub router_model: Arc<Model>,
    pub router_pipeline: Arc<RerankingPipeline>,
    pub router_params: Arc<GteParams>,
    pub gliner: Arc<GLiNER<SpanMode>>,
}

#[derive(Clone)]
pub struct RoutingResources {
    pub router_model: Arc<Model>,
    pub router_pipeline: Arc<RerankingPipeline>,
    pub router_params: Arc<GteParams>,
}

pub struct AppState {
    pub resources: RwLock<ModelResources>,
    pub root: RwLock<Node>,
    pub components: RwLock<Option<Components>>,
    pub http: Client,
}

impl AppState {
    fn load_config() -> Result<RootJson> {
        let config_path = config_path();
        let raw = fs::read_to_string(&config_path).map_err(|e| {
            AppError::InternalServerError(format!("Failed to read config '{}': {}", config_path, e))
        })?;
        serde_json::from_str(&raw).map_err(|e| {
            AppError::InternalServerError(format!("Failed to parse config '{}': {}", config_path, e))
        })
    }

    pub async fn new() -> Result<Self> {
        let parsed = Self::load_config()?;
        info!("🚀 Loading models at startup");
        let resources = Self::load_model_resources(parsed.components.as_ref()).await?;
        info!("✅ Models ready");

        Ok(Self {
            resources: RwLock::new(resources),
            root: RwLock::new(parsed.architecture),
            components: RwLock::new(parsed.components),
            http: Client::new(),
        })
    }

    pub async fn update_agent_config(&self, new_config: RootJson) -> Result<()> {
        let current_components = self.components.read().await.clone();
        let should_reload_models = current_components != new_config.components;

        if should_reload_models {
            info!("🔁 Reloading models for agent change");
            let resources = Self::load_model_resources(new_config.components.as_ref()).await?;
            let mut resources_lock = self.resources.write().await;
            *resources_lock = resources;
            drop(resources_lock);
        } else {
            info!("⚡ Applying agent config without model reload");
        }

        let mut root_lock = self.root.write().await;
        *root_lock = new_config.architecture;
        drop(root_lock);

        let mut components_lock = self.components.write().await;
        *components_lock = new_config.components;
        info!("✅ Agent config applied");
        Ok(())
    }

    pub async fn routing_resources(&self) -> RoutingResources {
        let resources = self.resources.read().await;
        RoutingResources {
            router_model: resources.router_model.clone(),
            router_pipeline: resources.router_pipeline.clone(),
            router_params: resources.router_params.clone(),
        }
    }

    pub async fn gliner_model(&self) -> Arc<GLiNER<SpanMode>> {
        self.resources.read().await.gliner.clone()
    }

    #[cfg(feature = "ui")]
    pub async fn reload_models(&self) -> Result<()> {
        info!("🔁 Reloading models from current files");
        let components = self.components.read().await.clone();
        let resources = Self::load_model_resources(components.as_ref()).await?;
        let mut resources_lock = self.resources.write().await;
        *resources_lock = resources;
        info!("✅ Models reloaded");
        Ok(())
    }

    async fn load_model_resources(components: Option<&Components>) -> Result<ModelResources> {
        let paths = model_asset_paths();
        let gte_model_url = components
            .and_then(|c| c.function_mapper.as_ref())
            .and_then(|m| m.model_url.clone())
            .unwrap_or_else(|| DEFAULT_GTE_MODEL_URL.into());

        let gte_tokenizer_url = components
            .and_then(|c| c.function_mapper.as_ref())
            .and_then(|m| m.tokenizer_url.clone())
            .unwrap_or_else(|| DEFAULT_GTE_TOKENIZER_URL.into());

        let gliner_model_url = components
            .and_then(|c| c.entity_recognizer.as_ref())
            .and_then(|m| m.model_url.clone())
            .unwrap_or_else(|| DEFAULT_GLINER_MODEL_URL.into());

        let gliner_tokenizer_url = components
            .and_then(|c| c.entity_recognizer.as_ref())
            .and_then(|m| m.tokenizer_url.clone())
            .unwrap_or_else(|| DEFAULT_GLINER_TOKENIZER_URL.into());

        Self::prepare_model_asset(&gte_model_url, &paths.gte_model).await?;
        Self::prepare_model_asset(&gte_tokenizer_url, &paths.gte_tokenizer).await?;
        Self::prepare_model_asset(&gliner_model_url, &paths.gliner_model).await?;
        Self::prepare_model_asset(&gliner_tokenizer_url, &paths.gliner_tokenizer).await?;

        let gte_model_path = paths.gte_model.clone();
        let gte_tokenizer_path = paths.gte_tokenizer.clone();
        let gliner_model_path = paths.gliner_model.clone();
        let gliner_tokenizer_path = paths.gliner_tokenizer.clone();

        tokio::task::spawn_blocking(move || {
            let params = Arc::new(GteParams::default().with_sigmoid(true));

            let pipeline = RerankingPipeline::new(&gte_tokenizer_path, &params)
                .map_err(|e| AppError::InternalServerError(format!("Failed to create RerankingPipeline: {}", e)))?;

            let model = Model::new(&gte_model_path, Default::default())
                .map_err(|e| AppError::InternalServerError(format!("Failed to create GTE Model: {}", e)))?;

            let gliner = GLiNER::<SpanMode>::new(
                GlinerParams::default(),
                Default::default(),
                &gliner_tokenizer_path,
                &gliner_model_path,
            )
            .map_err(|e| AppError::InternalServerError(format!("Failed to create GLiNER model: {}", e)))?;

            Ok(ModelResources {
                router_model: Arc::new(model),
                router_pipeline: Arc::new(pipeline),
                router_params: params,
                gliner: Arc::new(gliner),
            })
        })
        .await
        .map_err(|e| AppError::InternalServerError(format!("Task join error while loading models: {}", e)))?
    }

    async fn prepare_model_asset(source: &str, destination: &str) -> Result<()> {
        if source.starts_with("http://") || source.starts_with("https://") {
            crate::downloader::download_if_missing(source, destination).await
        } else if !source.trim().is_empty() {
            if source != destination {
                let bytes = tokio::fs::read(source)
                    .await
                    .map_err(|e| AppError::InternalServerError(format!("Failed to read model asset '{}': {}", source, e)))?;
                if let Some(parent) = std::path::Path::new(destination).parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| AppError::InternalServerError(format!("Failed to create model directory: {}", e)))?;
                }
                tokio::fs::write(destination, bytes)
                    .await
                    .map_err(|e| AppError::InternalServerError(format!("Failed to write model asset '{}': {}", destination, e)))?;
                info!("📦 Updated model asset {}", destination);
            }
            Ok(())
        } else {
            Err(AppError::BadRequest("Model asset source cannot be empty".to_string()))
        }
    }
}
