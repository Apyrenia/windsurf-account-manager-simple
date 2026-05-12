use crate::commands::devin_commands::{enrich_account_with_user_info, refresh_devin_session_in_memory};
use crate::models::{Account, OperationLog, OperationStatus, OperationType};
use crate::repository::DataStore;
use crate::services::{AuthContext, AuthService, UpdateSeatsResult, WindsurfService};
use crate::utils::AppError;
use serde_json::json;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

/// Devin session_token 统一使用的“远期” expires_at（32 天后）
///
/// Devin 体系的 session_token 没有显式过期时间，但现有 token 缓存逻辑（`is_token_expired`）
/// 依赖 `token_expires_at` 字段。此处只设置一个足够远的时间避免被错误认定为过期；真正
/// 的过期判断依靠 401 错误触发 `force_refresh`。
///
/// 暴露给 `devin_commands` 的建账路径（注册 / 邮件登录 / 账密登录 / session_token 迁入 /
/// 多组织二次完成 / refresh_devin_session）统一使用，保证账号卡「到期时间」字段在初建
/// 时就已填充，与刷新路径行为一致。
pub(crate) fn devin_session_pseudo_expires_at() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() + chrono::Duration::days(32)
}

/// 确保账户有有效的Token
/// 优先使用缓存的token，只在过期或不存在时刷新
pub async fn ensure_valid_token(
    store: &Arc<DataStore>,
    account: &mut Account,
    uuid: Uuid,
) -> Result<(), String> {
    ensure_valid_token_with_force(store, account, uuid, false).await
}

/// 检查账号是否为团队所有者（Admin角色）
/// 通过 GetCurrentUser API 获取 roles 字段判断是否为 root.admin
pub async fn check_is_team_owner(windsurf_service: &WindsurfService, ctx: &AuthContext, _email: &str) -> bool {
    if let Ok(user_result) = windsurf_service.get_current_user(ctx).await {
        // 检查 user_info.is_root_admin 字段（由 proto_parser 解析）
        if let Some(user_info) = user_result.get("user_info") {
            if let Some(is_root_admin) = user_info.get("is_root_admin").and_then(|v| v.as_bool()) {
                return is_root_admin;
            }
        }
    }
    false
}

/// 检查 API 响应是否为 401 错误
pub fn is_401_error(result: &serde_json::Value) -> bool {
    result.get("status_code")
        .and_then(|v| v.as_u64())
        .map(|code| code == 401)
        .unwrap_or(false)
}

/// 将 GetPlanStatus 返回的 plan_status JSON 应用到 Account 结构体
pub fn apply_plan_status_to_account(plan_status: &serde_json::Value, account: &mut crate::models::account::Account) {
    // 更新套餐名称
    if let Some(plan_name) = plan_status.get("plan_name").and_then(|v| v.as_str()) {
        account.plan_name = Some(plan_name.to_string());
    }
    
    // 更新已用配额 (used_prompt_credits + used_flex_credits)
    let used_prompt = plan_status.get("used_prompt_credits").and_then(|v| v.as_i64()).unwrap_or(0);
    let used_flex = plan_status.get("used_flex_credits").and_then(|v| v.as_i64()).unwrap_or(0);
    account.used_quota = Some((used_prompt + used_flex) as i32);
    
    // 更新总配额 (available_flex_credits + available_prompt_credits)
    let available_flex = plan_status.get("available_flex_credits").and_then(|v| v.as_i64()).unwrap_or(0);
    let available_prompt = plan_status.get("available_prompt_credits").and_then(|v| v.as_i64()).unwrap_or(0);
    if available_flex > 0 || available_prompt > 0 {
        account.total_quota = Some((available_flex + available_prompt) as i32);
    }
    
    // 更新订阅到期时间 (plan_end)
    if let Some(plan_end) = plan_status.get("plan_end").and_then(|v| v.as_i64()) {
        account.subscription_expires_at = chrono::DateTime::from_timestamp(plan_end, 0);
    }
    
    // 更新计费策略
    if let Some(bs) = plan_status.get("billing_strategy").and_then(|v| v.as_i64()) {
        account.billing_strategy = Some(bs as i32);
    }
    
    // 更新日/周配额百分比
    //
    // ⚠️ 关键修复（fork v1.7.11）：响应**缺字段**时写 0，而不是保留旧值。
    //
    // 原因：Windsurf API 在某些场景下（如 Devin Trial 账号日配额耗尽后）会
    //       **省略** `int_14` 字段，chaogei 原逻辑 "字段存在才更新" 会保留
    //       数据库里的历史值（常为 100%），导致 UI 永远显示 "日配额 100%" 而
    //       实际已经用完。
    //
    // 修复：字段缺失 → 视同 0%（耗尽）。对 CREDITS 模式账号无影响（UI 只在
    //       billing_strategy==QUOTA 时才显示日/周配额行，`isQuotaMode` 判断）。
    account.daily_quota_remaining_percent = Some(
        plan_status.get("daily_quota_remaining_percent").and_then(|v| v.as_i64()).unwrap_or(0) as i32
    );
    account.weekly_quota_remaining_percent = Some(
        plan_status.get("weekly_quota_remaining_percent").and_then(|v| v.as_i64()).unwrap_or(0) as i32
    );
    if let Some(v) = plan_status.get("daily_quota_reset_at_unix").and_then(|v| v.as_i64()) {
        account.daily_quota_reset_at_unix = Some(v);
    }
    if let Some(v) = plan_status.get("weekly_quota_reset_at_unix").and_then(|v| v.as_i64()) {
        account.weekly_quota_reset_at_unix = Some(v);
    }
    if let Some(v) = plan_status.get("overage_balance_micros").and_then(|v| v.as_i64()) {
        account.overage_balance_micros = Some(v);
    }
    
    account.last_quota_update = Some(chrono::Utc::now());
}

/// 确保账户有有效的Token（支持强制刷新）
/// force_refresh: 强制刷新token，用于处理服务器端使token失效的情况（如401错误）
pub async fn ensure_valid_token_with_force(
    store: &Arc<DataStore>,
    account: &mut Account,
    uuid: Uuid,
    force_refresh: bool,
) -> Result<(), String> {
    // 如果不是强制刷新且token有效，直接返回
    if !force_refresh && 
       account.token.is_some() && 
       account.token_expires_at.is_some() && 
       !AuthService::is_token_expired(&account.token_expires_at.unwrap()) {
        return Ok(());
    }
    
    if force_refresh {
        println!("[ensure_valid_token] 强制刷新 token (可能是 401 错误触发)");
    }

    // ==================== Devin 账号分支 ====================
    // Devin 账号使用 devin_auth1_token 重新换取 session_token，而非 Firebase refresh_token
    if account.is_devin_account() {
        let new_token = refresh_devin_session_in_memory(account).await?;
        let pseudo_expires = devin_session_pseudo_expires_at();
        account.token_expires_at = Some(pseudo_expires);
        // 落库：使用 update_account 以同步更新后的 devin_auth1_token / devin_account_id / devin_primary_org_id
        store
            .update_account(account.clone())
            .await
            .map_err(|e| e.to_string())?;
        let _ = new_token; // 仅为打消未使用警告
        return Ok(());
    }

    // ==================== Firebase 分支（原有逻辑） ====================
    let auth_service = AuthService::new();
    
    // 优先尝试使用refresh token
    let (token, refresh_token_new, expires_at) = if let Some(refresh_token) = &account.refresh_token {
        match auth_service.refresh_token(refresh_token).await {
            Ok(result) => result,
            Err(_) => {
                // refresh token失败，重新登录
                let password = store.get_decrypted_password(uuid)
                    .await
                    .map_err(|e| e.to_string())?;
                auth_service.sign_in(&account.email, &password)
                    .await
                    .map_err(|e| e.to_string())?
            }
        }
    } else {
        // 没有refresh token，直接重新登录
        let password = store.get_decrypted_password(uuid)
            .await
            .map_err(|e| e.to_string())?;
        auth_service.sign_in(&account.email, &password)
            .await
            .map_err(|e| e.to_string())?
    };
    
    // 更新token到数据库
    store.update_account_tokens(uuid, token.clone(), refresh_token_new.clone(), expires_at)
        .await
        .map_err(|e| e.to_string())?;
    
    // 更新内存中的账户对象
    account.token = Some(token);
    account.refresh_token = Some(refresh_token_new);
    account.token_expires_at = Some(expires_at);
    
    Ok(())
}

