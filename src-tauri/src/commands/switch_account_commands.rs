use crate::commands::patch_commands::{detect_windsurf_path_internal, apply_seamless_patch_internal};
use crate::models::{OperationLog, OperationType, OperationStatus};
use crate::repository::DataStore;
use crate::utils::errors::{AppError, AppResult};
use chrono::Utc;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use std::path::PathBuf;

// ==================== 切号进度事件（Tauri Event） ====================
//
// 前端 AccountCard.vue 通过 listen("switch-progress", ...) 订阅此事件并驱动进度弹窗。
// 维护守则：
// - `percent` 必须单调非递减（除非 phase=error）；前端依赖此假设做动画插值。
// - `step` 是稳定的枚举键（前端按 key 标记 checklist），文案放在 `label`。
// - `phase`:
//     "running" = 正在执行该阶段（前端条纹动画）
//     "success" = 全流程成功（percent=100，前端 1 秒后自动关闭）
//     "error"   = 失败（前端保持弹窗、展示 error label + "关闭"按钮）
// - 任何 early return 都必须先 emit phase=error，否则前端会卡在最后一次 running 状态。
#[derive(Clone, Serialize)]
struct SwitchProgressPayload {
    /// 阶段稳定键（与前端 checklist 顺序一一对应）
    step: &'static str,
    /// 人类可读阶段描述（允许带上下文变量）
    label: String,
    /// 0 ~ 100；error 时保留当前阶段的百分比，方便前端定位失败节点
    percent: u8,
    /// "running" | "success" | "error"
    phase: &'static str,
}

/// 封装事件发送；忽略发送错误（窗口已关闭等非致命情况）
fn emit_switch_progress(
    app: &AppHandle,
    step: &'static str,
    label: impl Into<String>,
    percent: u8,
    phase: &'static str,
) {
    let payload = SwitchProgressPayload {
        step,
        label: label.into(),
        percent,
        phase,
    };
    if let Err(e) = app.emit("switch-progress", &payload) {
        warn!("Failed to emit switch-progress event: {:?}", e);
    }
}

#[cfg(target_os = "windows")]
use winreg::{RegKey, enums::{HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS}};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    expires_in: String,
    token_type: String,
    refresh_token: String,
    id_token: String,
    user_id: String,
    project_id: String,
}

/// 使用refresh_token获取新的access_token
async fn refresh_access_token(refresh_token: &str) -> AppResult<GoogleTokenResponse> {
    // 使用专门用于 googleapis 的 HTTP 客户端（支持代理）
    let client = crate::services::get_google_api_client();
    
    // Google Token API
    let url = "https://securetoken.googleapis.com/v1/token";
    
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    
    let response = client
        .post(&format!("{}?key={}", url, crate::services::auth_service::FIREBASE_API_KEY))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("X-Client-Version", "Chrome/JsCore/11.0.0/FirebaseCore-web")
        .header("Origin", "https://windsurf.com")
        .header("Referer", "https://windsurf.com/")
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        error!("Failed to refresh token: {}", error_text);
        return Err(AppError::ApiRequest(format!("Failed to refresh token: {}", error_text)));
    }
    
    let token_response = response.json::<GoogleTokenResponse>().await
        .map_err(|e| AppError::Network(e.to_string()))?;
    
    Ok(token_response)
}

/// 序列化Protobuf字符串（field 1, wire type 2）
fn serialize_protobuf_string(value: &str) -> Vec<u8> {
    if value.is_empty() {
        return vec![];
    }
    
    let value_bytes = value.as_bytes();
    let value_length = value_bytes.len();
    
    // Field 1, wire type 2 (length-delimited): (1 << 3) | 2 = 0x0A
    let mut result = vec![0x0A];
    
    // Encode length as varint
    let mut length = value_length;
    while length > 127 {
        result.push((length as u8 & 0x7F) | 0x80);
        length >>= 7;
    }
    result.push(length as u8 & 0x7F);
    
    // Append value bytes
    result.extend_from_slice(value_bytes);
    result
}

