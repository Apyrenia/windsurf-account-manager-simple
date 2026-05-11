//! 额度耗尽自动切换（Fork 新增）
//!
//! ## 设计目标
//!
//! 当当前正在使用的 Windsurf 账号配额接近耗尽时，自动切换到剩余配额最多的可用账号，
//! 配合 `seamless_switch_enabled`（无感换号补丁）实现真正的"额度耗尽即换号、Windsurf 无需重启"。
//!
//! ## 数据流
//!
//! ```text
//! [前端 MainLayout 定时器]
//!     ↓ invoke("check_and_auto_switch")
//! [check_and_auto_switch]
//!     ├─ 1. 读取 Settings.auto_switch_enabled 等开关
//!     ├─ 2. get_current_windsurf_info → 拿到当前活跃账号的 email
//!     ├─ 3. 在账号库里按 email 匹配 → 当前 Account
//!     ├─ 4. GetPlanStatus API 查实时配额（兼容新版 QUOTA / 旧版 CREDIT 两套计费）
//!     ├─ 5. 与阈值对比 → 是否需要切换？
//!     ├─ 6. 否：返回 { triggered: false, reason }
//!     └─ 7. 是：选号策略 → perform_account_switch → 返回 { triggered: true, switched_to }
//! ```
//!
//! ## 与 auto_reset 的区别
//!
//! - 自动重置：批量重置团队成员的积分，针对**多账号**
//! - 自动切换：检测**当前正在用的单个账号**，触发账号级别的"换号"
//!
//! 两者无冲突，可以同时启用。

use crate::commands::switch_account_commands::perform_account_switch;
use crate::commands::windsurf_info::get_current_windsurf_info;
use crate::models::{Account, OperationLog, OperationStatus, OperationType};
use crate::repository::DataStore;
use crate::services::{AuthContext, WindsurfService};
use log::{info, warn};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// 自动切换决策结果（用于前端日志/toast）
///
/// 所有 `*_percent` / `*_remaining` 字段均为 0-100 百分比单位，
/// 统一兼容新版 QUOTA（按日/周配额百分比）和旧版 CREDIT（按积分使用率换算）两套计费体系。
#[derive(Debug, Clone, Serialize)]
struct AutoSwitchResult {
    /// 是否实际触发了切换
    triggered: bool,
    /// 当前账号邮箱（如检测到）
    current_email: Option<String>,
    /// 当前账号使用率百分比（0-100）
    current_usage_percent: Option<i32>,
    /// 当前账号剩余配额百分比（0-100）
    current_remaining: Option<i32>,
    /// 切换目标账号邮箱
    switched_to_email: Option<String>,
    /// 切换目标账号剩余配额百分比（0-100）
    switched_to_remaining: Option<i32>,
    /// 跳过/失败的原因
    reason: Option<String>,
}

impl AutoSwitchResult {
    fn skipped(reason: impl Into<String>) -> Self {
        Self {
            triggered: false,
            current_email: None,
            current_usage_percent: None,
            current_remaining: None,
            switched_to_email: None,
            switched_to_remaining: None,
            reason: Some(reason.into()),
        }
    }
}