#[tauri::command]
pub async fn login_account(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // ==================== Devin 账号分支 ====================
    if account.is_devin_account() {
        return login_account_devin(uuid, account, &store).await;
    }

    // 解密密码
    let password = store.get_decrypted_password(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 登录获取Token
    let auth_service = AuthService::new();
    let (token, refresh_token, expires_at) = auth_service.sign_in(&account.email, &password)
        .await
        .map_err(|e| e.to_string())?;
    
    // 更新Token和Refresh Token
    store.update_account_tokens(uuid, token.clone(), refresh_token, expires_at)
        .await
        .map_err(|e| e.to_string())?;
    
    // Firebase 账号分支（Devin 已在函数开头 return），直接用新 token 构造 AuthContext
    let ctx = AuthContext::firebase(token.clone());

    // 获取最新的配额信息
    let windsurf_service = WindsurfService::new();
    let mut updated_account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    
    // 读取设置，判断使用哪个 API
    let settings = store.get_settings().await.map_err(|e| e.to_string())?;
    println!("[login_account] use_lightweight_api = {}", settings.use_lightweight_api);
    
    if settings.use_lightweight_api {
        // 使用轻量级 GetPlanStatus API
        if let Ok(result) = windsurf_service.get_plan_status(&ctx).await {
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(plan_status) = result.get("plan_status") {
                    apply_plan_status_to_account(plan_status, &mut updated_account);
                    store.update_account(updated_account.clone()).await
                        .map_err(|e| format!("保存账户信息失败: {}", e))?;
                }
            }
        }
    } else {
        // 使用完整的 GetCurrentUser API
        if let Ok(user_info_result) = windsurf_service.get_current_user(&ctx).await {
            if let Some(user_info) = user_info_result.get("user_info") {
                // 提取用户基本信息（包含api_key）
                if let Some(user) = user_info.get("user") {
                    if let Some(api_key) = user.get("api_key").and_then(|v| v.as_str()) {
                        updated_account.windsurf_api_key = Some(api_key.to_string());
                    }
                    // 提取账户禁用状态
                    if let Some(disable_codeium) = user.get("disable_codeium").and_then(|v| v.as_bool()) {
                        updated_account.is_disabled = Some(disable_codeium);
                    }
                }

                // 提取套餐信息
                if let Some(plan) = user_info.get("plan") {
                    if let Some(plan_name) = plan.get("plan_name").and_then(|v| v.as_str()) {
                        updated_account.plan_name = Some(plan_name.to_string());
                    }
                    if let Some(bs) = plan.get("billing_strategy").and_then(|v| v.as_i64()) {
                        updated_account.billing_strategy = Some(bs as i32);
                    }
                }

                // 提取配额信息
                if let Some(subscription) = user_info.get("subscription") {
                    if let Some(used) = subscription.get("used_quota").and_then(|v| v.as_i64()) {
                        updated_account.used_quota = Some(used as i32);
                    }
                    if let Some(total) = subscription.get("quota").and_then(|v| v.as_i64()) {
                        updated_account.total_quota = Some(total as i32);
                    }
                    // 提取订阅到期时间
                    if let Some(expires_at) = subscription.get("expires_at").and_then(|v| v.as_i64()) {
                        updated_account.subscription_expires_at = chrono::DateTime::from_timestamp(expires_at, 0);
                    }
                    // 提取订阅激活状态
                    if let Some(subscription_active) = subscription.get("subscription_active").and_then(|v| v.as_bool()) {
                        updated_account.subscription_active = Some(subscription_active);
                    }
                }
                
                // 直接从 user_info 提取 is_root_admin（团队所有者）
                let is_root_admin = user_info.get("is_root_admin")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                updated_account.is_team_owner = Some(is_root_admin);

                updated_account.last_quota_update = Some(chrono::Utc::now());
                store.update_account(updated_account.clone()).await
                    .map_err(|e| format!("保存账户信息失败: {}", e))?;
            }
        }
    }
    
    // 如果使用轻量级 API 或者之前没有获取到，需要单独获取 is_team_owner
    if updated_account.is_team_owner.is_none() {
        let is_team_owner = check_is_team_owner(&windsurf_service, &ctx, &updated_account.email).await;
        updated_account.is_team_owner = Some(is_team_owner);
        store.update_account(updated_account.clone()).await
            .map_err(|e| format!("保存账户信息失败: {}", e))?;
    }

    // 记录日志
    let log = OperationLog::new(
        OperationType::Login,
        OperationStatus::Success,
        format!("账号登录成功: {}", account.email),
    )
    .with_account(uuid, account.email);
    
    let _ = store.add_log(log).await;
    
    Ok(json!({
        "success": true,
        "expires_at": expires_at.to_rfc3339(),
        "plan_name": updated_account.plan_name,
        "used_quota": updated_account.used_quota,
        "total_quota": updated_account.total_quota,
        "subscription_expires_at": updated_account.subscription_expires_at.map(|dt| dt.to_rfc3339()),
        "is_disabled": updated_account.is_disabled,
        "is_team_owner": updated_account.is_team_owner
    }))
}

#[tauri::command]
pub async fn refresh_token(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // ==================== Devin 账号分支 ====================
    if account.is_devin_account() {
        return refresh_token_devin(uuid, account, &store).await;
    }

    // 保留过期时间信息用于参考
    let old_expires_at = account.token_expires_at
        .map(|t| t.to_rfc3339())
        .unwrap_or_else(|| "未知".to_string());
    
    let auth_service = AuthService::new();
    
    // 优先尝试使用refresh token
    let (token, refresh_token_new, expires_at) = if let Some(refresh_token) = &account.refresh_token {
        // 尝试使用refresh token
        match auth_service.refresh_token(refresh_token).await {
            Ok(result) => result,
            Err(_) => {
                // refresh token失败，重新登录
                let password = store.get_decrypted_password(uuid)
                    .await
                    .map_err(|e| e.to_string())?;
                auth_service.sign_in(&account.email, &password)
                    .await
                    .map_err(|e| e.to_string())?
            }
        }
    } else {
        // 没有refresh token，直接重新登录
        let password = store.get_decrypted_password(uuid)
            .await
            .map_err(|e| e.to_string())?;
        auth_service.sign_in(&account.email, &password)
            .await
            .map_err(|e| e.to_string())?
    };
    
    // 更新Token和Refresh Token
    store.update_account_tokens(uuid, token.clone(), refresh_token_new, expires_at)
        .await
        .map_err(|e| e.to_string())?;
    
    // Firebase 账号分支，直接用新 token 构造 AuthContext
    let ctx = AuthContext::firebase(token.clone());

    // 获取最新的配额信息
    let windsurf_service = WindsurfService::new();
    let mut updated_account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    
    // 读取设置，判断使用哪个 API
    let settings = store.get_settings().await.map_err(|e| e.to_string())?;
    println!("[refresh_token] use_lightweight_api = {}", settings.use_lightweight_api);
    
    if settings.use_lightweight_api {
        // 使用轻量级 GetPlanStatus API
        if let Ok(result) = windsurf_service.get_plan_status(&ctx).await {
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(plan_status) = result.get("plan_status") {
                    apply_plan_status_to_account(plan_status, &mut updated_account);
                    store.update_account(updated_account.clone()).await
                        .map_err(|e| format!("保存账户信息失败: {}", e))?;
                }
            }
        }
    } else {
        // 使用完整的 GetCurrentUser API
        if let Ok(user_info_result) = windsurf_service.get_current_user(&ctx).await {
            if let Some(user_info) = user_info_result.get("user_info") {
                // 提取用户基本信息（包含api_key）
                if let Some(user) = user_info.get("user") {
                    if let Some(api_key) = user.get("api_key").and_then(|v| v.as_str()) {
                        updated_account.windsurf_api_key = Some(api_key.to_string());
                    }
                    // 提取账户禁用状态
                    if let Some(disable_codeium) = user.get("disable_codeium").and_then(|v| v.as_bool()) {
                        updated_account.is_disabled = Some(disable_codeium);
                    }
                }

                // 提取套餐信息
                if let Some(plan) = user_info.get("plan") {
                    if let Some(plan_name) = plan.get("plan_name").and_then(|v| v.as_str()) {
                        updated_account.plan_name = Some(plan_name.to_string());
                    }
                    if let Some(bs) = plan.get("billing_strategy").and_then(|v| v.as_i64()) {
                        updated_account.billing_strategy = Some(bs as i32);
                    }
                }

                // 提取配额信息
                if let Some(subscription) = user_info.get("subscription") {
                    if let Some(used) = subscription.get("used_quota").and_then(|v| v.as_i64()) {
                        updated_account.used_quota = Some(used as i32);
                    }
                    if let Some(total) = subscription.get("quota").and_then(|v| v.as_i64()) {
                        updated_account.total_quota = Some(total as i32);
                    }
                    if let Some(expires_at) = subscription.get("expires_at").and_then(|v| v.as_i64()) {
                        updated_account.subscription_expires_at = chrono::DateTime::from_timestamp(expires_at, 0);
                    }
                    // 提取订阅激活状态
                    if let Some(subscription_active) = subscription.get("subscription_active").and_then(|v| v.as_bool()) {
                        updated_account.subscription_active = Some(subscription_active);
                    }
                }
                
                // 直接从 user_info 提取 is_root_admin（团队所有者）
                let is_root_admin = user_info.get("is_root_admin")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                updated_account.is_team_owner = Some(is_root_admin);

                updated_account.last_quota_update = Some(chrono::Utc::now());
                store.update_account(updated_account.clone()).await
                    .map_err(|e| format!("保存账户信息失败: {}", e))?;
            }
        }
    }
    
    // 如果使用轻量级 API 或者之前没有获取到，需要单独获取 is_team_owner
    if updated_account.is_team_owner.is_none() {
        let is_team_owner = check_is_team_owner(&windsurf_service, &ctx, &updated_account.email).await;
        updated_account.is_team_owner = Some(is_team_owner);
        store.update_account(updated_account.clone()).await
            .map_err(|e| format!("保存账户信息失败: {}", e))?;
    }

    // 记录日志
    let log = OperationLog::new(
        OperationType::RefreshToken,
        OperationStatus::Success,
        format!("刷新Token成功: {}", account.email),
    )
    .with_account(uuid, account.email);

    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "token": token,
        "expires_at": expires_at.to_rfc3339(),
        "old_expires_at": old_expires_at,
        "message": "Token已成功刷新",
        "plan_name": updated_account.plan_name,
        "used_quota": updated_account.used_quota,
        "total_quota": updated_account.total_quota,
        "subscription_expires_at": updated_account.subscription_expires_at.map(|dt| dt.to_rfc3339()),
        "is_disabled": updated_account.is_disabled,
        "is_team_owner": updated_account.is_team_owner,
        "windsurf_api_key": updated_account.windsurf_api_key,
        "last_quota_update": updated_account.last_quota_update.map(|t| t.to_rfc3339()),
        "billing_strategy": updated_account.billing_strategy,
        "daily_quota_remaining_percent": updated_account.daily_quota_remaining_percent,
        "weekly_quota_remaining_percent": updated_account.weekly_quota_remaining_percent,
        "daily_quota_reset_at_unix": updated_account.daily_quota_reset_at_unix,
        "weekly_quota_reset_at_unix": updated_account.weekly_quota_reset_at_unix,
        "overage_balance_micros": updated_account.overage_balance_micros
    }))
}