/// 反序列化Protobuf响应获取auth_token
fn deserialize_protobuf_response(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    
    let mut pos = 0;
    while pos < data.len() {
        // Read field tag
        let tag = data[pos];
        pos += 1;
        
        // Get wire type (low 3 bits)
        let wire_type = tag & 0x07;
        let field_number = tag >> 3;
        
        // If it's length-delimited type (wire_type = 2)
        if wire_type == 2 {
            // Read varint length
            let mut length = 0;
            let mut shift = 0;
            while pos < data.len() {
                let byte = data[pos];
                pos += 1;
                length |= ((byte & 0x7F) as usize) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            
            // Read string content
            if pos + length <= data.len() {
                if let Ok(value) = std::str::from_utf8(&data[pos..pos + length]) {
                    // auth_token is typically field 1
                    if field_number == 1 && !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
                pos += length;
            }
        } else if wire_type == 0 {
            // Skip varint field
            while pos < data.len() {
                if data[pos] & 0x80 == 0 {
                    pos += 1;
                    break;
                }
                pos += 1;
            }
        } else {
            // Skip other types
            break;
        }
    }
    
    None
}

/// 使用access_token获取auth_token
async fn get_auth_token(access_token: &str) -> AppResult<String> {
    let client = reqwest::Client::new();
    
    // Windsurf GetOneTimeAuthToken endpoint
    let url = "https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetOneTimeAuthToken";
    
    // Serialize request as Protobuf
    let request_data = serialize_protobuf_string(access_token);
    
    let response = client
        .post(url)
        .header("Content-Type", "application/proto")
        .header("Accept", "application/proto")
        .header("User-Agent", "Windsurf/1.4.2")
        .body(request_data)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        error!("Failed to get auth token: {}", error_text);
        return Err(AppError::ApiRequest(format!("Failed to get auth token: {}", error_text)));
    }
    
    // Deserialize response
    let response_bytes = response.bytes().await
        .map_err(|e| AppError::Network(e.to_string()))?;
    
    let auth_token = deserialize_protobuf_response(&response_bytes)
        .ok_or_else(|| AppError::ApiRequest("Failed to parse auth token from response".to_string()))?;
    
    info!("Successfully obtained auth token");
    Ok(auth_token)
}

/// 根据客户端类型获取 protocol URI scheme 和数据目录名
fn get_client_uri_config(client_type: &str) -> (&'static str, &'static str) {
    match client_type {
        "windsurf-next" => ("windsurf-next", "Windsurf - Next"),
        _ => ("windsurf", "Windsurf"),
    }
}

/// 触发Windsurf回调URL以完成登录
async fn trigger_windsurf_callback(auth_token: &str, client_type: &str) -> AppResult<()> {
    let (scheme, _) = get_client_uri_config(client_type);
    
    // 生成state参数
    let state = Uuid::new_v4().to_string();
    
    // 构建URL
    // {scheme}://codeium.windsurf#access_token=<auth_token>&state=<state>&token_type=Bearer
    let params = [
        ("access_token", auth_token),
        ("state", &state),
        ("token_type", "Bearer"),
    ];
    
    let fragment = serde_urlencoded::to_string(&params)
        .map_err(|e| AppError::ApiRequest(format!("Failed to encode URL parameters: {}", e)))?;
    
    let callback_url = format!("{}://codeium.windsurf#{}", scheme, fragment);
    
    info!("Triggering Windsurf callback: {}", callback_url);
    
    // 使用系统默认程序打开URL（触发Windsurf处理）
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // 使用 PowerShell 的 Start-Process 来正确处理包含特殊字符的 URL
        Command::new("powershell")
            .args(&["-NoProfile", "-Command", &format!("Start-Process '{}'", callback_url)])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| AppError::FileOperation(format!("Failed to open URL: {}", e)))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg(&callback_url)
            .spawn()
            .map_err(|e| AppError::FileOperation(format!("Failed to open URL: {}", e)))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xdg-open")
            .arg(&callback_url)
            .spawn()
            .map_err(|e| AppError::FileOperation(format!("Failed to open URL: {}", e)))?;
    }
    
    info!("Successfully triggered Windsurf callback");
    Ok(())
}