/// 检查当前 Windsurf 账号配额，必要时自动切换到下一个可用账号
///
/// 由前端 `MainLayout.vue` 的定时器周期性调用（间隔由 `auto_switch_check_interval` 决定）。
///
/// ### 返回值字段
/// - `triggered`: 是否真的切了号
/// - `current_email`/`current_usage_percent`/`current_remaining`: 当前账号的状态
/// - `switched_to_email`/`switched_to_remaining`: 新账号信息（仅 triggered=true 时）
/// - `reason`: 跳过或失败时的原因（含"开关关闭"/"未达阈值"/"无可用账号"等）
///
/// ### 安全保证
/// - 开关关闭时立即 short-circuit，不发任何网络请求
/// - 当前账号识别失败 / API 失败时返回 `triggered=false`，不会误切号
/// - 切换前严格过滤：必须有 token、未禁用、剩余配额 > 阈值
#[tauri::command]
pub async fn check_and_auto_switch(
    app: AppHandle,
    data_store: State<'_, Arc<DataStore>>,
) -> Result<Value, String> {
    // ============ Step 1: 读取设置 ============
    let settings = data_store.get_settings().await.map_err(|e| e.to_string())?;

    if !settings.auto_switch_enabled {
        return Ok(serde_json::to_value(AutoSwitchResult::skipped(
            "自动切换功能未启用",
        ))
        .unwrap_or(json!({"triggered": false})));
    }

    let threshold_percent = settings.auto_switch_threshold_percent.clamp(1, 100);
    let remaining_threshold = settings.auto_switch_remaining_threshold.max(0);
    let strategy = settings.auto_switch_strategy.clone();
    let prefer_same_provider = settings.auto_switch_prefer_same_provider;

    // ============ Step 2: 识别当前活跃 Windsurf 账号 ============
    let current_info = get_current_windsurf_info().map_err(|e| e.to_string())?;
    let current_email = match current_info.email.as_ref() {
        Some(email) if !email.is_empty() => email.clone(),
        _ => {
            return Ok(serde_json::to_value(AutoSwitchResult::skipped(
                "未检测到当前活跃的 Windsurf 账号",
            ))
            .unwrap_or(json!({"triggered": false})));
        }
    };

    // ============ Step 3: 在账号库中匹配当前账号 ============
    let all_accounts = data_store
        .get_all_accounts()
        .await
        .map_err(|e| e.to_string())?;

    let current_account = match all_accounts
        .iter()
        .find(|a| a.email.to_lowercase() == current_email.to_lowercase())
    {
        Some(account) => account.clone(),
        None => {
            return Ok(serde_json::to_value(AutoSwitchResult::skipped(format!(
                "当前账号 {} 未在账号库中（请先导入）",
                current_email
            )))
            .unwrap_or(json!({"triggered": false})));
        }
    };

    // ============ Step 4: 调用 GetPlanStatus 获取实时配额 ============
    let current_ctx = match AuthContext::from_account(&current_account) {
        Ok(ctx) if !ctx.token.is_empty() => ctx,
        _ => {
            return Ok(serde_json::to_value(AutoSwitchResult::skipped(format!(
                "当前账号 {} 缺少有效 Token，无法查询配额",
                current_email
            )))
            .unwrap_or(json!({"triggered": false})));
        }
    };

    let windsurf_service = WindsurfService::new();
    let (usage_percent, remaining_percent) =
        match query_account_quota(&windsurf_service, &current_ctx).await {
            Ok(pair) => pair,
            Err(e) => {
                warn!("[AutoSwitch] 查询当前账号配额失败: {}", e);
                return Ok(serde_json::to_value(AutoSwitchResult::skipped(format!(
                    "查询配额失败: {}",
                    e
                )))
                .unwrap_or(json!({"triggered": false})));
            }
        };

    info!(
        "[AutoSwitch] 当前账号 {}: usage={}%, remaining={}%",
        current_email, usage_percent, remaining_percent
    );

    // ============ Step 5: 判断是否触发切换 ============
    //
    // 触发条件（同时满足）：
    // 1. 使用率百分比 >= 阈值（默认 95%）
    // 2. 剩余配额百分比 <= 阈值（默认 5%；=0 时跳过该判定）
    //
    // 单位均为 0-100 百分比，新旧两套计费已在 query_account_quota 里统一。
    let usage_condition = usage_percent >= threshold_percent;
    let remaining_condition = remaining_threshold == 0 || remaining_percent <= remaining_threshold;
    let should_switch = usage_condition && remaining_condition;

    if !should_switch {
        return Ok(json!({
            "triggered": false,
            "current_email": current_email,
            "current_usage_percent": usage_percent,
            "current_remaining": remaining_percent,
            "reason": format!(
                "未达切换条件（使用率 {}% < {}% 或剩余 {}% > {}%）",
                usage_percent, threshold_percent, remaining_percent, remaining_threshold
            )
        }));
    }

    // ============ Step 6: 按策略选择目标账号 ============
    let candidate = pick_next_account(
        &all_accounts,
        &current_account,
        prefer_same_provider,
        &strategy,
        remaining_threshold,
    );

    let target = match candidate {
        Some(account) => account,
        None => {
            warn!("[AutoSwitch] 没有可用账号可供切换");
            // 即使没账号可切，也要发个事件让用户知道
            let result = AutoSwitchResult {
                triggered: false,
                current_email: Some(current_email.clone()),
                current_usage_percent: Some(usage_percent),
                current_remaining: Some(remaining_percent),
                switched_to_email: None,
                switched_to_remaining: None,
                reason: Some("当前账号已耗尽，但没有其他可用账号".to_string()),
            };
            let _ = app.emit("auto-switch-result", &result);
            return Ok(serde_json::to_value(&result).unwrap_or(json!({"triggered": false})));
        }
    };

    let target_email = target.email.clone();
    let target_remaining_pct = account_remaining_percent(&target);
    let target_remaining = if target_remaining_pct >= 0 {
        Some(target_remaining_pct)
    } else {
        None
    };

    info!(
        "[AutoSwitch] 触发切换: {} → {} (策略={}, 目标剩余={}%)",
        current_email,
        target_email,
        strategy,
        target_remaining_pct
    );

    // ============ Step 7: 调用核心切号流程 ============
    let switch_result = match perform_account_switch(app.clone(), &data_store, target.id.to_string()).await {
        Ok(value) => value,
        Err(e) => {
            warn!("[AutoSwitch] 切换失败: {}", e);
            // 记录失败日志
            let log = OperationLog::new(
                OperationType::SwitchAccount,
                OperationStatus::Failed,
                format!(
                    "自动切换失败: {} → {} ({})",
                    current_email, target_email, e
                ),
            )
            .with_account(target.id, target_email.clone());
            let _ = data_store.add_log(log).await;

            let result = AutoSwitchResult {
                triggered: false,
                current_email: Some(current_email),
                current_usage_percent: Some(usage_percent),
                current_remaining: Some(remaining_percent),
                switched_to_email: None,
                switched_to_remaining: None,
                reason: Some(format!("切换失败: {}", e)),
            };
            let _ = app.emit("auto-switch-result", &result);
            return Ok(serde_json::to_value(&result).unwrap_or(json!({"triggered": false})));
        }
    };

    let switch_success = switch_result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !switch_success {
        let err_msg = switch_result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误")
            .to_string();
        warn!("[AutoSwitch] switch_account 返回失败: {}", err_msg);

        let log = OperationLog::new(
            OperationType::SwitchAccount,
            OperationStatus::Failed,
            format!("自动切换失败: {} → {} ({})", current_email, target_email, err_msg),
        )
        .with_account(target.id, target_email.clone());
        let _ = data_store.add_log(log).await;

        let result = AutoSwitchResult {
            triggered: false,
            current_email: Some(current_email),
            current_usage_percent: Some(usage_percent),
            current_remaining: Some(remaining_percent),
            switched_to_email: None,
            switched_to_remaining: None,
            reason: Some(format!("切换失败: {}", err_msg)),
        };
        let _ = app.emit("auto-switch-result", &result);
        return Ok(serde_json::to_value(&result).unwrap_or(json!({"triggered": false})));
    }

    // ============ Step 8: 成功 ============
    let log = OperationLog::new(
        OperationType::SwitchAccount,
        OperationStatus::Success,
        format!(
            "自动切换成功（额度耗尽）: {} → {} [当前使用率 {}%, 剩余 {}%]",
            current_email, target_email, usage_percent, remaining_percent
        ),
    )
    .with_account(target.id, target_email.clone());
    let _ = data_store.add_log(log).await;

    let result = AutoSwitchResult {
        triggered: true,
        current_email: Some(current_email),
        current_usage_percent: Some(usage_percent),
        current_remaining: Some(remaining_percent),
        switched_to_email: Some(target_email),
        switched_to_remaining: target_remaining,
        reason: None,
    };
    let _ = app.emit("auto-switch-result", &result);

    Ok(serde_json::to_value(&result).unwrap_or(json!({"triggered": true})))
}