/// 获取账号的套餐状态（积分/配额信息）
/// 比 get_current_user 更轻量，专用于刷新积分状态
#[tauri::command]
pub async fn get_plan_status(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token（优先使用缓存）
    ensure_valid_token(&store, &mut account, uuid).await?;
    
    // 构造 AuthContext（涵盖 Devin 账号的 5 个完整 header）
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    
    // 调用GetPlanStatus API
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_plan_status(&ctx)
        .await
        .map_err(|e: AppError| e.to_string())?;
    
    // 如果成功，更新账号的配额信息
    if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
        if let Some(plan_status) = result.get("plan_status") {
            let mut updated_account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
            
            apply_plan_status_to_account(plan_status, &mut updated_account);
            
            // 获取团队成员信息，判断是否为团队所有者（Admin）
            let is_team_owner = check_is_team_owner(&windsurf_service, &ctx, &updated_account.email).await;
            updated_account.is_team_owner = Some(is_team_owner);
            
            // 保存更新后的账户信息
            store.update_account(updated_account)
                .await
                .map_err(|e| format!("保存账户信息失败: {}", e))?;
        }
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn reset_credits(
    id: String,
    seat_count: Option<i32>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token（优先使用缓存）
    ensure_valid_token(&store, &mut account, uuid).await?;
    
    // 构造 AuthContext（涵盖 Devin 账号的 5 个完整 header）
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    
    // 获取座位数选项配置
    let settings = store.get_settings().await.map_err(|e| e.to_string())?;
    let seat_count_options = settings.seat_count_options;
    
    // 执行积分重置
    let windsurf_service = WindsurfService::new();
    let result: serde_json::Value = windsurf_service.reset_credits(&ctx, seat_count, account.last_seat_count, &seat_count_options)
        .await
        .map_err(|e: AppError| e.to_string())?;
    
    // 更新最后使用的座位数
    if let Some(used_seat_count) = result.get("seat_count_used").and_then(|v| v.as_i64()) {
        account.last_seat_count = Some(used_seat_count as i32);
        store.update_account(account.clone())
            .await
            .map_err(|e| e.to_string())?;
    }
    
    // 记录日志
    let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let log = OperationLog::new(
        OperationType::ResetCredits,
        if success { OperationStatus::Success } else { OperationStatus::Failed },
        format!("积分重置{}: {}", if success { "成功" } else { "失败" }, account.email),
    )
    .with_account(uuid, account.email)
    .with_details(result.clone());
    
    let _ = store.add_log(log).await;
    
    Ok(result)
}

#[tauri::command]
pub async fn update_seats(
    id: String,
    seat_count: i32,
    retry_times: i32,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token（优先使用缓存）
    ensure_valid_token(&store, &mut account, uuid).await?;
    
    // 使用缓存的或新刷新的Token
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    
    // 执行座位更新
    let windsurf_service = WindsurfService::new();
    let result: UpdateSeatsResult = windsurf_service.update_seats(&ctx, seat_count, retry_times)
        .await
        .map_err(|e: AppError| e.to_string())?;
    
    // 记录日志
    let account = store.get_account(uuid).await.ok();
    if let Some(acc) = account {
        // 提取解析后的座位信息
        let details = if let Some(last_attempt) = result.attempts.last() {
            if let Some(raw) = &last_attempt.raw_response {
                // 尝试解析JSON格式的响应数据
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
                    let mut info = Vec::new();
                    if let Some(usage) = parsed.get("seat_usage") {
                        info.push(format!("座位使用: {}", usage));
                    }
                    if let Some(price) = parsed.get("total_monthly_price") {
                        info.push(format!("月费: ${}", price));
                    }
                    if let Some(price_per) = parsed.get("price_per_seat") {
                        info.push(format!("每座位: ${}", price_per));
                    }
                    if let Some(next_billing) = parsed.get("next_billing_time") {
                        info.push(format!("下次计费: {}", next_billing));
                    }
                    if !info.is_empty() {
                        format!(" ({})", info.join(", "))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        let log = OperationLog::new(
            OperationType::UpdateSeats,
            if result.success { OperationStatus::Success } else { OperationStatus::Failed },
            format!("更新座位数为{}: {}{}", seat_count, acc.email, details),
        )
        .with_account(uuid, acc.email);
        
        let _ = store.add_log(log).await;
    }
    
    Ok(serde_json::to_value(result).unwrap())
}

#[tauri::command]
pub async fn get_billing(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token（优先使用缓存）
    ensure_valid_token(&store, &mut account, uuid).await?;
    
    // 使用缓存的或新刷新的Token
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    
    // 获取账单信息
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_team_billing(&ctx)
        .await
        .map_err(|e: AppError| e.to_string())?;
    
    // 记录日志
    let account = store.get_account(uuid).await.ok();
    if let Some(acc) = account {
        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let log = OperationLog::new(
            OperationType::GetBilling,
            if success { OperationStatus::Success } else { OperationStatus::Failed },
            format!("查询账单{}: {}", if success { "成功" } else { "失败" }, acc.email),
        )
        .with_account(uuid, acc.email);
        
        let _ = store.add_log(log).await;
    }

    Ok(result)
}

/// 取消订阅
///
/// # Arguments
/// * `id` - 账户ID
/// * `reason` - 取消原因（例如："too_expensive", "not_using", "missing_features", "switching_service", "other"）
///
/// # Returns
/// 返回包含操作结果的 JSON 对象
#[tauri::command]
pub async fn cancel_subscription(
    id: String,
    reason: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息并构造 AuthContext（支持 Devin 5-header 的 完整鉴权）
    let account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    // 取消订阅
    let windsurf_service = WindsurfService::new();
    let result: serde_json::Value = windsurf_service.cancel_plan(&ctx, &reason)
        .await
        .map_err(|e: AppError| e.to_string())?;

    // 记录日志直接复用之前获取的 account
    {
        let acc = &account;
        let log = OperationLog::new(
            OperationType::UpdatePlan, // 使用 UpdatePlan 类型，因为这也是订阅管理操作
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                OperationStatus::Success
            } else {
                OperationStatus::Failed
            },
            format!("取消订阅 (原因: {}): {}", reason, acc.email),
        )
        .with_account(uuid, acc.email.clone());

        let _ = store.add_log(log).await;
    }

    Ok(result)
}

/// 恢复订阅
///
/// # Arguments
/// * `id` - 账户ID
///
/// # Returns
/// 返回包含操作结果的 JSON 对象
#[tauri::command]
pub async fn resume_subscription(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息并构造 AuthContext（支持 Devin 5-header 的 完整鉴权）
    let account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    // 恢复订阅
    let windsurf_service = WindsurfService::new();
    let result: serde_json::Value = windsurf_service.resume_plan(&ctx)
        .await
        .map_err(|e: AppError| e.to_string())?;

    // 记录日志直接复用之前获取的 account
    {
        let acc = &account;
        let log = OperationLog::new(
            OperationType::UpdatePlan, // 使用 UpdatePlan 类型，因为这也是订阅管理操作
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                OperationStatus::Success
            } else {
                OperationStatus::Failed
            },
            format!("恢复订阅: {}", acc.email),
        )
        .with_account(uuid, acc.email.clone());

        let _ = store.add_log(log).await;
    }

    Ok(result)
}

async fn reset_credits_internal(
    id: &str,
    seat_count: Option<i32>,
    store: &Arc<DataStore>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token（优先使用缓存）
    ensure_valid_token(&store, &mut account, uuid).await?;
    
    // 构造 AuthContext（涵盖 Devin 账号的 5 个完整 header）
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    
    // 获取座位数选项配置
    let settings = store.get_settings().await.map_err(|e| e.to_string())?;
    let seat_count_options = settings.seat_count_options;
    
    // 执行积分重置
    let windsurf_service = WindsurfService::new();
    let result: serde_json::Value = windsurf_service.reset_credits(&ctx, seat_count, account.last_seat_count, &seat_count_options)
        .await
        .map_err(|e: AppError| e.to_string())?;
    
    // v1.7.7 批量积分重置性能修复（与主端同步）：_no_save 变体 + request_save_coalesced 合并落盘
    // 单账号 reset_credits 路径：coalescer 立即 spawn worker 异步落盘（exit hook 兜底 crash 安全）。
    // batch_reset_credits 路径：N 个并发 reset 的 N 次 save 经 CAS 合并为 1~2 次实际 atomic_write。
    // 更新最后使用的座位数
    if let Some(used_seat_count) = result.get("seat_count_used").and_then(|v| v.as_i64()) {
        account.last_seat_count = Some(used_seat_count as i32);
        store.update_account_no_save(account.clone())
            .await
            .map_err(|e| e.to_string())?;
        store.request_save_coalesced();
    }
    
    // 记录详细的操作日志
    let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let message = result.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or(if success { "积分重置成功" } else { "积分重置失败" });
    
    let log = OperationLog::new(
        OperationType::ResetCredits,
        if success { OperationStatus::Success } else { OperationStatus::Failed },
        format!("{}: {}", account.email, message),
    )
    .with_account(uuid, account.email.clone());
    
    let _ = store.add_log(log).await;
    
    Ok(result)
}

#[tauri::command]
pub async fn update_plan(
    id: String,
    plan_type: String,
    payment_period: Option<u8>,
    preview: Option<bool>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let period = payment_period.unwrap_or(1); // 默认月付
    let is_preview = preview.unwrap_or(false); // 默认非预览模式

    // 获取账号信息并构造 AuthContext（支持 Devin 5-header 的 完整鉴权）
    let account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    // 更换订阅计划
    let windsurf_service = WindsurfService::new();
    let result: serde_json::Value = windsurf_service.update_plan(&ctx, &plan_type, period, is_preview)
        .await
        .map_err(|e: AppError| e.to_string())?;

    // 记录日志直接复用之前获取的 account
    let period_name = if period == 2 { "年付" } else { "月付" };
    {
        let acc = &account;
        let log = OperationLog::new(
            OperationType::UpdatePlan,
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                OperationStatus::Success
            } else {
                OperationStatus::Failed
            },
            format!("更换订阅计划到{}({}): {}", plan_type, period_name, acc.email),
        )
        .with_account(uuid, acc.email.clone());

        let _ = store.add_log(log).await;
    }

    // 更换成功后,获取最新的账号信息并保存
    if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
        let mut updated_account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
        let settings = store.get_settings().await.map_err(|e| e.to_string())?;
        
        if settings.use_lightweight_api {
            // 使用轻量级 GetPlanStatus API
            if let Ok(plan_result) = windsurf_service.get_plan_status(&ctx).await {
                if plan_result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                    if let Some(plan_status) = plan_result.get("plan_status") {
                        apply_plan_status_to_account(plan_status, &mut updated_account);
                        store.update_account(updated_account.clone()).await
                            .map_err(|e| format!("保存账户信息失败: {}", e))?;
                    }
                }
            }
        } else {
            // 使用完整的 GetCurrentUser API
            if let Ok(user_info_result) = windsurf_service.get_current_user(&ctx).await {
                if let Some(user_info) = user_info_result.get("user_info") {
                    // 提取用户基本信息（包含api_key）
                    if let Some(user) = user_info.get("user") {
                        if let Some(api_key) = user.get("api_key").and_then(|v| v.as_str()) {
                            updated_account.windsurf_api_key = Some(api_key.to_string());
                        }
                    }

                    // 提取套餐信息
                    if let Some(plan) = user_info.get("plan") {
                        if let Some(plan_name) = plan.get("plan_name").and_then(|v| v.as_str()) {
                            updated_account.plan_name = Some(plan_name.to_string());
                        }
                        if let Some(bs) = plan.get("billing_strategy").and_then(|v| v.as_i64()) {
                            updated_account.billing_strategy = Some(bs as i32);
                        }
                    }

                    // 提取配额信息
                    if let Some(subscription) = user_info.get("subscription") {
                        if let Some(used) = subscription.get("used_quota").and_then(|v| v.as_i64()) {
                            updated_account.used_quota = Some(used as i32);
                        }
                        if let Some(total) = subscription.get("quota").and_then(|v| v.as_i64()) {
                            updated_account.total_quota = Some(total as i32);
                        }
                        if let Some(expires_at) = subscription.get("expires_at").and_then(|v| v.as_i64()) {
                            updated_account.subscription_expires_at = chrono::DateTime::from_timestamp(expires_at, 0);
                        }
                        // 提取订阅激活状态
                        if let Some(subscription_active) = subscription.get("subscription_active").and_then(|v| v.as_bool()) {
                            updated_account.subscription_active = Some(subscription_active);
                        }
                    }

                    updated_account.last_quota_update = Some(chrono::Utc::now());
                    store.update_account(updated_account.clone()).await
                        .map_err(|e| format!("保存账户信息失败: {}", e))?;
                }
            }
        }

        // 返回包含更新后账户信息的结果
        return Ok(json!({
            "success": true,
            "plan_type": plan_type,
            "plan_name": updated_account.plan_name,
            "used_quota": updated_account.used_quota,
            "total_quota": updated_account.total_quota,
            "subscription_expires_at": updated_account.subscription_expires_at.map(|dt| dt.to_rfc3339()),
            "message": format!("成功更换到 {} 计划", plan_type.to_uppercase())
        }));
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_current_user(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    get_current_user_internal(&id, &store, false).await
}

/// 内部实现，支持 401 自动重试
fn get_current_user_internal<'a>(
    id: &'a str,
    store: &'a Arc<DataStore>,
    is_retry: bool,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'a>> {
    let id = id.to_string();
    let store = store.clone();
    Box::pin(async move {
        let id = id.as_str();
    let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token（如果是重试则强制刷新）
    ensure_valid_token_with_force(&store, &mut account, uuid, is_retry).await?;
    
    // 使用缓存的或新刷新的Token
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    
    // 读取设置，判断使用哪个 API
    let settings = store.get_settings().await.map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    
    println!("[get_current_user] use_lightweight_api = {}", settings.use_lightweight_api);
    
    let mut updated_account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    
    if settings.use_lightweight_api {
        // 使用轻量级 GetPlanStatus API
        println!("[get_current_user] Using GetPlanStatus API");
        
        let result = windsurf_service.get_plan_status(&ctx)
            .await
            .map_err(|e: AppError| e.to_string())?;
        
        // 检查是否是 401 错误，如果是且未重试过，则强制刷新 token 并重试
        let status_code = result.get("status_code").and_then(|v| v.as_u64()).unwrap_or(0);
        if status_code == 401 && !is_retry {
            println!("[get_current_user] 收到 401 错误，强制刷新 token 并重试...");
            return get_current_user_internal(id, &store, true).await;
        }
        
        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        
        // 提取 plan_status 中的字段，构建兼容的数据结构
        let mut plan_name = String::new();
        let mut used_quota: i64 = 0;
        let mut total_quota: i64 = 0;
        let mut expires_at: i64 = 0;
        let mut billing_strategy_val: Option<i32> = updated_account.billing_strategy;
        
        if success {
            if let Some(plan_status) = result.get("plan_status") {
                apply_plan_status_to_account(plan_status, &mut updated_account);
                
                // 同步局部变量用于构建响应JSON
                plan_name = updated_account.plan_name.clone().unwrap_or_default();
                used_quota = updated_account.used_quota.unwrap_or(0) as i64;
                total_quota = updated_account.total_quota.unwrap_or(0) as i64;
                expires_at = updated_account.subscription_expires_at
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0);
                billing_strategy_val = updated_account.billing_strategy;
                
                store.update_account(updated_account).await
                    .map_err(|e| format!("保存账户信息失败: {}", e))?;
            }
        }
        
        // 记录日志
        let log = OperationLog::new(
            OperationType::GetAccountInfo,
            if success { OperationStatus::Success } else { OperationStatus::Failed },
            format!("获取配额信息(轻量级){}: {}", if success { "成功" } else { "失败" }, account.email),
        )
        .with_account(uuid, account.email);
        let _ = store.add_log(log).await;
        
        // 返回与完整 API 兼容的数据格式
        if success {
            Ok(json!({
                "success": true,
                "lightweight": true,
                "user_info": {
                    "plan": {
                        "plan_name": plan_name,
                        "billing_strategy": billing_strategy_val
                    },
                    "subscription": {
                        "used_quota": used_quota,
                        "quota": total_quota,
                        "expires_at": expires_at
                    }
                },
                "plan_status": result.get("plan_status"),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        } else {
            Ok(result)
        }
    } else {
        // 使用完整的 GetCurrentUser API
        println!("[get_current_user] Using GetCurrentUser API");
        
        let result: serde_json::Value = windsurf_service.get_current_user(&ctx)
            .await
            .map_err(|e: AppError| e.to_string())?;
        
        // 检查是否是 401 错误，如果是且未重试过，则强制刷新 token 并重试
        let status_code = result.get("status_code").and_then(|v| v.as_u64()).unwrap_or(0);
        if status_code == 401 && !is_retry {
            println!("[get_current_user] 收到 401 错误，强制刷新 token 并重试...");
            return get_current_user_internal(id, &store, true).await;
        }
        
        // 提取并保存用户信息到数据库
        if let Some(user_info) = result.get("user_info") {
            // 提取用户基本信息（包含api_key）
            if let Some(user) = user_info.get("user") {
                if let Some(api_key) = user.get("api_key").and_then(|v| v.as_str()) {
                    updated_account.windsurf_api_key = Some(api_key.to_string());
                }
            }

            // 提取套餐信息
            if let Some(plan) = user_info.get("plan") {
                if let Some(plan_name) = plan.get("plan_name").and_then(|v| v.as_str()) {
                    updated_account.plan_name = Some(plan_name.to_string());
                }
                if let Some(bs) = plan.get("billing_strategy").and_then(|v| v.as_i64()) {
                    updated_account.billing_strategy = Some(bs as i32);
                }
            }

            // 提取配额信息
            if let Some(subscription) = user_info.get("subscription") {
                if let Some(used) = subscription.get("used_quota").and_then(|v| v.as_i64()) {
                    updated_account.used_quota = Some(used as i32);
                }
                if let Some(total) = subscription.get("quota").and_then(|v| v.as_i64()) {
                    updated_account.total_quota = Some(total as i32);
                }
                // 提取订阅到期时间
                if let Some(expires_at) = subscription.get("expires_at").and_then(|v| v.as_i64()) {
                    updated_account.subscription_expires_at = chrono::DateTime::from_timestamp(expires_at, 0);
                }
                // 提取订阅激活状态
                if let Some(subscription_active) = subscription.get("subscription_active").and_then(|v| v.as_bool()) {
                    updated_account.subscription_active = Some(subscription_active);
                }
            }
            
            // 提取 is_root_admin（团队所有者）
            let is_root_admin = user_info.get("is_root_admin")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            updated_account.is_team_owner = Some(is_root_admin);

            updated_account.last_quota_update = Some(chrono::Utc::now());

            // 保存更新后的账户信息
            store.update_account(updated_account).await
                .map_err(|e| format!("保存账户信息失败: {}", e))?;
        }
        
        // 记录日志
        let success = result.get("user_info").is_some();
        let log = OperationLog::new(
            OperationType::GetAccountInfo,
            if success { OperationStatus::Success } else { OperationStatus::Failed },
            format!("获取用户信息{}: {}", if success { "成功" } else { "失败" }, account.email),
        )
        .with_account(uuid, account.email);
        
        let _ = store.add_log(log).await;
        
        Ok(result)
    }
    })
}

#[tauri::command]
pub async fn get_account_info(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token（优先使用缓存）
    ensure_valid_token(&store, &mut account, uuid).await?;
    
    // 使用缓存的或新刷新的Token
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    let local_info = json!({
        "id": account.id,
        "email": account.email,
        "nickname": account.nickname,
        "group": account.group,
        "tags": account.tags,
        "created_at": account.created_at,
        "last_login_at": account.last_login_at,
        "last_seat_count": account.last_seat_count,
        "token_expires_at": account.token_expires_at,
        "status": account.status,
        "auth_provider": account.auth_provider,
    });

    // ==================== Devin 账号分支 ====================
    // Devin 账号不持有 Firebase idToken，不能调 accounts:lookup；改走 Windsurf GetCurrentUser
    if account.is_devin_account() {
        let windsurf_service = WindsurfService::new();
        let user_info_result = windsurf_service
            .get_current_user(&ctx)
            .await
            .map_err(|e| e.to_string())?;

        return Ok(json!({
            "success": true,
            "auth_provider": "devin",
            "local_info": local_info,
            "firebase_info": serde_json::Value::Null,
            "devin_info": {
                "devin_account_id": account.devin_account_id,
                "devin_primary_org_id": account.devin_primary_org_id,
                "has_auth1_token": account.devin_auth1_token.is_some(),
                "user_info": user_info_result.get("user_info").cloned()
                    .unwrap_or(serde_json::Value::Null),
            }
        }));
    }

    // ==================== Firebase 分支（原有逻辑） ====================
    // AuthService 是 Firebase 专属的 Google Identity API，仅需 idToken
    let auth_service = AuthService::new();
    let account_info = auth_service.get_account_info(&ctx.token)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(json!({
        "success": true,
        "auth_provider": account.auth_provider.clone().unwrap_or_else(|| "firebase".to_string()),
        "local_info": local_info,
        "firebase_info": {
            "localId": account_info.local_id,
            "email": account_info.email,
            "displayName": account_info.display_name,
            "emailVerified": account_info.email_verified,
            "passwordHash": account_info.password_hash,
            "passwordUpdatedAt": account_info.password_updated_at,
            "validSince": account_info.valid_since,
            "disabled": account_info.disabled,
            "createdAt": account_info.created_at,
            "lastLoginAt": account_info.last_login_at,
            "lastRefreshAt": account_info.last_refresh_at,
            "providerUserInfo": account_info.provider_user_info
        }
    }))
}

// ============================================================================
// Devin 账号专用的内部辅助函数
// ============================================================================

/// Devin 账号版的 `login_account`：使用 auth1_token 重新换取 session_token + enrich
async fn login_account_devin(
    uuid: Uuid,
    mut account: Account,
    store: &Arc<DataStore>,
) -> Result<serde_json::Value, String> {
    let new_token = refresh_devin_session_in_memory(&mut account).await?;
    account.token_expires_at = Some(devin_session_pseudo_expires_at());
    account.status = crate::models::account::AccountStatus::Active;
    account.last_login_at = Some(chrono::Utc::now());

    // 拉取最新用户信息
    enrich_account_with_user_info(&mut account, &new_token).await;

    // is_team_owner：refresh_devin_session_in_memory 已更新 account 的 Devin 字段，此时构造的 ctx 具备完整 5-header
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    if account.is_team_owner.is_none() {
        let is_team_owner =
            check_is_team_owner(&windsurf_service, &ctx, &account.email).await;
        account.is_team_owner = Some(is_team_owner);
    }

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    // 日志
    let log = OperationLog::new(
        OperationType::Login,
        OperationStatus::Success,
        format!("Devin 账号会话刷新成功: {}", account.email),
    )
    .with_account(uuid, account.email.clone());
    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "auth_provider": "devin",
        "expires_at": account.token_expires_at.map(|t| t.to_rfc3339()),
        "plan_name": account.plan_name,
        "used_quota": account.used_quota,
        "total_quota": account.total_quota,
        "subscription_expires_at": account.subscription_expires_at.map(|dt| dt.to_rfc3339()),
        "is_disabled": account.is_disabled,
        "is_team_owner": account.is_team_owner,
    }))
}