/// 一键切换账号命令（简化版：使用回调URL登录）
///
/// 进度事件：整个流程通过 `emit_switch_progress` 向前端持续上报 `switch-progress` 事件
/// （见本文件头部的 SwitchProgressPayload 文档）。前端 AccountCard.vue 通过 listen 订阅。
#[tauri::command]
pub async fn switch_account(
    app: AppHandle,
    id: String,
    data_store: State<'_, Arc<DataStore>>,
) -> Result<Value, String> {
    perform_account_switch(app, data_store.inner(), id).await
}

/// 切号核心逻辑（Fork 新增抽取）
///
/// 与 `switch_account` Tauri 命令共享同一份实现：
/// - 自动切换（`check_and_auto_switch`）需要在不暴露 `State` 的情况下复用切号流程；
/// - 直接抽到独立函数，避免 200+ 行核心逻辑被复制粘贴。
///
/// 注意：本函数仍会向前端 emit `switch-progress` 事件，前端 toast/弹窗可以选择监听。
pub(crate) async fn perform_account_switch(
    app: AppHandle,
    data_store: &Arc<DataStore>,
    id: String,
) -> Result<Value, String> {
    info!("Switching account: {}", id);
    emit_switch_progress(&app, "preparing", "开始切换账号...", 5, "running");
    
    let account_id = Uuid::parse_str(&id).map_err(|e| {
        emit_switch_progress(&app, "preparing", format!("账号ID无效: {}", e), 5, "error");
        e.to_string()
    })?;
    
    // 获取账号信息
    let account = data_store
        .get_account(account_id)
        .await
        .map_err(|e| {
            emit_switch_progress(&app, "preparing", format!("读取账号失败: {}", e), 5, "error");
            e.to_string()
        })?;
    
    // Step 1~2: 根据账号体系分流获取 access_token / auth_token
    //
    // - Firebase 账号：refresh_token → Google access_token → GetOneTimeAuthToken → one-time auth_token
    // - Devin 账号：account.token (devin-session-token$...) 直接作为 GetOneTimeAuthToken 的 auth_token 入参；
    //   由 AuthContext 自动附带 4 个 Devin 扩展 header 完成鉴权，无 Google OAuth 环节
    let (access_token, expires_in, auth_token) = if account.is_devin_account() {
        use crate::services::{AuthContext, WindsurfService};
        info!("[Devin] Using session-token based one-time auth token flow");
        emit_switch_progress(&app, "fetch_access", "使用 Devin session token 认证...", 15, "running");

        let ctx = AuthContext::from_account(&account).map_err(|e| {
            emit_switch_progress(&app, "fetch_access", format!("Devin 认证上下文构建失败: {}", e), 15, "error");
            e.to_string()
        })?;
        let windsurf = WindsurfService::new();
        emit_switch_progress(&app, "fetch_auth", "正在获取 one-time auth_token...", 35, "running");
        let auth_token = match windsurf.get_one_time_auth_token(&ctx).await {
            Ok(token) => {
                info!("[Devin] Successfully obtained one-time auth token");
                token
            }
            Err(e) => {
                error!("[Devin] Failed to get one-time auth token: {:?}", e);
                emit_switch_progress(&app, "fetch_auth", format!("获取 auth_token 失败: {}", e), 35, "error");
                return Ok(json!({
                    "success": false,
                    "error": format!("获取auth_token失败: {}", e)
                }));
            }
        };

        // Devin session_token 对外层 update_account_token 仅作占位写入（值不变），
        // expires_in 取 account 现有远期伪值，缺失则默认 30 天
        let access_token = account.token.clone().unwrap_or_default();
        let expires_in = account
            .token_expires_at
            .map(|t| (t - Utc::now()).num_seconds().max(0).to_string())
            .unwrap_or_else(|| "2592000".to_string());
        (access_token, expires_in, auth_token)
    } else {
        // Firebase 分支：必须有 refresh_token 才能换 Google access_token
        emit_switch_progress(&app, "fetch_access", "正在准备 access_token...", 15, "running");
        if account.refresh_token.is_none() || account.refresh_token.as_ref().unwrap().is_empty() {
            emit_switch_progress(&app, "fetch_access", "账号没有 refresh_token，请先登录", 15, "error");
            return Ok(json!({
                "success": false,
                "error": "账号没有refresh_token，请先登录"
            }));
        }

        let refresh_token = account.refresh_token.clone().unwrap();

        // Step 1: 检查本地token是否有效
        let (access_token, expires_in) = if let (Some(token), Some(expires_at)) = (&account.token, &account.token_expires_at) {
            // 检查token是否还有至少5分钟有效期
            let now = Utc::now();
            let buffer = chrono::Duration::minutes(5);
            if *expires_at > now + buffer {
                info!("Using cached access token, expires at: {}", expires_at);
                let remaining_seconds = (*expires_at - now).num_seconds();
                (token.clone(), remaining_seconds.to_string())
            } else {
                info!("Token expired or expiring soon, refreshing...");
                let token_response = match refresh_access_token(&refresh_token).await {
                    Ok(resp) => resp,
                    Err(e) => {
                        error!("Failed to refresh access token: {:?}", e);
                        emit_switch_progress(&app, "fetch_access", format!("刷新 access_token 失败: {}", e), 15, "error");
                        return Ok(json!({
                            "success": false,
                            "error": format!("获取access_token失败: {}", e)
                        }));
                    }
                };
                (token_response.access_token, token_response.expires_in)
            }
        } else {
            // 没有本地token，需要刷新
            info!("No cached token, refreshing access token...");
            let token_response = match refresh_access_token(&refresh_token).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to refresh access token: {:?}", e);
                    emit_switch_progress(&app, "fetch_access", format!("刷新 access_token 失败: {}", e), 15, "error");
                    return Ok(json!({
                        "success": false,
                        "error": format!("获取access_token失败: {}", e)
                    }));
                }
            };
            (token_response.access_token, token_response.expires_in)
        };

        // Step 2: 获取auth_token
        info!("Getting auth token...");
        emit_switch_progress(&app, "fetch_auth", "正在获取 one-time auth_token...", 35, "running");
        let auth_token = match get_auth_token(&access_token).await {
            Ok(token) => token,
            Err(e) => {
                error!("Failed to get auth token: {:?}", e);
                emit_switch_progress(&app, "fetch_auth", format!("获取 auth_token 失败: {}", e), 35, "error");
                return Ok(json!({
                    "success": false,
                    "error": format!("获取auth_token失败: {}", e)
                }));
            }
        };

        (access_token, expires_in, auth_token)
    };
    
    // 读取设置：客户端类型 + 无感换号状态
    let settings = data_store.get_settings().await.map_err(|e| e.to_string())?;
    let client_type = settings.windsurf_client_type.clone();
    let mut seamless_patch_active = settings.seamless_switch_enabled;
    let mut auto_enabled_seamless = false;
    
    // 如果无感换号未启用，尝试自动启用
    if !seamless_patch_active {
        info!("Seamless switch not enabled, attempting auto-enable...");
        emit_switch_progress(&app, "auto_patch", "尝试自动启用无感换号补丁...", 55, "running");
    } else {
        // 已启用时也 emit 一次，让前端 checklist 的该步骤显示为"已启用（跳过）"
        emit_switch_progress(&app, "auto_patch", "无感换号已启用，跳过补丁应用", 55, "running");
    }
    if !seamless_patch_active {
        
        // Step A: 检测或使用已有的客户端路径
        let windsurf_path = settings.windsurf_path.as_ref()
            .filter(|p| !p.is_empty())
            .cloned()
            .or_else(|| {
                info!("No windsurf path configured, auto-detecting...");
                match detect_windsurf_path_internal(&client_type) {
                    Ok(path) => {
                        info!("Auto-detected windsurf path: {}", path);
                        Some(path)
                    }
                    Err(e) => {
                        warn!("Failed to auto-detect windsurf path: {}", e);
                        None
                    }
                }
            });
        
        // Step B: 如果有路径，自动应用无感换号补丁
        if let Some(ref path) = windsurf_path {
            info!("Auto-applying seamless patch at: {}", path);
            match apply_seamless_patch_internal(path, &data_store).await {
                Ok(result) => {
                    let already_patched = result.get("already_patched")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let success = result.get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if success {
                        seamless_patch_active = true;
                        auto_enabled_seamless = true;
                        if already_patched {
                            info!("Seamless patch was already applied");
                        } else {
                            info!("Seamless patch auto-applied successfully");
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to auto-apply seamless patch: {}", e);
                }
            }
        }
    }
    
    // Step 3: 尝试重置机器ID（可能需要管理员权限）
    info!("Attempting to reset machine ID...");
    emit_switch_progress(&app, "reset_mid", "重置机器 ID...", 70, "running");
    let reset_result = reset_machine_id_internal(&client_type).await;
    let machine_id_reset = match reset_result {
        Ok(_) => {
            info!("Machine ID reset successful");
            true
        },
        Err(e) => {
            warn!("Failed to reset machine ID: {:?}", e);
            warn!("重置机器ID失败，可能需要管理员权限。但切换账号仍可继续。");
            false
        }
    };
    
    // Step 4: 触发客户端回调URL以自动登录
    info!("Triggering {} callback...", client_type);
    emit_switch_progress(&app, "callback", format!("触发 {} 登录...", client_type), 85, "running");
    if let Err(e) = trigger_windsurf_callback(&auth_token, &client_type).await {
        error!("Failed to trigger callback: {:?}", e);
        emit_switch_progress(&app, "callback", format!("触发登录失败: {}", e), 85, "error");
        return Ok(json!({
            "success": false,
            "error": format!("触发Windsurf登录失败: {}", e)
        }));
    }
    
    // 更新账号的token信息
    emit_switch_progress(&app, "finalize", "保存账号状态...", 95, "running");
    let expires_at = Utc::now() + chrono::Duration::seconds(expires_in.parse::<i64>().unwrap_or(3600));
    if let Err(e) = data_store.update_account_token(
        account_id,
        access_token.clone(),
        expires_at
    ).await {
        error!("Failed to update account token: {:?}", e);
    }
    
    info!("Successfully triggered Windsurf login for account");
    
    // 记录切换账号日志
    let log = OperationLog::new(
        OperationType::SwitchAccount,
        OperationStatus::Success,
        format!("切换账号: {}", account.email),
    ).with_account(account_id, account.email);
    let _ = data_store.add_log(log).await;
    
    let (_, client_display) = get_client_uri_config(&client_type);
    
    let message = if auto_enabled_seamless {
        if machine_id_reset {
            format!("已自动启用无感换号并切换账号，{}已重启", client_display)
        } else {
            format!("已自动启用无感换号并切换账号，{}已重启（机器ID未重置）", client_display)
        }
    } else if seamless_patch_active {
        if machine_id_reset {
            format!("已通过无感换号切换账号并重置机器ID，{}无需重启", client_display)
        } else {
            format!("已通过无感换号切换账号，{}无需重启（机器ID未重置）", client_display)
        }
    } else if machine_id_reset {
        format!("已触发{}登录并重置机器ID", client_display)
    } else {
        format!("已触发{}登录（未重置机器ID，可能需要管理员权限）", client_display)
    };
    
    emit_switch_progress(&app, "done", "切换完成", 100, "success");

    Ok(json!({
        "success": true,
        "message": message,
        "auth_token": auth_token,
        "machine_id_reset": machine_id_reset,
        "seamless_patch_active": seamless_patch_active,
        "auto_enabled_seamless": auto_enabled_seamless
    }))
}

/// 内部重置机器ID函数
async fn reset_machine_id_internal(client_type: &str) -> AppResult<()> {
    use std::fs;
    use rand::Rng;
    
    // 生成新的机器ID（符合VSCode格式）
    let mut rng = rand::thread_rng();
    
    // machineId: 64位hex字符串（256位）
    let machine_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let new_machine_id = hex::encode(&machine_bytes);
    
    // macMachineId: 32位hex字符串（MD5格式）
    let new_mac_machine_id = format!("{:032x}", rng.gen::<u128>());
    
    // sqmId: UUID格式，不带括号
    let new_sqm_id = Uuid::new_v4().to_string().to_uppercase();
    
    // devDeviceId: 标准UUID格式
    let new_device_id = Uuid::new_v4().to_string().to_lowercase();
    
    // 更新storage.json
    let (_, data_dir_name) = get_client_uri_config(client_type);
    let mut storage_path = directories::BaseDirs::new()
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("C:/Users/Default/AppData/Roaming"));
    storage_path.push(data_dir_name);
    storage_path.push("User");
    storage_path.push("globalStorage");
    storage_path.push("storage.json");
    
    if storage_path.exists() {
        let content = fs::read_to_string(&storage_path)
            .map_err(|e| AppError::FileOperation(format!("Failed to read storage.json: {}", e)))?;
        let mut storage: Value = serde_json::from_str(&content)
            .map_err(AppError::Serialization)?;
        
        storage["telemetry.machineId"] = json!(new_machine_id);
        storage["telemetry.macMachineId"] = json!(new_mac_machine_id);
        storage["telemetry.sqmId"] = json!(new_sqm_id);
        storage["telemetry.devDeviceId"] = json!(new_device_id);
        
        let updated = serde_json::to_string_pretty(&storage)
            .map_err(AppError::Serialization)?;
        fs::write(&storage_path, updated)
            .map_err(|e| AppError::FileOperation(format!("Failed to write storage.json: {}. 可能需要管理员权限", e)))?;
        
        info!("Updated storage.json with new machine IDs");
    } else {
        warn!("storage.json not found at {:?}", storage_path);
    }
    
    // Windows特定：更新注册表（程序启动时已要求管理员权限）
    #[cfg(target_os = "windows")]
    {
        // 只更新 HKEY_LOCAL_MACHINE 下的 Cryptography MachineGuid（需要管理员权限）
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        
        // 生成新的GUID（不带大括号的格式）
        let new_machine_guid = Uuid::new_v4().to_string().to_uppercase();
        
        match hklm.open_subkey_with_flags(
            "SOFTWARE\\Microsoft\\Cryptography",
            KEY_ALL_ACCESS
        ) {
            Ok(crypto_key) => {
                match crypto_key.set_value("MachineGuid", &new_machine_guid) {
                    Ok(()) => {
                        info!("Updated HKLM\\SOFTWARE\\Microsoft\\Cryptography\\MachineGuid to: {}", new_machine_guid);
                        Ok(())
                    }
                    Err(e) => {
                        let msg = format!("Failed to update MachineGuid: {}. 确保以管理员权限运行", e);
                        error!("{}", msg);
                        Err(AppError::FileOperation(msg))
                    }
                }
            }
            Err(e) => {
                let msg = format!("Failed to open HKLM\\SOFTWARE\\Microsoft\\Cryptography: {}. 需要管理员权限", e);
                error!("{}", msg);
                Err(AppError::FileOperation(msg))
            }
        }
    }
    
    // macOS特定：尝试重置系统级机器标识
    #[cfg(target_os = "macos")]
    {
        // macOS 的硬件 UUID 无法修改，但可以尝试重置一些软件级别的标识
        // 注意：某些操作可能需要 sudo 权限
        
        // 尝试删除客户端的本地缓存标识文件
        let home = std::env::var("HOME").unwrap_or_default();
        let cache_paths = vec![
            format!("{}/.config/{}/machineid", home, data_dir_name),
            format!("{}/Library/Application Support/{}/.installerId", home, data_dir_name),
        ];
        
        for cache_path in cache_paths {
            let path = PathBuf::from(&cache_path);
            if path.exists() {
                match fs::remove_file(&path) {
                    Ok(()) => info!("Removed cache file: {}", cache_path),
                    Err(e) => warn!("Failed to remove {}: {}", cache_path, e),
                }
            }
        }
        
        // 尝试重置系统级 machine-id（需要 sudo 权限）
        // /var/lib/dbus/machine-id 在 macOS 上通常不存在
        // 但某些应用可能会读取 IOPlatformUUID
        
        info!("macOS machine ID reset completed (software level only)");
        Ok(())
    }
    
    // Linux特定：尝试重置 /etc/machine-id 和 /var/lib/dbus/machine-id
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        
        // 生成新的 machine-id（32位hex字符串）
        let new_linux_machine_id = format!("{:032x}", rand::thread_rng().gen::<u128>());
        
        // 尝试更新 /etc/machine-id（需要 root 权限）
        let etc_machine_id = PathBuf::from("/etc/machine-id");
        if etc_machine_id.exists() {
            match fs::write(&etc_machine_id, format!("{}\n", new_linux_machine_id)) {
                Ok(()) => {
                    info!("Updated /etc/machine-id to: {}", new_linux_machine_id);
                }
                Err(e) => {
                    warn!("Failed to update /etc/machine-id: {}. 需要 sudo 权限", e);
                    // 尝试使用 sudo
                    let result = Command::new("sudo")
                        .args(["bash", "-c", &format!("echo '{}' > /etc/machine-id", new_linux_machine_id)])
                        .output();
                    match result {
                        Ok(output) if output.status.success() => {
                            info!("Updated /etc/machine-id via sudo");
                        }
                        _ => {
                            warn!("Could not update /etc/machine-id even with sudo");
                        }
                    }
                }
            }
        }
        
        // 尝试更新 /var/lib/dbus/machine-id（通常是 /etc/machine-id 的符号链接）
        let dbus_machine_id = PathBuf::from("/var/lib/dbus/machine-id");
        if dbus_machine_id.exists() && !dbus_machine_id.is_symlink() {
            match fs::write(&dbus_machine_id, format!("{}\n", new_linux_machine_id)) {
                Ok(()) => {
                    info!("Updated /var/lib/dbus/machine-id");
                }
                Err(e) => {
                    warn!("Failed to update /var/lib/dbus/machine-id: {}", e);
                }
            }
        }
        
        // 尝试删除客户端的本地缓存标识文件
        let home = std::env::var("HOME").unwrap_or_default();
        let cache_paths = vec![
            format!("{}/.config/{}/machineid", home, data_dir_name),
            format!("{}/.local/share/{}/.installerId", home, data_dir_name),
        ];
        
        for cache_path in cache_paths {
            let path = PathBuf::from(&cache_path);
            if path.exists() {
                match fs::remove_file(&path) {
                    Ok(()) => info!("Removed cache file: {}", cache_path),
                    Err(e) => warn!("Failed to remove {}: {}", cache_path, e),
                }
            }
        }
        
        info!("Linux machine ID reset completed");
        Ok(())
    }
}