/// 查询账号配额，返回 `(usage_percent, remaining_percent)`，单位均为 0-100 百分比
///
/// 薄包装：调用 `WindsurfService::query_quota_summary` 公用方法，兼容两套计费体系。
/// 详细字段说明见 `QuotaSummary`。
async fn query_account_quota(
    service: &WindsurfService,
    ctx: &AuthContext,
) -> Result<(i32, i32), String> {
    let summary = service
        .query_quota_summary(ctx)
        .await
        .map_err(|e| e.to_string())?;
    Ok((summary.usage_percent, summary.remaining_percent))
}

/// 计算账号的剩余配额百分比（0-100），用于 `pick_next_account` 的过滤和排序
///
/// 与 `query_account_quota` 的逻辑对齐，但读取的是 `Account` 模型里**已缓存**的字段
/// （账号刷新时由 `refresh_account_info` / `login_account` 更新），不发网络请求。
///
/// ## 优先级
/// 1. **QUOTA 模式**：取 `daily_quota_remaining_percent` / `weekly_quota_remaining_percent` 中较小者
/// 2. **CREDIT 模式（fallback）**：从 `used_quota` / `total_quota` 换算
/// 3. **无数据**：返回 -1（调用方据此过滤掉无配额数据的账号）
fn account_remaining_percent(acc: &Account) -> i32 {
    match (acc.daily_quota_remaining_percent, acc.weekly_quota_remaining_percent) {
        (Some(d), Some(w)) => d.min(w).clamp(0, 100),
        (Some(d), None) => d.clamp(0, 100),
        (None, Some(w)) => w.clamp(0, 100),
        (None, None) => match (acc.used_quota, acc.total_quota) {
            (Some(u), Some(t)) if t > 0 => {
                let usage = ((u as f64 / t as f64) * 100.0).round() as i32;
                (100 - usage).clamp(0, 100)
            }
            _ => -1,
        },
    }
}