/// Devin 账号版的 `refresh_token` 命令主体
async fn refresh_token_devin(
    uuid: Uuid,
    mut account: Account,
    store: &Arc<DataStore>,
) -> Result<serde_json::Value, String> {
    let new_token = refresh_devin_session_in_memory(&mut account).await?;
    account.token_expires_at = Some(devin_session_pseudo_expires_at());
    account.status = crate::models::account::AccountStatus::Active;

    // refresh_devin_session_in_memory 已同步更新 account 的所有 Devin 字段，此时构造的 ctx 具备完整 5-header
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    // 读取设置，判断使用哪个 API
    let settings = store.get_settings().await.map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();

    if settings.use_lightweight_api {
        if let Ok(result) = windsurf_service.get_plan_status(&ctx).await {
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(plan_status) = result.get("plan_status") {
                    apply_plan_status_to_account(plan_status, &mut account);
                }
            }
        }
    } else {
        enrich_account_with_user_info(&mut account, &new_token).await;
    }

    if account.is_team_owner.is_none() {
        let is_team_owner =
            check_is_team_owner(&windsurf_service, &ctx, &account.email).await;
        account.is_team_owner = Some(is_team_owner);
    }

    store
        .update_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    let log = OperationLog::new(
        OperationType::RefreshToken,
        OperationStatus::Success,
        format!("Devin 账号 Token 刷新成功: {}", account.email),
    )
    .with_account(uuid, account.email.clone());
    let _ = store.add_log(log).await;

    Ok(json!({
        "success": true,
        "auth_provider": "devin",
        "message": "Devin 会话刷新成功",
        "expires_at": account.token_expires_at.map(|t| t.to_rfc3339()),
        "plan_name": account.plan_name,
        "used_quota": account.used_quota,
        "total_quota": account.total_quota,
    }))
}

