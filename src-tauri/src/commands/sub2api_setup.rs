use crate::app_config::AppType;
use crate::config::atomic_write;
use crate::provider::{Provider, ProviderMeta};
use crate::services::ProviderService;
use crate::store::AppState;
use crate::sub2api_brand;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
struct SetupConfig {
    site_name: String,
    #[allow(dead_code)]
    site_logo: String,
    api_base_url: String,
    api_key: String,
    model: String,
    reasoning_effort: String,
    wire_api: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SetupProfile {
    site_name: String,
    #[allow(dead_code)]
    site_logo: String,
    api_base_url: String,
    model: String,
    reasoning_effort: String,
    wire_api: String,
    setup_enabled: bool,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    data: T,
}

#[derive(Debug, Clone)]
struct PendingSetup {
    config: SetupConfig,
    ticket: Option<String>,
}

#[derive(Default)]
pub struct Sub2APISetupState {
    pending: Mutex<Option<PendingSetup>>,
    queued_ticket: Mutex<Option<String>>,
}

impl Sub2APISetupState {
    pub fn queue_ticket(&self, ticket: String) {
        if let Ok(mut queued) = self.queued_ticket.lock() {
            *queued = Some(ticket);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupPreview {
    site_name: String,
    api_base_url: String,
    model: String,
    reasoning_effort: String,
    wire_api: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupApplyResult {
    configured: bool,
    verified: bool,
    server_confirmed: bool,
    backup_directory: String,
    restart_required: bool,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Unable to initialize the secure setup client".into())
}

async fn post_json<T: DeserializeOwned>(path: &str, body: Value) -> Result<T, String> {
    let response = http_client()?
        .post(sub2api_brand::api_url(path))
        .header(reqwest::header::CACHE_CONTROL, "no-store")
        .json(&body)
        .send()
        .await
        .map_err(|_| "Unable to contact Sub2API")?;
    if !response.status().is_success() {
        return Err("The setup ticket is invalid, expired, or already used".into());
    }
    response
        .json::<ApiEnvelope<T>>()
        .await
        .map(|envelope| envelope.data)
        .map_err(|_| "Sub2API returned an invalid setup response".into())
}

async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let response = http_client()?
        .get(sub2api_brand::api_url(path))
        .header(reqwest::header::CACHE_CONTROL, "no-store")
        .send()
        .await
        .map_err(|_| "Unable to contact Sub2API")?;
    if !response.status().is_success() {
        return Err("Sub2API client setup is unavailable".into());
    }
    response
        .json::<ApiEnvelope<T>>()
        .await
        .map(|envelope| envelope.data)
        .map_err(|_| "Sub2API returned an invalid profile".into())
}

fn validate_config(config: SetupConfig) -> Result<SetupConfig, String> {
    if config.api_key.trim().is_empty() || config.api_key.len() > 1024 {
        return Err("Sub2API returned an invalid API key".into());
    }
    let base_url = sub2api_brand::validate_service_base_url(&config.api_base_url)?;
    if config.wire_api != "responses" {
        return Err("This build only accepts the OpenAI Responses API".into());
    }
    if config.model != sub2api_brand::DEFAULT_MODEL
        || config.reasoning_effort != sub2api_brand::DEFAULT_REASONING_EFFORT
    {
        return Err("Sub2API returned a configuration unsupported by this build".into());
    }
    Ok(SetupConfig {
        api_base_url: base_url,
        ..config
    })
}

fn preview(config: &SetupConfig) -> SetupPreview {
    SetupPreview {
        site_name: config.site_name.clone(),
        api_base_url: config.api_base_url.clone(),
        model: config.model.clone(),
        reasoning_effort: config.reasoning_effort.clone(),
        wire_api: config.wire_api.clone(),
    }
}

#[tauri::command]
pub async fn sub2api_prepare_ticket(
    setup_state: State<'_, Sub2APISetupState>,
    ticket: String,
) -> Result<SetupPreview, String> {
    let ticket = sub2api_brand::parse_setup_ticket(&format!("sub2api://v1/setup?ticket={ticket}"))?;
    let config: SetupConfig = post_json(
        "/api/v1/client-setup/exchange",
        json!({ "ticket": &ticket }),
    )
    .await?;
    let config = validate_config(config)?;
    let result = preview(&config);
    *setup_state
        .pending
        .lock()
        .map_err(|_| "Setup state is unavailable")? = Some(PendingSetup {
        config,
        ticket: Some(ticket),
    });
    Ok(result)
}

#[tauri::command]
pub async fn sub2api_prepare_manual(
    setup_state: State<'_, Sub2APISetupState>,
    api_key: String,
) -> Result<SetupPreview, String> {
    let key = api_key.trim().to_string();
    if key.is_empty() || key.len() > 1024 {
        return Err("Enter a valid API key".into());
    }
    let profile: SetupProfile = get_json("/api/v1/client-setup/profile").await?;
    if !profile.setup_enabled {
        return Err("Sub2API client setup is temporarily disabled".into());
    }
    let config = validate_config(SetupConfig {
        site_name: profile.site_name,
        site_logo: profile.site_logo,
        api_base_url: profile.api_base_url,
        api_key: key,
        model: profile.model,
        reasoning_effort: profile.reasoning_effort,
        wire_api: profile.wire_api,
    })?;
    let result = preview(&config);
    *setup_state
        .pending
        .lock()
        .map_err(|_| "Setup state is unavailable")? = Some(PendingSetup {
        config,
        ticket: None,
    });
    Ok(result)
}

async fn verify_models(config: &SetupConfig) -> Result<(), String> {
    let response = http_client()?
        .get(sub2api_brand::models_url(&config.api_base_url))
        .bearer_auth(&config.api_key)
        .header(reqwest::header::CACHE_CONTROL, "no-store")
        .send()
        .await
        .map_err(|_| "Could not connect to /v1/models")?;
    let status = response.status();
    if !status.is_success() {
        return Err(models_verification_error(status));
    }
    Ok(())
}

fn models_verification_error(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 => "The API key was rejected. Copy a current key from the website and try again.".into(),
        402 | 403 => "The API key cannot access Codex yet. Check the account balance, subscription, and key group on the website, then try again.".into(),
        429 => "Sub2API is temporarily rate-limiting verification. Wait a moment and try again.".into(),
        500..=599 => "Sub2API is temporarily unavailable. Your Codex configuration was not changed; try again later.".into(),
        code => format!("/v1/models verification failed with HTTP {code}. Check the key and service status, then try again."),
    }
}

struct LiveBackup {
    directory: PathBuf,
    auth: Option<Vec<u8>>,
    config: Option<Vec<u8>>,
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    fs::read(path)
        .map(Some)
        .map_err(|_| "Unable to back up the existing Codex configuration".into())
}

fn create_live_backup() -> Result<LiveBackup, String> {
    let auth_path = crate::codex_config::get_codex_auth_path();
    let config_path = crate::codex_config::get_codex_config_path();
    let auth = read_optional(&auth_path)?;
    let config = read_optional(&config_path)?;
    let directory = crate::codex_config::get_codex_config_dir()
        .join("backups")
        .join(format!(
            "sub2api-{}",
            chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f")
        ));
    fs::create_dir_all(&directory).map_err(|_| "Unable to create the Codex backup directory")?;
    if let Some(bytes) = &auth {
        atomic_write(&directory.join("auth.json"), bytes).map_err(|e| e.to_string())?;
    }
    if let Some(bytes) = &config {
        atomic_write(&directory.join("config.toml"), bytes).map_err(|e| e.to_string())?;
    }
    Ok(LiveBackup {
        directory,
        auth,
        config,
    })
}

fn restore_file(path: &Path, value: &Option<Vec<u8>>) -> Result<(), String> {
    match value {
        Some(bytes) => atomic_write(path, bytes).map_err(|e| e.to_string()),
        None if path.exists() => fs::remove_file(path)
            .map_err(|_| "Unable to restore the original Codex configuration".into()),
        None => Ok(()),
    }
}

fn restore_live_backup(backup: &LiveBackup) -> Result<(), String> {
    restore_file(&crate::codex_config::get_codex_auth_path(), &backup.auth)?;
    restore_file(
        &crate::codex_config::get_codex_config_path(),
        &backup.config,
    )
}

fn provider_for(config: &SetupConfig) -> Provider {
    let quoted_name = toml_edit::Value::from(config.site_name.as_str()).to_string();
    let quoted_model = toml_edit::Value::from(config.model.as_str()).to_string();
    let quoted_base = toml_edit::Value::from(config.api_base_url.as_str()).to_string();
    let quoted_reasoning = toml_edit::Value::from(config.reasoning_effort.as_str()).to_string();
    let config_toml = format!(
        "model_provider = \"sub2api\"\nmodel = {quoted_model}\nmodel_reasoning_effort = {quoted_reasoning}\ndisable_response_storage = true\n\n[model_providers.sub2api]\nname = {quoted_name}\nbase_url = {quoted_base}\nwire_api = \"responses\"\nrequires_openai_auth = true\n"
    );
    let mut meta = ProviderMeta::default();
    meta.api_format = Some("openai_responses".into());
    Provider {
        id: sub2api_brand::PROVIDER_ID.into(),
        name: config.site_name.clone(),
        settings_config: json!({ "auth": { "OPENAI_API_KEY": config.api_key }, "config": config_toml }),
        website_url: Some(sub2api_brand::trusted_site_origin().into()),
        category: Some("aggregator".into()),
        created_at: None,
        sort_index: None,
        notes: Some("Managed by Sub2API Client".into()),
        meta: Some(meta),
        icon: Some("openai".into()),
        icon_color: None,
        in_failover_queue: false,
    }
}

fn rollback_provider_state(
    app_state: &AppState,
    previous_provider: Option<&Provider>,
    previous_current: &str,
) {
    if let Some(previous) = previous_provider {
        let _ = app_state
            .db
            .save_provider(AppType::Codex.as_str(), previous);
    } else {
        let _ = app_state
            .db
            .delete_provider(AppType::Codex.as_str(), sub2api_brand::PROVIDER_ID);
    }
    if previous_current.is_empty() {
        let _ = crate::settings::set_current_provider(&AppType::Codex, None);
    } else {
        let _ = app_state
            .db
            .set_current_provider(AppType::Codex.as_str(), previous_current);
        let _ = crate::settings::set_current_provider(&AppType::Codex, Some(previous_current));
    }
}

#[tauri::command]
pub async fn sub2api_apply_setup(
    app_state: State<'_, AppState>,
    setup_state: State<'_, Sub2APISetupState>,
    client_version: String,
) -> Result<SetupApplyResult, String> {
    let pending = setup_state
        .pending
        .lock()
        .map_err(|_| "Setup state is unavailable")?
        .clone()
        .ok_or("No setup is waiting for confirmation")?;

    // Verify credentials before mutating local files. The authenticated models request
    // also supplies the server-side proof used by the ticket confirmation endpoint.
    verify_models(&pending.config).await?;
    let backup = create_live_backup()?;
    let provider = provider_for(&pending.config);
    let previous_current =
        ProviderService::current(app_state.inner(), AppType::Codex).map_err(|e| e.to_string())?;
    let existing = app_state
        .db
        .get_provider_by_id(sub2api_brand::PROVIDER_ID, AppType::Codex.as_str())
        .map_err(|e| e.to_string())?;
    let mutation = if existing.is_some() {
        ProviderService::update(app_state.inner(), AppType::Codex, None, provider)
    } else {
        ProviderService::add(app_state.inner(), AppType::Codex, provider, false)
    }
    .and_then(|_| {
        ProviderService::switch(
            app_state.inner(),
            AppType::Codex,
            sub2api_brand::PROVIDER_ID,
        )
        .map(|_| true)
    });

    if let Err(error) = mutation {
        let _ = restore_live_backup(&backup);
        rollback_provider_state(app_state.inner(), existing.as_ref(), &previous_current);
        return Err(format!("Codex configuration was not changed: {error}"));
    }

    // Verify again after the provider is active. If anything diverged between the
    // reviewed preview and the live write, restore both files and provider state.
    if let Err(error) = verify_models(&pending.config).await {
        let _ = restore_live_backup(&backup);
        rollback_provider_state(app_state.inner(), existing.as_ref(), &previous_current);
        return Err(format!(
            "Codex verification failed after applying the provider; the original configuration was restored: {error}"
        ));
    }

    let server_confirmed = if let Some(ticket) = &pending.ticket {
        let mut confirmed = false;
        for delay_ms in [0_u64, 250, 750] {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            if post_json::<Value>(
                "/api/v1/client-setup/confirm",
                json!({ "ticket": ticket, "client_version": client_version, "platform": std::env::consts::OS }),
            )
            .await
            .is_ok()
            {
                confirmed = true;
                break;
            }
        }
        confirmed
    } else {
        true
    };
    *setup_state
        .pending
        .lock()
        .map_err(|_| "Setup state is unavailable")? = None;

    Ok(SetupApplyResult {
        configured: true,
        verified: true,
        server_confirmed,
        backup_directory: backup.directory.to_string_lossy().to_string(),
        restart_required: true,
    })
}

#[tauri::command]
pub fn sub2api_setup_status(app_state: State<'_, AppState>) -> Result<bool, String> {
    ProviderService::current(app_state.inner(), AppType::Codex)
        .map(|current| current == sub2api_brand::PROVIDER_ID)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sub2api_take_queued_ticket(setup_state: State<'_, Sub2APISetupState>) -> Option<String> {
    setup_state.queued_ticket.lock().ok()?.take()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn test_config() -> SetupConfig {
        SetupConfig {
            site_name: "Sub2API".into(),
            site_logo: String::new(),
            api_base_url: "https://ai.tavonilo.com".into(),
            api_key: "sk-test-never-log-this".into(),
            model: sub2api_brand::DEFAULT_MODEL.into(),
            reasoning_effort: sub2api_brand::DEFAULT_REASONING_EFFORT.into(),
            wire_api: "responses".into(),
        }
    }

    #[test]
    fn provider_contains_the_expected_responses_configuration() {
        let provider = provider_for(&test_config());
        let config = provider.settings_config["config"].as_str().unwrap();
        assert_eq!(provider.id, "sub2api");
        assert_eq!(
            provider.settings_config["auth"]["OPENAI_API_KEY"],
            "sk-test-never-log-this"
        );
        assert!(config.contains("model = \"gpt-5.6-terra\""));
        assert!(config.contains("model_reasoning_effort = \"high\""));
        assert!(config.contains("wire_api = \"responses\""));
        assert!(config.contains("base_url = \"https://ai.tavonilo.com\""));
    }

    #[test]
    fn model_verification_errors_are_actionable_without_echoing_credentials() {
        let forbidden = models_verification_error(reqwest::StatusCode::FORBIDDEN);
        assert!(forbidden.contains("balance"));
        assert!(forbidden.contains("subscription"));
        assert!(!forbidden.contains("sk-test-never-log-this"));

        let unavailable = models_verification_error(reqwest::StatusCode::BAD_GATEWAY);
        assert!(unavailable.contains("temporarily unavailable"));
        assert!(unavailable.contains("not changed"));
    }

    #[test]
    #[serial]
    fn codex_backup_restores_original_files() {
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", home.path());
        let auth_path = crate::codex_config::get_codex_auth_path();
        let config_path = crate::codex_config::get_codex_config_path();
        fs::create_dir_all(auth_path.parent().unwrap()).unwrap();
        atomic_write(&auth_path, br#"{"OPENAI_API_KEY":"original"}"#).unwrap();
        atomic_write(&config_path, b"model = \"original\"\n").unwrap();

        let backup = create_live_backup().unwrap();
        atomic_write(&auth_path, br#"{"OPENAI_API_KEY":"replacement"}"#).unwrap();
        atomic_write(&config_path, b"model = \"replacement\"\n").unwrap();
        restore_live_backup(&backup).unwrap();

        assert_eq!(
            fs::read(&auth_path).unwrap(),
            br#"{"OPENAI_API_KEY":"original"}"#
        );
        assert_eq!(fs::read(&config_path).unwrap(), b"model = \"original\"\n");
        assert!(backup.directory.join("auth.json").exists());
        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }
}
