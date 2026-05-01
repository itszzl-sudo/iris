//! 守护进程 API 集成测试
//!
//! 需要守护进程在端口 19999 上运行。
//! 运行方式: cargo test -p iris-jetcrab-daemon --test api_integration_test -- --nocapture --ignored

use std::time::Duration;

async fn api_get(path: &str) -> Result<serde_json::Value, String> {
    let url = format!("http://127.0.0.1:19999{}", path);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("client build: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("GET {} failed: {}", url, e))?;
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("JSON parse: {}", e))
}

async fn api_put(path: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = format!("http://127.0.0.1:19999{}", path);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("client build: {}", e))?;
    let resp = client
        .put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("PUT {} failed: {}", url, e))?;
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("JSON parse: {}", e))
}

async fn get_html(path: &str) -> Result<String, String> {
    let url = format!("http://127.0.0.1:19999{}", path);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("client build: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("GET {} failed: {}", url, e))?;
    resp.text()
        .await
        .map_err(|e| format!("text: {}", e))
}

#[tokio::test]
#[ignore]
async fn test_api_status() {
    let json = api_get("/api/status").await.expect("GET /api/status");
    assert_eq!(json["status"], "running", "daemon should be running");
    assert!(json["server_running"].is_boolean());
}

#[tokio::test]
#[ignore]
async fn test_api_get_config() {
    let json = api_get("/api/config").await.expect("GET /api/config");
    assert!(json["http_port"].is_number());
    assert!(json["daemon_port"].is_number());
    assert!(json["show_icon"].is_boolean());
    assert!(json["ai_provider"].is_string());
}

#[tokio::test]
#[ignore]
async fn test_api_get_ai_config() {
    let json = api_get("/api/ai/config").await.expect("GET /api/ai/config");
    assert!(json["ai_provider"].is_string());
    assert!(json["ai_api_key"].is_string());
    assert!(json["ai_model"].is_string());
    assert!(json["ai_endpoint"].is_string());
}

#[tokio::test]
#[ignore]
async fn test_api_update_ai_config() {
    // 读取当前配置
    let before = api_get("/api/ai/config").await.expect("GET before update");
    let orig_provider = before["ai_provider"].as_str().unwrap_or("openai").to_string();
    let orig_model = before["ai_model"].as_str().unwrap_or("").to_string();

    // 更新为 deepseek
    let update = serde_json::json!({
        "ai_provider": "deepseek",
        "ai_api_key": "test-integration-key",
        "ai_model": "deepseek-chat",
        "ai_endpoint": "https://api.deepseek.com/v1"
    });
    let result = api_put("/api/ai/config", update).await.expect("PUT /api/ai/config");
    assert_eq!(result["status"], "ok", "update should succeed");

    // 验证已保存
    let after = api_get("/api/ai/config").await.expect("GET after update");
    assert_eq!(after["ai_provider"], "deepseek");
    assert_eq!(after["ai_model"], "deepseek-chat");

    // 恢复原始配置
    let restore = serde_json::json!({
        "ai_provider": orig_provider,
        "ai_model": orig_model,
        "ai_api_key": "",
        "ai_endpoint": "https://api.openai.com/v1"
    });
    api_put("/api/ai/config", restore).await.expect("restore config");
}

#[tokio::test]
#[ignore]
async fn test_api_connected_clients_empty() {
    let json = api_get("/api/connected-clients").await.expect("GET /api/connected-clients");
    assert_eq!(json["count"], 0, "should have no connected clients");
    assert!(json["clients"].is_array());
}

#[tokio::test]
#[ignore]
async fn test_management_page_accessible() {
    let html = get_html("/").await.expect("GET /");
    assert!(html.contains("Iris JetCrab 管理面板"),
        "management page should contain title");
    assert!(html.contains("cfgAiProvider"),
        "management page should contain AI provider dropdown");
}

#[tokio::test]
#[ignore]
async fn test_confirm_open_page() {
    let html = get_html("/open").await.expect("GET /open");
    assert!(html.contains("确认打开"), "confirm page should contain 确认打开");
}

#[tokio::test]
#[ignore]
async fn test_deepseek_option_in_html() {
    let html = get_html("/").await.expect("GET /");
    assert!(html.contains(r#"value="deepseek""#),
        "dropdown should have deepseek option");
    assert!(html.contains("DeepSeek"),
        "dropdown should display DeepSeek label");
}