/// Devin 账号版的 `refresh_token_internal`（为批量刷新服务）
async fn refresh_token_internal_devin(
    account: &mut Account,
    store: &Arc<DataStore>,
    use_lightweight_api: bool,
    save_immediately: bool,
) -> Result<serde_json::Value, String> {
    let new_token = refresh_devin_session_in_memory(account).await?;
    account.token_expires_at = Some(devin_session_pseudo_expires_at());

    // refresh_devin_session_in_memory 已更新 account 的 Devin 字段，此时 ctx 具备完整 5-header
    let ctx = AuthContext::from_account(account).map_err(|e| e.to_string())?;

    let windsurf_service = WindsurfService::new();
    if use_lightweight_api {
        if let Ok(result) = windsurf_service.get_plan_status(&ctx).await {
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(plan_status) = result.get("plan_status") {
                    apply_plan_status_to_account(plan_status, account);
                }
            }
        }
        // 关键字段补拿：GetPlanStatus 不返回 windsurf_api_key / is_disabled /
        // subscription_active，且 plan_end 对部分 Trial 账号可能缺失或为 0。
        // 若任一关键字段仍为 None，补调一次 GetCurrentUser 回填，避免这些字段在
        // 轻量级模式下永远拿不到（典型场景：新注册后从未单刷过的 Trial 账号）。
        let needs_enrich = account.subscription_expires_at.is_none()
            || account.windsurf_api_key.is_none()
            || account.is_disabled.is_none()
            || account.subscription_active.is_none();
        if needs_enrich {
            enrich_account_with_user_info(account, &new_token).await;
        }
    } else {
        enrich_account_with_user_info(account, &new_token).await;
    }

    if account.is_team_owner.is_none() {
        let is_team_owner =
            check_is_team_owner(&windsurf_service, &ctx, &account.email).await;
        account.is_team_owner = Some(is_team_owner);
    }

    // 批量刷新场景（save_immediately = false）也必须写入 store 内存，否则
    // `batch_refresh_tokens` 末尾的 flush() 只会把旧数据再写一遍，Devin 账号的
    // 刷新结果（token / 配额 / 到期时间 / 百分比 等）全部丢失，等同于未保存。
    if save_immediately {
        store
            .update_account(account.clone())
            .await
            .map_err(|e| e.to_string())?;
    } else {
        store
            .update_account_no_save(account.clone())
            .await
            .map_err(|e| e.to_string())?;
    }

    // 返回完整字段（与 Firebase 分支 refresh_token_internal 对齐），
    // 供 batch_refresh_tokens 前端合并逻辑同步 store 内存，避免 UI 需要 F5 才能看到新值。
    // subscription_expires_at 以 Unix 秒输出，与前端 dayjs.unix() 合并逻辑匹配。
    Ok(json!({
        "success": true,
        "auth_provider": "devin",
        "message": "Devin 会话刷新成功",
        "email": account.email,
        "expires_at": account.token_expires_at.map(|t| t.to_rfc3339()),
        "plan_name": account.plan_name,
        "used_quota": account.used_quota,
        "total_quota": account.total_quota,
        "windsurf_api_key": account.windsurf_api_key,
        "is_disabled": account.is_disabled,
        "is_team_owner": account.is_team_owner,
        "subscription_expires_at": account.subscription_expires_at.map(|t| t.timestamp()),
        "subscription_active": account.subscription_active,
        "last_quota_update": account.last_quota_update.map(|t| t.to_rfc3339()),
        "billing_strategy": account.billing_strategy,
        "daily_quota_remaining_percent": account.daily_quota_remaining_percent,
        "weekly_quota_remaining_percent": account.weekly_quota_remaining_percent,
        "daily_quota_reset_at_unix": account.daily_quota_reset_at_unix,
        "weekly_quota_reset_at_unix": account.weekly_quota_reset_at_unix,
        "overage_balance_micros": account.overage_balance_micros,
    }))
}