/// 重置机器ID命令（供前端调用）
#[tauri::command]
pub async fn reset_machine_id(
    data_store: State<'_, Arc<DataStore>>,
) -> Result<Value, String> {
    let client_type = match data_store.get_settings().await {
        Ok(s) => s.windsurf_client_type,
        Err(_) => "windsurf".to_string(),
    };
    match reset_machine_id_internal(&client_type).await {
        Ok(()) => Ok(json!({
            "success": true,
            "message": "机器ID重置成功"
        })),
        Err(e) => Ok(json!({
            "success": false,
            "message": format!("机器ID重置失败: {}", e)
        }))
    }
}

#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use std::ptr;
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, HANDLE, TOKEN_ELEVATION, TOKEN_QUERY};
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::handleapi::CloseHandle;
    
    unsafe {
        let mut token_handle: HANDLE = ptr::null_mut();
        
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token_handle
        ) == 0 {
            return false;
        }
        
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        
        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size
        );
        
        CloseHandle(token_handle);
        
        result != 0 && elevation.TokenIsElevated != 0
    }
}

/// 检查应用程序是否以管理员/root权限运行
#[tauri::command]
pub async fn check_admin_privileges() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(is_elevated())
    }
    
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // Unix系统：检查 euid 是否为 0 (root)
        Ok(is_root())
    }
}

/// 检查是否以 root 权限运行 (Unix)
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}