/// 根据策略从候选池中挑选下一个目标账号
///
/// ### 过滤规则
/// 1. 不能是当前账号本身
/// 2. 不能是 `is_disabled = Some(true)`
/// 3. 必须有 `token`（refresh_token 二选一即可，但 token 是后续 perform_account_switch 必需）
/// 4. 必须有配额数据（QUOTA 字段或 CREDIT 字段二选一，否则跳过）
/// 5. 剩余配额百分比必须 > `min_remaining_percent`（防止刚好够阈值的账号也被选上）
///
/// ### 排序规则
/// - `most_remaining`（默认）：剩余配额百分比降序 → 最富的优先
/// - `round_robin`：按 `last_login_at` 升序 → 最久没用的优先（轮换）
///
/// ### `prefer_same_provider`
/// 启用时优先返回与当前账号同认证体系（Firebase/Devin）的账号，
/// 避免不必要的认证链路切换。如果同体系无候选则降级到全部候选。
///
/// ### 单位说明
/// `min_remaining_percent` 为 0-100 百分比单位，与 `query_account_quota` 返回值对齐。
/// 例如设为 5 表示"剩余 ≤ 5% 的账号不能被选为目标"。
fn pick_next_account(
    all_accounts: &[Account],
    current: &Account,
    prefer_same_provider: bool,
    strategy: &str,
    min_remaining_percent: i32,
) -> Option<Account> {
    let current_is_devin = current.is_devin_account();

    let base_filter = |acc: &&Account| -> bool {
        // 排除自己
        if acc.id == current.id {
            return false;
        }
        // 排除被 Windsurf 禁用的账号
        if acc.is_disabled == Some(true) {
            return false;
        }
        // 必须有 token（perform_account_switch 内会再校验 refresh_token，但这里先粗筛）
        let has_token = acc.token.as_ref().map(|t| !t.is_empty()).unwrap_or(false)
            || acc
                .refresh_token
                .as_ref()
                .map(|t| !t.is_empty())
                .unwrap_or(false);
        if !has_token {
            return false;
        }
        // 必须有配额数据（QUOTA 或 CREDIT 任一），并且剩余百分比严格大于阈值
        let remaining_pct = account_remaining_percent(acc);
        if remaining_pct < 0 {
            return false; // 没配额数据
        }
        if min_remaining_percent > 0 {
            remaining_pct > min_remaining_percent
        } else {
            remaining_pct > 0
        }
    };

    // 同体系优先：先在同 provider 中挑，没有再扩大范围
    let provider_filter = |acc: &&Account| -> bool {
        if !prefer_same_provider {
            return true;
        }
        acc.is_devin_account() == current_is_devin
    };

    let pick_from = |candidates: Vec<Account>| -> Option<Account> {
        let mut pool = candidates;
        match strategy {
            "round_robin" => {
                pool.sort_by(|a, b| match (a.last_login_at, b.last_login_at) {
                    (Some(la), Some(lb)) => la.cmp(&lb),
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                });
            }
            // 默认 "most_remaining"：按剩余百分比降序
            _ => {
                pool.sort_by(|a, b| {
                    let ra = account_remaining_percent(a);
                    let rb = account_remaining_percent(b);
                    rb.cmp(&ra)
                });
            }
        }
        pool.into_iter().next()
    };

    // 第一轮：同体系
    let same_provider: Vec<Account> = all_accounts
        .iter()
        .filter(base_filter)
        .filter(provider_filter)
        .cloned()
        .collect();

    if let Some(account) = pick_from(same_provider) {
        return Some(account);
    }

    // 第二轮（仅 prefer_same_provider=true 时）：放宽到所有 provider
    if prefer_same_provider {
        let all_candidates: Vec<Account> = all_accounts.iter().filter(base_filter).cloned().collect();
        return pick_from(all_candidates);
    }

    None
}