#[tauri::command]
pub async fn get_team_credit_entries(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    
    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;
    
    // 确保有有效的Token
    ensure_valid_token(&store, &mut account, uuid).await?;
    
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    
    // 调用GetTeamCreditEntries API
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_team_credit_entries(&ctx)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result)
}

#[tauri::command]
pub async fn batch_reset_credits(
    ids: Vec<String>,
    seat_count: Option<i32>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    use futures::stream::{self, StreamExt};
    
    // 设置并发限制，避免过多并发请求
    const MAX_CONCURRENT: usize = 5;
    
    // 创建任务流并并发执行
    let store_arc = store.inner().clone();
    
    let results: Vec<serde_json::Value> = stream::iter(ids.into_iter().enumerate())
        .map(|(index, id_str)| {
            let store_clone = store_arc.clone();
            let seat_count_clone = seat_count;
            
            async move {
                if let Ok(_uuid) = Uuid::parse_str(&id_str) {
                    // 每个请求添加小延迟，分散请求
                    if index > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(200 * index as u64)).await;
                    }
                    
                    // 直接使用 API 服务进行批量操作
                    // 注意：传递 seat_count_clone 而不是强制分配的座位数
                    // 如果 seat_count 为 None，reset_credits_internal 会使用账号的 last_seat_count
                    let result = match reset_credits_internal(&id_str, seat_count_clone, &store_clone).await {
                        Ok(res) => {
                            let seat_used = res.get("seat_count_used")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);
                            json!({ "success": true, "data": res, "seat_count_used": seat_used })
                        },
                        Err(err) => json!({ "success": false, "error": err })
                    };
                    json!({
                        "id": id_str,
                        "result": result
                    })
                } else {
                    json!({
                        "id": id_str,
                        "result": json!({ "success": false, "error": "Invalid UUID" })
                    })
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT)
        .collect()
        .await;
    
    // 记录批量操作日志
    let success_count = results.iter()
        .filter(|r| r.get("result")
            .and_then(|res| res.get("success"))
            .and_then(|s| s.as_bool())
            .unwrap_or(false))
        .count();
    
    let log = OperationLog::new(
        OperationType::BatchOperation,
        if success_count > 0 { OperationStatus::Success } else { OperationStatus::Failed },
        format!("批量重置积分: 成功 {}/{} 个账号", success_count, results.len()),
    );
    let _ = store.add_log(log).await;
    
    Ok(json!({
        "results": results,
        "success_count": success_count,
        "total_count": results.len()
    }))
}

/// 批量刷新 Token（优化版：只在最后保存一次）
#[tauri::command]
pub async fn batch_refresh_tokens(
    ids: Vec<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    use futures::stream::{self, StreamExt};
    
    let store_arc = store.inner().clone();
    let settings = store.get_settings().await.map_err(|e| e.to_string())?;
    let use_lightweight_api = settings.use_lightweight_api;
    
    // 读取用户设置的并发配置
    let max_concurrent = if settings.unlimited_concurrent_refresh {
        ids.len() // 全量并发
    } else {
        settings.concurrent_limit.max(1) // 至少 1 个并发
    };
    
    let results: Vec<serde_json::Value> = stream::iter(ids.into_iter())
        .map(|id_str| {
            let store_clone = store_arc.clone();
            
            async move {
                if Uuid::parse_str(&id_str).is_ok() {
                    match refresh_token_internal(&id_str, &store_clone, use_lightweight_api, false).await {
                        Ok(res) => json!({
                            "id": id_str,
                            "success": true,
                            "data": res
                        }),
                        Err(err) => json!({
                            "id": id_str,
                            "success": false,
                            "error": err
                        })
                    }
                } else {
                    json!({
                        "id": id_str,
                        "success": false,
                        "error": "Invalid UUID"
                    })
                }
            }
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await;
    
    // 所有操作完成后，统一保存一次
    store.flush().await.map_err(|e| e.to_string())?;
    
    let success_count = results.iter()
        .filter(|r| r.get("success").and_then(|s| s.as_bool()).unwrap_or(false))
        .count();
    
    let log = OperationLog::new(
        OperationType::BatchOperation,
        if success_count > 0 { OperationStatus::Success } else { OperationStatus::Failed },
        format!("批量刷新Token: 成功 {}/{} 个账号", success_count, results.len()),
    );
    let _ = store.add_log(log).await;
    
    Ok(json!({
        "results": results,
        "success_count": success_count,
        "total_count": results.len()
    }))
}

/// 内部刷新 Token 方法（支持延迟保存）
async fn refresh_token_internal(
    id: &str,
    store: &Arc<DataStore>,
    use_lightweight_api: bool,
    save_immediately: bool,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
    
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;

    // ==================== Devin 账号分支 ====================
    if account.is_devin_account() {
        return refresh_token_internal_devin(
            &mut account,
            store,
            use_lightweight_api,
            save_immediately,
        )
        .await;
    }

    let auth_service = AuthService::new();
    
    // 刷新 token
    let (token, refresh_token_new, expires_at) = if let Some(ref_token) = &account.refresh_token {
        match auth_service.refresh_token(ref_token).await {
            Ok(result) => result,
            Err(_) => {
                let password = store.get_decrypted_password(uuid).await.map_err(|e| e.to_string())?;
                auth_service.sign_in(&account.email, &password).await.map_err(|e| e.to_string())?
            }
        }
    } else {
        let password = store.get_decrypted_password(uuid).await.map_err(|e| e.to_string())?;
        auth_service.sign_in(&account.email, &password).await.map_err(|e| e.to_string())?
    };
    
    // 使用延迟保存的方法更新 token
    if save_immediately {
        store.update_account_tokens(uuid, token.clone(), refresh_token_new, expires_at)
            .await.map_err(|e| e.to_string())?;
    } else {
        store.update_account_tokens_no_save(uuid, token.clone(), refresh_token_new, expires_at)
            .await.map_err(|e| e.to_string())?;
    }
    
    // Firebase 账号分支，直接用新 token 构造 AuthContext
    let ctx = AuthContext::firebase(token.clone());

    // 获取配额信息
    let windsurf_service = WindsurfService::new();
    let mut updated_account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    
    if use_lightweight_api {
        // 使用轻量级 GetPlanStatus API
        if let Ok(result) = windsurf_service.get_plan_status(&ctx).await {
            if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(plan_status) = result.get("plan_status") {
                    apply_plan_status_to_account(plan_status, &mut updated_account);
                }
            }
        }
    } else {
        // 使用完整的 GetCurrentUser API
        if let Ok(user_info_result) = windsurf_service.get_current_user(&ctx).await {
            if let Some(user_info) = user_info_result.get("user_info") {
                // 提取用户基本信息（包含api_key）
                if let Some(user) = user_info.get("user") {
                    if let Some(api_key) = user.get("api_key").and_then(|v| v.as_str()) {
                        updated_account.windsurf_api_key = Some(api_key.to_string());
                    }
                    // 提取账户禁用状态
                    if let Some(disable_codeium) = user.get("disable_codeium").and_then(|v| v.as_bool()) {
                        updated_account.is_disabled = Some(disable_codeium);
                    }
                }

                // 提取套餐信息
                if let Some(plan) = user_info.get("plan") {
                    if let Some(plan_name) = plan.get("plan_name").and_then(|v| v.as_str()) {
                        updated_account.plan_name = Some(plan_name.to_string());
                    }
                    if let Some(bs) = plan.get("billing_strategy").and_then(|v| v.as_i64()) {
                        updated_account.billing_strategy = Some(bs as i32);
                    }
                }

                // 提取配额信息
                if let Some(subscription) = user_info.get("subscription") {
                    if let Some(used) = subscription.get("used_quota").and_then(|v| v.as_i64()) {
                        updated_account.used_quota = Some(used as i32);
                    }
                    if let Some(total) = subscription.get("quota").and_then(|v| v.as_i64()) {
                        updated_account.total_quota = Some(total as i32);
                    }
                    if let Some(expires_at) = subscription.get("expires_at").and_then(|v| v.as_i64()) {
                        updated_account.subscription_expires_at = chrono::DateTime::from_timestamp(expires_at, 0);
                    }
                    // 提取订阅激活状态
                    if let Some(subscription_active) = subscription.get("subscription_active").and_then(|v| v.as_bool()) {
                        updated_account.subscription_active = Some(subscription_active);
                    }
                }
                
                // 提取 is_root_admin（团队所有者）
                let is_root_admin = user_info.get("is_root_admin")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                updated_account.is_team_owner = Some(is_root_admin);

                updated_account.last_quota_update = Some(chrono::Utc::now());
            }
        }
    }
    
    // 如果使用轻量级 API，需要单独获取 is_team_owner
    if updated_account.is_team_owner.is_none() {
        if let Ok(user_result) = windsurf_service.get_current_user(&ctx).await {
            if let Some(user_info) = user_result.get("user_info") {
                let is_root_admin = user_info.get("is_root_admin")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                updated_account.is_team_owner = Some(is_root_admin);
            }
        }
    }
    
    // 更新账号信息（不立即保存）
    store.update_account_no_save(updated_account.clone()).await
        .map_err(|e| format!("更新账户信息失败: {}", e))?;
    
    // 返回完整的账户信息，供前端直接更新本地 store
    Ok(json!({
        "email": account.email,
        "expires_at": expires_at.to_rfc3339(),
        "plan_name": updated_account.plan_name,
        "used_quota": updated_account.used_quota,
        "total_quota": updated_account.total_quota,
        "windsurf_api_key": updated_account.windsurf_api_key,
        "is_disabled": updated_account.is_disabled,
        "is_team_owner": updated_account.is_team_owner,
        "subscription_expires_at": updated_account.subscription_expires_at.map(|t| t.to_rfc3339()),
        "subscription_active": updated_account.subscription_active,
        "last_quota_update": updated_account.last_quota_update.map(|t| t.to_rfc3339()),
        "billing_strategy": updated_account.billing_strategy,
        "daily_quota_remaining_percent": updated_account.daily_quota_remaining_percent,
        "weekly_quota_remaining_percent": updated_account.weekly_quota_remaining_percent,
        "daily_quota_reset_at_unix": updated_account.daily_quota_reset_at_unix,
        "weekly_quota_reset_at_unix": updated_account.weekly_quota_reset_at_unix,
        "overage_balance_micros": updated_account.overage_balance_micros
    }))
}

/// 获取试用绑卡链接
///
/// # Arguments
/// * `id` - 账号ID
/// * `teams_tier` - 团队等级: 1=Teams, 2=Pro, 3=Enterprise
/// * `payment_period` - 支付周期: 1=月付, 2=年付
/// * `team_name` - 团队名称 (仅 Teams/Enterprise 需要)
/// * `seat_count` - 席位数量 (仅 Teams/Enterprise 需要)
/// * `turnstile_token` - Turnstile 验证令牌 (start_trial=true 时所有计划均必需)
///
/// # Returns
/// 返回包含Stripe Checkout链接的JSON对象
#[tauri::command]
pub async fn get_trial_payment_link(
    id: String,
    teams_tier: Option<i32>,
    payment_period: Option<i32>,
    start_trial: Option<bool>,
    team_name: Option<String>,
    seat_count: Option<i32>,
    turnstile_token: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // 确保有有效的Token
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    // 默认值
    let final_teams_tier = teams_tier.unwrap_or(2); // 默认 Pro
    let final_payment_period = payment_period.unwrap_or(1); // 默认月付
    let final_start_trial = start_trial.unwrap_or(true); // 默认开启试用

    // 调用Windsurf API获取支付链接
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.subscribe_to_plan(
        &ctx, 
        final_teams_tier,
        final_payment_period,
        final_start_trial,
        team_name.as_deref(),
        seat_count,
        turnstile_token.as_deref()
    )
        .await
        .map_err(|e: AppError| e.to_string())?;

    // 记录日志
    let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let stripe_url = result.get("stripe_url").and_then(|v| v.as_str()).unwrap_or("");

    let plan_name = if final_teams_tier == 1 { "Teams" } else { "Pro" };
    let period_name = if final_payment_period == 1 { "月付" } else { "年付" };
    
    let log = OperationLog::new(
        OperationType::GetAccountInfo, // 暂时使用GetAccountInfo类型，可以考虑添加新的类型
        if success { OperationStatus::Success } else { OperationStatus::Failed },
        format!(
            "获取试用绑卡链接{}: {} (计划: {} {})",
            if success { "成功" } else { "失败" },
            account.email,
            plan_name,
            period_name
        ),
    )
    .with_account(uuid, account.email.clone())
    .with_details(json!({
        "teams_tier": final_teams_tier,
        "payment_period": final_payment_period,
        "stripe_url": stripe_url,
    }));

    let _ = store.add_log(log).await;

    Ok(result)
}

/// 获取团队配置
#[tauri::command]
pub async fn get_team_config(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // 确保有有效的Token
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    // 调用API获取团队配置
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_team_config(&ctx)
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(result)
}

/// 更新团队配置
#[tauri::command]
pub async fn update_team_config(
    id: String,
    config: serde_json::Value,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // 确保有有效的Token
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    // 调用API更新团队配置
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.update_team_config(&ctx, config)
        .await
        .map_err(|e: AppError| e.to_string())?;

    // 记录日志
    let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let log = OperationLog::new(
        OperationType::GetAccountInfo,
        if success { OperationStatus::Success } else { OperationStatus::Failed },
        format!(
            "更新团队设置{}: {}",
            if success { "成功" } else { "失败" },
            account.email
        ),
    )
    .with_account(uuid, account.email.clone());

    let _ = store.add_log(log).await;

    Ok(result)
}

/// 获取可用模型配置
#[tauri::command]
pub async fn get_cascade_model_configs(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_cascade_model_configs(&ctx)
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(result)
}

/// 获取 Command 模型配置
#[tauri::command]
pub async fn get_command_model_configs(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_command_model_configs(&ctx)
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(result)
}

/// 获取团队模型控制配置
#[tauri::command]
pub async fn get_team_organizational_controls(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_team_organizational_controls(&ctx)
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(result)
}

/// 更新团队模型控制配置
#[tauri::command]
pub async fn upsert_team_organizational_controls(
    id: String,
    team_id: String,
    cascade_models: Vec<String>,
    command_models: Vec<String>,
    extension_models: Vec<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.upsert_team_organizational_controls(
        &ctx,
        &team_id,
        cascade_models,
        command_models,
        extension_models,
    )
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(result)
}

/// 获取可用的 MCP 插件列表
#[tauri::command]
pub async fn get_available_mcp_plugins(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // 确保有有效的Token
    ensure_valid_token(&store, &mut account, uuid).await?;

    // 获取 api_key (windsurf_api_key)
    let api_key = account.windsurf_api_key.clone().unwrap_or_default();
    if api_key.is_empty() {
        return Err("账号没有 API Key，请先刷新账号信息".to_string());
    }

    // 调用 API 获取 MCP 插件列表
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.get_available_mcp_plugins(&api_key)
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(result)
}

/// 删除用户 (Windsurf DeleteUser API)
#[tauri::command]
pub async fn delete_windsurf_user(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // 确保有有效的Token
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    if ctx.token.is_empty() {
        return Err("账号没有有效的 Token".to_string());
    }

    // 获取 api_key
    let api_key = account.windsurf_api_key.clone().unwrap_or_default();
    if api_key.is_empty() {
        return Err("账号没有 API Key，请先刷新账号信息".to_string());
    }

    log::info!("[DeleteWindsurfUser] Deleting user for account: {}", account.email);

    // 调用 DeleteUser API
    let windsurf_service = WindsurfService::new();
    let result = windsurf_service.delete_user(&ctx, &api_key)
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(result)
}

/// 检查Pro试用资格
///
/// 接收 account_id 从 SQLite 读取完整账号，构造正确的 AuthContext：
/// - Firebase 账号：header `authorization: Bearer <idToken>`
/// - Devin 账号：header `x-auth-token` + `x-devin-session-token` + `x-devin-account-id` 等
#[tauri::command]
pub async fn check_pro_trial_eligibility(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    ensure_valid_token(&store, &mut account, uuid).await?;
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    windsurf_service.check_pro_trial_eligibility(&ctx).await.map_err(|e| e.to_string())
}

/// 获取账号的有效Token（用于批量操作或二次请求）
///
/// 内部调 ensure_valid_token 确保 token 未过期，返回 `{ success, token }`。
/// 若账号 token 过期且无法刷新，返回错误由调用方处理。
#[tauri::command]
pub async fn get_account_valid_token(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 获取账号信息
    let mut account = store.get_account(uuid)
        .await
        .map_err(|e| e.to_string())?;

    // 确保有有效的Token
    ensure_valid_token(&store, &mut account, uuid).await?;

    // 使用缓存的或新刷新的Token
    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "token": ctx.token
    }))
}

// ==================== 用户API密钥管理命令 ====================

/// 获取用户API密钥摘要列表
#[tauri::command]
pub async fn get_api_key_summary(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    windsurf_service.get_api_key_summary(&ctx).await.map_err(|e| e.to_string())
}

/// 删除用户API密钥
///
/// key_id / api_key 二选一作为 target。
#[tauri::command]
pub async fn delete_api_key(
    id: String,
    key_id: Option<String>,
    api_key: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    windsurf_service.delete_api_key(&ctx, key_id.as_deref(), api_key.as_deref())
        .await.map_err(|e| e.to_string())
}

/// 注册用户并生成新的 sk-ws-01 格式 API Key
///
/// 需要账号的 Firebase ID Token（`account.token`）。
#[tauri::command]
pub async fn register_user_api_key(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let account = store.get_account(uuid).await.map_err(|e| e.to_string())?;

    let firebase_token = account.token.clone().ok_or("No Firebase token available")?;
    let windsurf_service = WindsurfService::new();
    windsurf_service.register_user(&firebase_token).await.map_err(|e| e.to_string())
}

// ==================== 第三方 API Provider Key 管理 ====================

/// 获取已设置的第三方API Provider列表
#[tauri::command]
pub async fn get_set_user_api_provider_keys(
    id: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    windsurf_service.get_set_user_api_provider_keys(&ctx).await.map_err(|e| e.to_string())
}

/// 设置第三方API Provider Key
#[tauri::command]
pub async fn set_user_api_provider_key(
    id: String,
    provider: String,
    provider_api_key: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    let provider_id = windsurf_service.provider_name_to_id(&provider);
    windsurf_service.set_user_api_provider_key(&ctx, provider_id, &provider_api_key)
        .await.map_err(|e| e.to_string())
}

/// 删除第三方API Provider Key
#[tauri::command]
pub async fn delete_user_api_provider_key(
    id: String,
    provider: String,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
    ensure_valid_token(&store, &mut account, uuid).await?;

    let ctx = AuthContext::from_account(&account).map_err(|e| e.to_string())?;
    let windsurf_service = WindsurfService::new();
    let provider_id = windsurf_service.provider_name_to_id(&provider);
    windsurf_service.delete_user_api_provider_key(&ctx, provider_id)
        .await.map_err(|e| e.to_string())
}

// ==================== 迁移 / 开发者主密钥 / 排行榜 ====================

/// 迁移API Key到新的会话Token
#[tauri::command]
pub async fn migrate_api_key(
    api_key: String,
) -> Result<serde_json::Value, String> {
    let windsurf_service = WindsurfService::new();
    windsurf_service.migrate_api_key(&api_key).await.map_err(|e| e.to_string())
}

/// 获取开发者主API Key（通过 session_token）
#[tauri::command]
pub async fn get_primary_api_key_for_devs(
    session_token: String,
) -> Result<serde_json::Value, String> {
    let windsurf_service = WindsurfService::new();
    windsurf_service.get_primary_api_key_for_devs(&session_token).await.map_err(|e| e.to_string())
}

/// 获取全球排行榜API Key
///
/// 传入账号 id 时：强制 ensure_valid_token 刷新后把 token 作为 X-Auth-Token 请求头；
/// 不传时：匿名请求（部分 Windsurf 端点允许）。
#[tauri::command]
pub async fn get_global_leaderboard_api_key(
    id: Option<String>,
    store: State<'_, Arc<DataStore>>,
) -> Result<serde_json::Value, String> {
    let auth_token = if let Some(account_id) = id {
        let uuid = Uuid::parse_str(&account_id).map_err(|e| e.to_string())?;
        let mut account = store.get_account(uuid).await.map_err(|e| e.to_string())?;
        ensure_valid_token(&store, &mut account, uuid).await?;
        account.token
    } else {
        None
    };

    let windsurf_service = WindsurfService::new();
    windsurf_service.get_global_leaderboard_api_key(auth_token.as_deref()).await.map_err(|e| e.to_string())
}

/// 获取排行榜数据（ELO 评分、投票数、胜率等）
#[tauri::command]
pub async fn get_leaderboard(
    api_key: String,
) -> Result<serde_json::Value, String> {
    let windsurf_service = WindsurfService::new();
    windsurf_service.get_leaderboard(&api_key).await.map_err(|e| e.to_string())
}
