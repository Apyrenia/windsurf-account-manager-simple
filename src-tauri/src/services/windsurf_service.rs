use crate::services::auth_context::{AuthContext, AuthHeaderExt};
use crate::utils::{AppError, AppResult};
use base64::{Engine, engine::general_purpose};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

const WINDSURF_BASE_URL: &str = "https://web-backend.windsurf.com";

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSeatsResult {
    pub success: bool,
    pub attempts: Vec<AttemptResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttemptResult {
    pub attempt: usize,
    pub status_code: Option<u16>,
    pub raw_response: Option<String>,
    pub error: Option<String>,
    pub timestamp: String,
}

/// 标准化的账号配额信息，兼容新旧两套计费体系（QUOTA / CREDIT）
///
/// 由 `WindsurfService::query_quota_summary` 返回，是所有上层调用方
/// （`auto_switch`、`auto_reset`、未来的额度监控等）的**唯一**配额数据入口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSummary {
    /// 已使用百分比，0-100
    pub usage_percent: i32,
    /// 剩余百分比，0-100；QUOTA 模式下取日/周中较小者
    pub remaining_percent: i32,
    /// 已使用估算值（CREDIT 单位，10000 满分制）
    /// - 旧版 CREDIT 模式：返回真实积分数
    /// - 新版 QUOTA 模式：用 `usage_percent * 100` 模拟，保持旧代码的 *100 scaling 兼容
    pub used_estimate: i64,
    /// 总额度估算值（CREDIT 单位）
    /// - 旧版 CREDIT 模式：返回真实总积分
    /// - 新版 QUOTA 模式：固定 10000
    pub total_estimate: i64,
    /// 日配额剩余百分比（仅 QUOTA 模式有值）
    pub daily_remaining_percent: Option<i32>,
    /// 周配额剩余百分比（仅 QUOTA 模式有值）
    pub weekly_remaining_percent: Option<i32>,
    /// 计费模式标识：`"quota"` | `"credit"`，调试用
    pub billing_mode: String,
    /// 原始 plan_status JSON（保留给需要其他字段的调用方）
    pub raw_plan_status: serde_json::Value,
}


pub struct WindsurfService {
    client: Arc<reqwest::Client>,
}

impl WindsurfService {
    pub fn new() -> Self {
        // 使用全局共享的 HTTP 客户端，避免每次请求都创建新实例
        Self {
            client: super::get_http_client(),
        }
    }

    fn build_request_body(&self, token: &str, seat_count: i32) -> Vec<u8> {
        // UpdateSeats的body格式: 0x0a + token长度(varint) + token + 0x10 + seat_count
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        
        let mut body = vec![0x0a];
        
        // Token长度（使用varint编码）
        if token_length < 128 {
            body.push(token_length as u8);
        } else {
            // 对于JWT token（通常>1000字节），需要两字节的varint
            body.push(((token_length & 0x7F) | 0x80) as u8);
            body.push((token_length >> 7) as u8);
        }
        
        // Token内容
        body.extend_from_slice(token_bytes);
        
        // 座位数（field 2, varint）
        body.push(0x10);
        body.push(seat_count as u8);
        
        body
    }

    /// 构建更新计划请求体
    /// 
    /// Protobuf 结构 (UpdatePlanRequest):
    /// - Field 1 (LengthDelimited): auth_token (string)
    /// - Field 2 (Varint): price (StripePrice enum)
    /// - Field 3 (Varint): preview (bool) - 预览模式
    /// - Field 4 (Varint): payment_period (PaymentPeriod enum: 1=月付, 2=年付)
    /// - Field 5 (Varint): teams_tier (TeamsTier enum: 1-11)
    fn build_update_plan_body(&self, token: &str, plan_type: &str, payment_period: u8, preview: bool) -> Vec<u8> {
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();

        let mut body = vec![0x0a];

        // Token长度（使用varint编码）
        if token_length < 128 {
            body.push(token_length as u8);
        } else {
            body.push(((token_length & 0x7F) | 0x80) as u8);
            body.push((token_length >> 7) as u8);
        }

        body.extend_from_slice(token_bytes);
        
        // Field 2: price (StripePrice)
        // 1 = STRIPE_PRICE_TEAMS_MONTHLY (月付价格)
        // 2 = STRIPE_PRICE_TEAMS_YEARLY (年付价格)
        body.push(0x10);
        body.push(if payment_period == 2 { 0x02 } else { 0x01 });
        
        // Field 3: preview (bool) - 0x18 = field 3 varint
        if preview {
            body.push(0x18);
            body.push(0x01);
        }
        
        // Field 4: payment_period (0x20 = field 4 varint)
        // 1 = PAYMENT_PERIOD_MONTH (月付)
        // 2 = PAYMENT_PERIOD_YEAR (年付)
        body.push(0x20);
        body.push(if payment_period == 2 { 0x02 } else { 0x01 });
        
        // Field 5: teams_tier (0x28 = field 5 varint)
        body.push(0x28);

        // 根据订阅类型添加不同的后缀字节 (TeamsTier枚举值)
        match plan_type.to_lowercase().as_str() {
            "free" => body.push(0x00),                     // 0 = TEAMS_TIER_UNSPECIFIED (Free)
            "teams" => body.push(0x01),                    // 1 = TEAMS_TIER_TEAMS
            "pro" => body.push(0x02),                      // 2 = TEAMS_TIER_PRO
            "enterprise_saas" => body.push(0x03),          // 3 = TEAMS_TIER_ENTERPRISE_SAAS
            "hybrid" => body.push(0x04),                   // 4 = TEAMS_TIER_HYBRID
            "enterprise_self_hosted" => body.push(0x05),   // 5 = TEAMS_TIER_ENTERPRISE_SELF_HOSTED
            "waitlist_pro" => body.push(0x06),             // 6 = TEAMS_TIER_WAITLIST_PRO
            "teams_ultimate" => body.push(0x07),           // 7 = TEAMS_TIER_TEAMS_ULTIMATE
            "pro_ultimate" => body.push(0x08),             // 8 = TEAMS_TIER_PRO_ULTIMATE
            "trial" => body.push(0x09),                    // 9 = TEAMS_TIER_TRIAL
            "enterprise_self_serve" => body.push(0x0a),    // 10 = TEAMS_TIER_ENTERPRISE_SELF_SERVE
            "enterprise_saas_pooled" => body.push(0x0b),   // 11 = TEAMS_TIER_ENTERPRISE_SAAS_POOLED
            "devin_enterprise" => body.push(0x0c),         // 12 = TEAMS_TIER_DEVIN_ENTERPRISE
            "devin_teams" => body.push(0x0e),              // 14 = TEAMS_TIER_DEVIN_TEAMS
            "devin_teams_v2" => body.push(0x0f),           // 15 = TEAMS_TIER_DEVIN_TEAMS_V2
            "devin_pro" => body.push(0x10),                // 16 = TEAMS_TIER_DEVIN_PRO
            "devin_max" => body.push(0x11),                // 17 = TEAMS_TIER_DEVIN_MAX
            "max" => body.push(0x12),                      // 18 = TEAMS_TIER_MAX
            "devin_free" => body.push(0x13),               // 19 = TEAMS_TIER_DEVIN_FREE
            "devin_trial" => body.push(0x14),              // 20 = TEAMS_TIER_DEVIN_TRIAL
            "enterprise" | _ => body.push(0x0a),           // 默认使用 ENTERPRISE_SELF_SERVE
        }

        body
    }

    /// 构建取消订阅请求体
    ///
    /// Protobuf 结构：
    /// - Field 1 (LengthDelimited): Firebase ID Token
    /// - Field 2 (Varint): 1 (表示取消操作)
    /// - Field 5 (LengthDelimited): 取消原因字符串
    fn build_cancel_plan_body(&self, token: &str, reason: &str) -> Vec<u8> {
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        let reason_bytes = reason.as_bytes();
        let reason_length = reason_bytes.len();

        let mut body = vec![0x0a]; // Field 1, wire type 2 (LengthDelimited)

        // Token长度（使用varint编码）
        if token_length < 128 {
            body.push(token_length as u8);
        } else {
            body.push(((token_length & 0x7F) | 0x80) as u8);
            body.push((token_length >> 7) as u8);
        }

        // Token内容
        body.extend_from_slice(token_bytes);

        // Field 2: int32 = 1 (表示取消操作)
        body.push(0x10); // Field 2, wire type 0 (Varint)
        body.push(0x01); // value = 1

        // Field 5: 取消原因字符串
        body.push(0x2a); // Field 5, wire type 2 (LengthDelimited)

        // 原因字符串长度
        if reason_length < 128 {
            body.push(reason_length as u8);
        } else {
            body.push(((reason_length & 0x7F) | 0x80) as u8);
            body.push((reason_length >> 7) as u8);
        }

        // 原因字符串内容
        body.extend_from_slice(reason_bytes);

        body
    }

    /// 构建恢复订阅请求体
    ///
    /// Protobuf 结构：
    /// - Field 1 (LengthDelimited): Firebase ID Token
    /// - Field 3 (Varint): 1 (表示恢复操作)
    fn build_resume_plan_body(&self, token: &str) -> Vec<u8> {
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();

        let mut body = vec![0x0a]; // Field 1, wire type 2 (LengthDelimited)

        // Token长度（使用varint编码）
        if token_length < 128 {
            body.push(token_length as u8);
        } else {
            body.push(((token_length & 0x7F) | 0x80) as u8);
            body.push((token_length >> 7) as u8);
        }

        // Token内容
        body.extend_from_slice(token_bytes);

        // Field 3: int32 = 1 (表示恢复操作)
        body.push(0x18); // Field 3, wire type 0 (Varint)
        body.push(0x01); // value = 1

        body
    }

    fn build_subscribe_to_plan_body(
        &self, 
        token: &str, 
        success_url: &str, 
        cancel_url: &str, 
        teams_tier: i32,
        payment_period: i32,
        start_trial: bool,
        team_name: Option<&str>,
        seats: Option<i32>,
        turnstile_token: Option<&str>
    ) -> Vec<u8> {
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        let success_url_bytes = success_url.as_bytes();
        let success_url_length = success_url_bytes.len();
        let cancel_url_bytes = cancel_url.as_bytes();
        let cancel_url_length = cancel_url_bytes.len();

        let mut body = Vec::new();

        // 字段1: auth_token (string, field number 1, wire type 2)
        body.push(0x0a); // field 1, wire type 2 (length-delimited)
        let mut len = token_length;
        while len >= 0x80 {
            body.push(((len & 0x7F) | 0x80) as u8);
            len >>= 7;
        }
        body.push(len as u8);
        body.extend_from_slice(token_bytes);

        // 字段3: start_trial (bool, field number 3, wire type 0)
        if start_trial {
            body.push(0x18); // field 3, wire type 0 (0x18 = (3 << 3) | 0)
            body.push(0x01); // value = true
        }

        // 字段4: Success URL (string, field number 4, wire type 2)
        body.push(0x22); // field 4, wire type 2 (0x22 = (4 << 3) | 2)
        body.push(success_url_length as u8);
        body.extend_from_slice(success_url_bytes);

        // 字段5: Cancel URL (string, field number 5, wire type 2)
        body.push(0x2a); // field 5, wire type 2 (0x2a = (5 << 3) | 2)
        body.push(cancel_url_length as u8);
        body.extend_from_slice(cancel_url_bytes);

        // 字段6: seats (int64, field number 6, wire type 0)
        // 所有团队/企业类计划需要 seats，个人计划(Pro/Max/Trial/Free等)不设置
        if matches!(teams_tier, 1 | 3 | 4 | 5 | 7 | 10 | 11 | 12 | 14 | 15) {
            let seat_count = seats.unwrap_or(1);
            if seat_count > 0 {
                body.push(0x30); // field 6, wire type 0 (0x30 = (6 << 3) | 0)
                body.push(seat_count as u8);
            }
        }

        // 字段7: team_name (string, field number 7, wire type 2) - Teams/Enterprise 需要
        if let Some(name) = team_name {
            if !name.is_empty() {
                let name_bytes = name.as_bytes();
                body.push(0x3a); // field 7, wire type 2 (0x3a = (7 << 3) | 2)
                body.push(name_bytes.len() as u8);
                body.extend_from_slice(name_bytes);
            }
        }

        // 字段8: teams_tier (enum, field number 8, wire type 0)
        body.push(0x40); // field 8, wire type 0 (varint)
        body.push(teams_tier as u8);

        // 字段9: payment_period (enum, field number 9, wire type 0)
        body.push(0x48); // field 9, wire type 0 (varint)
        body.push(payment_period as u8);

        // 字段10: turnstile_token (string, field number 10, wire type 2) - start_trial=true 时所有计划均需
        if let Some(turnstile) = turnstile_token {
            let turnstile_bytes = turnstile.as_bytes();
            body.push(0x52); // field 10, wire type 2 (0x52 = (10 << 3) | 2)
            let mut tlen = turnstile_bytes.len();
            while tlen >= 0x80 {
                body.push(((tlen & 0x7F) | 0x80) as u8);
                tlen >>= 7;
            }
            body.push(tlen as u8);
            body.extend_from_slice(turnstile_bytes);
        }

        body
    }

    pub async fn update_seats(&self, ctx: &AuthContext, seat_count: i32, retry_times: i32) -> AppResult<UpdateSeatsResult> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/UpdateSeats", WINDSURF_BASE_URL);
        
        let mut attempts = Vec::new();
        let mut success = false;
        
        for i in 0..retry_times {
            let body = self.build_request_body(token, seat_count);
            
            let result = self.client
                .post(&url)
                .body(body)
                .header("accept", "*/*")
                .header("accept-language", "zh-CN,zh;q=0.9")
                .header("cache-control", "no-cache")
                .header("connect-protocol-version", "1")
                .header("content-type", "application/proto")
                .header("pragma", "no-cache")
                .header("priority", "u=1, i")
                .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
                .header("sec-ch-ua-mobile", "?0")
                .header("sec-ch-ua-platform", r#""Windows""#)
                .header("sec-fetch-dest", "empty")
                .header("sec-fetch-mode", "cors")
                .header("sec-fetch-site", "same-site")
                .with_auth(ctx)
                .header("x-debug-email", "")
                .header("x-debug-team-name", "")
                .header("Referer", "https://windsurf.com/")
                .send()
                .await;
            
            match result {
                Ok(response) => {
                    let status_code = response.status().as_u16();
                    let response_bytes = response.bytes().await.unwrap_or_default();
                    
                    // 尝试解析响应
                    let mut raw_response = String::from_utf8_lossy(&response_bytes).to_string();
                    let mut parsed_data = None;
                    
                    // 200 或 204 都表示成功
                    if status_code == 200 || status_code == 204 {
                        // 尝试解析Protobuf响应
                        if response_bytes.len() > 0 {
                            match crate::services::proto_parser::ProtobufParser::parse_update_seats_response(&response_bytes) {
                                Ok(parsed) => {
                                    println!("[UpdateSeats] Successfully parsed response: {:?}", parsed);
                                    parsed_data = Some(parsed.clone());
                                    
                                    // 检查解析后的成功状态
                                    if let Some(parsed_success) = parsed.get("success").and_then(|v| v.as_bool()) {
                                        success = parsed_success;
                                    } else {
                                        success = true; // 如果没有明确的失败标志，视为成功
                                    }
                                    
                                    // 构造更详细的响应
                                    raw_response = parsed.to_string();
                                },
                                Err(e) => {
                                    println!("[UpdateSeats] Failed to parse response: {}", e);
                                    // 解析失败但状态码是200/204，仍视为成功
                                    success = true;
                                }
                            }
                        } else {
                            success = true; // 204 No Content
                        }
                    }
                    
                    // 构造尝试结果
                    let mut attempt_result = AttemptResult {
                        attempt: i as usize + 1,
                        status_code: Some(status_code),
                        raw_response: Some(raw_response),
                        error: None,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };
                    
                    // 如果有解析数据，添加到结果中
                    if let Some(data) = parsed_data {
                        // 将解析的数据作为JSON字符串存储
                        if let Ok(json_str) = serde_json::to_string_pretty(&data) {
                            attempt_result.raw_response = Some(json_str);
                        }
                    }
                    
                    attempts.push(attempt_result);
                    
                    // 如果成功，直接返回，不需要继续重试
                    if success {
                        break;
                    }
                },
                Err(e) => {
                    attempts.push(AttemptResult {
                        attempt: i as usize + 1,
                        status_code: None,
                        raw_response: None,
                        error: Some(e.to_string()),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
            
            // 两次请求之间稍作延迟
            if i < retry_times - 1 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
        
        Ok(UpdateSeatsResult {
            success,
            attempts,
        })
    }

    pub async fn get_team_credit_entries(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetTeamCreditEntries", WINDSURF_BASE_URL);
        
        // GetTeamCreditEntries的body格式: 0x0a + token长度 + token
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        
        let mut full_body = vec![0x0a];
        
        // Token长度（使用varint编码）
        if token_length < 128 {
            full_body.push(token_length as u8);
        } else {
            full_body.push(((token_length & 0x7F) | 0x80) as u8);
            full_body.push((token_length >> 7) as u8);
        }
        
        full_body.extend_from_slice(token_bytes);
        
        println!("[GetTeamCreditEntries] Sending request to {}", url);
        println!("[GetTeamCreditEntries] Token length: {} bytes", token_length);
        println!("[GetTeamCreditEntries] Request body length: {} bytes", full_body.len());
        
        // 打印前几个字节用于调试
        if full_body.len() >= 3 {
            println!("[GetTeamCreditEntries] Body prefix: {:02x} {:02x} {:02x}", full_body[0], full_body[1], full_body[2]);
        }
        
        let result = self.client
            .post(&url)
            .body(full_body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .with_auth(ctx)
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await;
        
        match result {
            Ok(response) => {
                let status_code = response.status().as_u16();
                println!("[GetTeamCreditEntries] Response status: {}", status_code);
                
                let response_bytes = response.bytes().await.unwrap_or_default();
                println!("[GetTeamCreditEntries] Response size: {} bytes", response_bytes.len());
                
                if status_code == 200 {
                    // 空响应可能表示没有积分记录
                    if response_bytes.len() == 0 {
                        println!("[GetTeamCreditEntries] Empty response - no credit entries found");
                        return Ok(json!({
                            "success": true,
                            "entries": [],
                            "total_entries": 0,
                            "message": "该团队暂无积分记录"
                        }));
                    }
                    // 打印响应的前100个字节用于调试
                    let preview = if response_bytes.starts_with(b"data:application/proto;base64,") {
                        "Base64 encoded response"
                    } else {
                        "Binary response"
                    };
                    println!("[GetTeamCreditEntries] Response format: {}", preview);
                    
                    // 尝试解析Protobuf响应
                    match crate::services::proto_parser::ProtobufParser::parse_get_team_credit_entries_response(&response_bytes) {
                        Ok(parsed) => {
                            println!("[GetTeamCreditEntries] Successfully parsed credit entries response");
                            println!("[GetTeamCreditEntries] Total entries: {}", 
                                parsed.get("total_entries").and_then(|v| v.as_i64()).unwrap_or(0));
                            Ok(parsed)
                        },
                        Err(e) => {
                            println!("[GetTeamCreditEntries] Failed to parse response: {}", e);
                            // 返回原始响应以便调试
                            let raw_response = if response_bytes.starts_with(b"data:application/proto;base64,") {
                                String::from_utf8_lossy(&response_bytes).to_string()
                            } else {
                                format!("data:application/proto;base64,{}", general_purpose::STANDARD.encode(&response_bytes))
                            };
                            Ok(json!({
                                "success": false,
                                "error": format!("Parse error: {}", e),
                                "raw_response": raw_response
                            }))
                        }
                    }
                } else {
                    println!("[GetTeamCreditEntries] Unexpected status code: {}", status_code);
                    Ok(json!({
                        "success": false,
                        "status_code": status_code,
                        "error": format!("HTTP error: {}", status_code)
                    }))
                }
            },
            Err(e) => {
                println!("[GetTeamCreditEntries] Request failed: {}", e);
                Ok(json!({
                    "success": false,
                    "error": format!("Request failed: {}", e)
                }))
            }
        }
    }
    
    pub async fn get_team_billing(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetTeamBilling", WINDSURF_BASE_URL);
        
        // GetTeamBilling的body格式: 0x0a + token长度 + token
        // 注意：不是 0x0a 0xa1 0x07，那是UpdatePlan用的
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        
        let mut full_body = vec![0x0a];
        
        // Token长度（使用varint编码）
        if token_length < 128 {
            full_body.push(token_length as u8);
        } else {
            full_body.push(((token_length & 0x7F) | 0x80) as u8);
            full_body.push((token_length >> 7) as u8);
        }
        
        full_body.extend_from_slice(token_bytes);
        
        println!("[GetTeamBilling] Sending request to {}", url);
        
        let result = self.client
            .post(&url)
            .body(full_body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .with_auth(ctx)
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await;
        
        match result {
            Ok(response) => {
                let status_code = response.status().as_u16();
                println!("[GetTeamBilling] Response status: {}", status_code);
                
                let response_bytes = response.bytes().await.unwrap_or_default();
                println!("[GetTeamBilling] Response size: {} bytes", response_bytes.len());
                
                if status_code == 200 && response_bytes.len() > 0 {
                    // 尝试解析Protobuf响应
                    match crate::services::proto_parser::ProtobufParser::parse_get_team_billing_response(&response_bytes) {
                        Ok(parsed) => {
                            println!("[GetTeamBilling] Successfully parsed billing response");
                            Ok(parsed)
                        },
                        Err(e) => {
                            println!("[GetTeamBilling] Failed to parse response: {}", e);
                            Ok(json!({
                                "success": false,
                                "error": format!("Parse error: {}", e),
                                "raw_response": general_purpose::STANDARD.encode(&response_bytes)
                            }))
                        }
                    }
                } else {
                    Ok(json!({
                        "success": false,
                        "status_code": status_code,
                        "error": "Invalid response"
                    }))
                }
            },
            Err(e) => {
                println!("[GetTeamBilling] Request failed: {}", e);
                Ok(json!({
                    "success": false,
                    "error": e.to_string()
                }))
            }
        }
    }

    /// 更新订阅计划
    /// 
    /// # Arguments
    /// * `token` - Firebase ID Token
    /// * `plan_type` - 计划类型（teams, pro, enterprise 等）
    /// * `payment_period` - 付款周期（1=月付, 2=年付）
    /// * `preview` - 预览模式（true=仅预览不实际执行）
    pub async fn update_plan(&self, ctx: &AuthContext, plan_type: &str, payment_period: u8, preview: bool) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/UpdatePlan", WINDSURF_BASE_URL);
        
        // 验证 payment_period
        let period = if payment_period == 2 { 2 } else { 1 };
        let period_name = if period == 2 { "年付" } else { "月付" };
        
        println!("[UpdatePlan] plan_type={}, period={}, preview={}", plan_type, period_name, preview);
        
        let body = self.build_update_plan_body(token, plan_type, period, preview);
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await?;
        
        let status_code = response.status().as_u16();
        let response_bytes = response.bytes().await.unwrap_or_default();
        
        println!("[UpdatePlan] Response status: {}, size: {} bytes", status_code, response_bytes.len());
        
        // 尝试解析 Protobuf 响应
        if status_code == 200 && response_bytes.len() > 0 {
            match crate::services::proto_parser::ProtobufParser::parse_update_plan_response(&response_bytes) {
                Ok(parsed) => {
                    println!("[UpdatePlan] Successfully parsed response");
                    
                    // 检查是否有支付失败原因
                    let payment_failure = parsed.get("payment_failure_reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    
                    let applied_changes = parsed.get("applied_changes")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    
                    return Ok(serde_json::json!({
                        "success": payment_failure.is_empty() && (preview || applied_changes),
                        "preview": preview,
                        "plan_type": plan_type,
                        "payment_period": period,
                        "payment_period_name": period_name,
                        "status_code": status_code,
                        "applied_changes": applied_changes,
                        "payment_failure_reason": if payment_failure.is_empty() { None } else { Some(payment_failure) },
                        "billing_update": parsed.get("billing_update"),
                        "requires_password_reset": parsed.get("requires_password_reset"),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }));
                },
                Err(e) => {
                    println!("[UpdatePlan] Failed to parse response: {}", e);
                }
            }
        }
        
        // 解析失败时返回原始响应
        let raw_response = if response_bytes.starts_with(b"data:application/proto;base64,") {
            String::from_utf8_lossy(&response_bytes).to_string()
        } else {
            format!("data:application/proto;base64,{}", general_purpose::STANDARD.encode(&response_bytes))
        };
        
        Ok(serde_json::json!({
            "success": status_code == 200,
            "preview": preview,
            "plan_type": plan_type,
            "payment_period": period,
            "payment_period_name": period_name,
            "status_code": status_code,
            "raw_response": raw_response,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// 取消订阅
    ///
    /// # Arguments
    /// * `token` - Firebase ID Token
    /// * `reason` - 取消原因（例如："too_expensive", "not_using", "missing_features", "switching_service", "other"）
    ///
    /// # Returns
    /// 返回包含操作结果的 JSON 对象
    pub async fn cancel_plan(&self, ctx: &AuthContext, reason: &str) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/CancelPlan", WINDSURF_BASE_URL);

        println!("[CancelPlan] Canceling subscription with reason: {}", reason);

        let body = self.build_cancel_plan_body(token, reason);

        println!("[CancelPlan] Request body length: {} bytes", body.len());
        println!("[CancelPlan] Request body hex: {}", body.iter().map(|b| format!("{:02x}", b)).collect::<String>());

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("x-api-key", token)
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await?;

        let status_code = response.status().as_u16();
        let response_bytes = response.bytes().await.unwrap_or_default();
        let response_text = String::from_utf8_lossy(&response_bytes).to_string();

        println!("[CancelPlan] Response status: {}", status_code);
        println!("[CancelPlan] Response length: {} bytes", response_bytes.len());

        Ok(serde_json::json!({
            "success": status_code == 200,
            "reason": reason,
            "status_code": status_code,
            "raw_response": response_text,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// 恢复订阅
    ///
    /// # Arguments
    /// * `token` - Firebase ID Token
    ///
    /// # Returns
    /// 返回包含操作结果的 JSON 对象
    pub async fn resume_plan(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/CancelPlan", WINDSURF_BASE_URL);

        println!("[ResumePlan] Resuming subscription");

        let body = self.build_resume_plan_body(token);

        println!("[ResumePlan] Request body length: {} bytes", body.len());
        println!("[ResumePlan] Request body hex: {}", body.iter().map(|b| format!("{:02x}", b)).collect::<String>());

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("x-api-key", token)
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await?;

        let status_code = response.status().as_u16();
        let response_bytes = response.bytes().await.unwrap_or_default();
        let response_text = String::from_utf8_lossy(&response_bytes).to_string();

        println!("[ResumePlan] Response status: {}", status_code);
        println!("[ResumePlan] Response length: {} bytes", response_bytes.len());

        Ok(serde_json::json!({
            "success": status_code == 200,
            "status_code": status_code,
            "raw_response": response_text,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// 获取一次性 auth_token（供 Windsurf 桌面客户端 OAuth 回调登录使用）
    ///
    /// # 背景
    /// Windsurf 桌面客户端通过 `windsurf://codeium.windsurf#access_token=<one_time_auth_token>`
    /// 触发登录，该一次性票据由后端 `GetOneTimeAuthToken` RPC 颁发。
    ///
    /// # 鉴权兼容性
    /// - Firebase 账号：入参 `auth_token` = Firebase ID Token，请求仅需 `x-auth-token` 头
    /// - Devin 账号：入参 `auth_token` = `devin-session-token$...` 形式的 session_token；
    ///   请求还须附带 `X-Devin-Auth1-Token` / `X-Devin-Account-Id` /
    ///   `X-Devin-Primary-Org-Id` / `X-Devin-Session-Token` 4 个扩展头
    ///
    /// `with_auth(ctx)` 会根据 `ctx.devin` 自动分流，调用方无需感知具体账号体系。
    ///
    /// # Returns
    /// 成功时返回一次性 auth_token 字符串
    pub async fn get_one_time_auth_token(&self, ctx: &AuthContext) -> AppResult<String> {
        let token = ctx.token_str();
        let url = format!(
            "{}/exa.seat_management_pb.SeatManagementService/GetOneTimeAuthToken",
            WINDSURF_BASE_URL
        );

        // 请求体：GetOneTimeAuthTokenRequest { auth_token = 1 }
        let body = self.encode_string_field(1, token);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        println!(
            "[GetOneTimeAuthToken] Status: {}, Size: {} bytes",
            status_code,
            response_body.len()
        );

        if status_code != 200 {
            let err_text = String::from_utf8_lossy(&response_body).to_string();
            return Err(AppError::Api(format!(
                "GetOneTimeAuthToken 请求失败: status={}, body={}",
                status_code, err_text
            )));
        }

        // 响应体：GetOneTimeAuthTokenResponse { auth_token = 1 }
        let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
        let parsed = parser.parse_message()
            .map_err(|e| AppError::Api(format!("解析 GetOneTimeAuthToken 响应失败: {}", e)))?;

        let auth_token = parsed.get("string_1")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Api("GetOneTimeAuthToken 响应中未找到 auth_token 字段".to_string()))?;

        Ok(auth_token.to_string())
    }

    pub async fn get_current_user(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetCurrentUser", WINDSURF_BASE_URL);
        
        // 构建请求体：0x0a + token长度(varint) + token + 0x10 0x01 0x18 0x01 0x20 0x01
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        
        let mut body = vec![0x0a];
        
        // 添加varint编码的token长度
        if token_length < 128 {
            body.push(token_length as u8);
        } else {
            body.push(((token_length & 0x7F) | 0x80) as u8);
            body.push((token_length >> 7) as u8);
        }
        
        body.extend_from_slice(token_bytes);
        
        // 添加额外的字段
        body.extend_from_slice(&[0x10, 0x01, 0x18, 0x01, 0x20, 0x01]);
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .with_auth(ctx)
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await?;
        
        let status_code = response.status().as_u16();
        let response_body = response.bytes().await?;
        
        println!("[GetCurrentUser] Status code: {}", status_code);
        println!("[GetCurrentUser] Response size: {} bytes", response_body.len());
        
        if status_code == 200 {
            // 使用proto_parser解析响应
            match super::proto_parser::parse_get_current_user_response(&response_body) {
                Ok(parsed_result) => {
                    Ok(serde_json::json!({
                        "success": true,
                        "status_code": status_code,
                        "parsed_data": parsed_result["parsed_data"],
                        "user_info": parsed_result["user_info"],
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                },
                Err(parse_error) => {
                    // 解析失败，返回原始响应
                    let response_str = String::from_utf8_lossy(&response_body);
                    let base64_data = if response_str.starts_with("data:application/proto;base64,") {
                        &response_str[31..]
                    } else {
                        &response_str
                    };
                    
                    Ok(serde_json::json!({
                        "success": true,
                        "status_code": status_code,
                        "raw_response": base64_data.trim(),
                        "parse_error": parse_error,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                }
            }
        } else {
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "Failed to get current user",
                "raw_response": String::from_utf8_lossy(&response_body).to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取套餐状态（积分/配额信息）
    /// 比 GetCurrentUser 更轻量，专门用于刷新积分状态
    pub async fn get_plan_status(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetPlanStatus", WINDSURF_BASE_URL);
        
        // 构建请求体：GetPlanStatusRequest { auth_token = 1 }
        // 格式：0x0a + token长度(varint) + token
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        
        let mut body = vec![0x0a];
        
        // 添加varint编码的token长度
        if token_length < 128 {
            body.push(token_length as u8);
        } else {
            body.push(((token_length & 0x7F) | 0x80) as u8);
            body.push((token_length >> 7) as u8);
        }
        
        body.extend_from_slice(token_bytes);
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .with_auth(ctx)
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await?;
        
        let status_code = response.status().as_u16();
        let response_body = response.bytes().await?;
        
        println!("[GetPlanStatus] Status code: {}", status_code);
        println!("[GetPlanStatus] Response size: {} bytes", response_body.len());
        
        if status_code == 200 {
            // 使用proto_parser解析响应
            match super::proto_parser::ProtobufParser::parse_get_plan_status_response(&response_body) {
                Ok(parsed_result) => {
                    Ok(serde_json::json!({
                        "success": true,
                        "status_code": status_code,
                        "plan_status": parsed_result,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                },
                Err(parse_error) => {
                    // 解析失败，返回原始响应
                    let response_str = String::from_utf8_lossy(&response_body);
                    let base64_data = if response_str.starts_with("data:application/proto;base64,") {
                        &response_str[31..]
                    } else {
                        &response_str
                    };
                    
                    Ok(serde_json::json!({
                        "success": true,
                        "status_code": status_code,
                        "raw_response": base64_data.trim(),
                        "parse_error": parse_error,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                }
            }
        } else {
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "Failed to get plan status",
                "raw_response": String::from_utf8_lossy(&response_body).to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 统一查询账号配额，返回兼容新旧两套计费体系的标准化结果
    ///
    /// ## 设计目的
    ///
    /// Windsurf 经历过计费体系切换：
    /// - **旧版 CREDIT**（已废弃，新账号字段恒为 0）：`used_*_credits` / `available_*_credits`
    /// - **新版 QUOTA**（当前生效）：`daily_quota_remaining_percent` / `weekly_quota_remaining_percent`
    ///
    /// 历史上 `auto_switch` / `auto_reset` 各自手写解析逻辑，且都基于已废弃的 CREDIT 字段，
    /// 导致在新版账号上判断完全失效。本方法**统一**入口，所有上层只用 `QuotaSummary` 即可，
    /// 不再关心计费类型。
    ///
    /// ## 返回字段含义
    ///
    /// - `usage_percent` / `remaining_percent`：0-100 百分比，**新代码用这两个**
    /// - `used_estimate` / `total_estimate`：CREDIT 单位的估算值（10000 满分制），
    ///   旧版有真实值时返回真值；新版下用 `(usage_percent * 100, 10000)` 模拟，
    ///   **保证现有依赖 used/total + scaling 的旧代码（如 `auto_reset` 的 `remaining_threshold * 100`）
    ///   能继续工作**，无需 migration 用户配置
    /// - `billing_mode`：`"quota"` 或 `"credit"`，便于日志/调试
    ///
    /// ## QUOTA 模式聚合规则
    ///
    /// 同时有日/周配额时，取**剩余更少**者作为决策依据（即"最先耗尽"的口径，最保守）。
    pub async fn query_quota_summary(&self, ctx: &AuthContext) -> AppResult<QuotaSummary> {
        let plan_status = self.get_plan_status(ctx).await?;
        let ps = plan_status.get("plan_status").unwrap_or(&plan_status);

        // === 新版 QUOTA 模式优先 ===
        let daily_remaining = ps.get("daily_quota_remaining_percent").and_then(|v| v.as_i64());
        let weekly_remaining = ps
            .get("weekly_quota_remaining_percent")
            .and_then(|v| v.as_i64());

        if daily_remaining.is_some() || weekly_remaining.is_some() {
            let min_remaining = match (daily_remaining, weekly_remaining) {
                (Some(d), Some(w)) => d.min(w),
                (Some(d), None) => d,
                (None, Some(w)) => w,
                _ => 100,
            }
            .clamp(0, 100) as i32;
            let usage_percent = (100 - min_remaining).clamp(0, 100);
            // 估算字段：100% → 10000，0% → 0；保持 *100 scaling 兼容
            let used_estimate = (usage_percent * 100) as i64;
            let total_estimate = 10000_i64;
            return Ok(QuotaSummary {
                usage_percent,
                remaining_percent: min_remaining,
                used_estimate,
                total_estimate,
                daily_remaining_percent: daily_remaining.map(|v| v as i32),
                weekly_remaining_percent: weekly_remaining.map(|v| v as i32),
                billing_mode: "quota".to_string(),
                raw_plan_status: plan_status.clone(),
            });
        }

        // === 降级：旧版 CREDIT 模式 ===
        let used_prompt = ps.get("used_prompt_credits").and_then(|v| v.as_i64()).unwrap_or(0);
        let used_flex = ps.get("used_flex_credits").and_then(|v| v.as_i64()).unwrap_or(0);
        let available_flex = ps
            .get("available_flex_credits")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let available_prompt = ps
            .get("available_prompt_credits")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let used = used_prompt + used_flex;
        let total = available_flex + available_prompt;

        let (usage_percent, remaining_percent) = if total > 0 {
            let usage = ((used as f64 / total as f64) * 100.0).round().clamp(0.0, 100.0) as i32;
            (usage, (100 - usage).clamp(0, 100))
        } else {
            (0, 100)
        };

        Ok(QuotaSummary {
            usage_percent,
            remaining_percent,
            used_estimate: used,
            total_estimate: total,
            daily_remaining_percent: None,
            weekly_remaining_percent: None,
            billing_mode: "credit".to_string(),
            raw_plan_status: plan_status,
        })
    }

    pub async fn reset_credits(&self, ctx: &AuthContext, seat_count: Option<i32>, last_seat_count: Option<i32>, seat_count_options: &[i32]) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        // 确定使用的座位数
        let seat_count = if let Some(sc) = seat_count {
            sc
        } else if seat_count_options.is_empty() {
            // 如果没有配置选项，使用默认值
            18
        } else if let Some(last) = last_seat_count {
            // 根据配置的选项轮番切换
            if let Some(current_idx) = seat_count_options.iter().position(|&x| x == last) {
                // 找到当前座位数在选项中的位置，选择下一个
                let next_idx = (current_idx + 1) % seat_count_options.len();
                seat_count_options[next_idx]
            } else {
                // 如果上次使用的座位数不在选项中，使用第一个选项
                seat_count_options[0]
            }
        } else {
            // 没有上次记录，使用第一个选项
            seat_count_options[0]
        };
        
        println!("[ResetCredits] 使用座位数: {}", seat_count);
        
        // 执行一次座位更新即可触发积分重置
        let seats_result = self.update_seats(ctx, seat_count, 1).await?;
        
        // 直接返回座位更新的结果
        Ok(serde_json::json!({
            "success": seats_result.success,
            "seat_count_used": seat_count,
            "steps": {
                "update_seats": seats_result
            },
            "message": if seats_result.success {
                format!("积分重置成功，座位数已更新为{}", seat_count)
            } else {
                "积分重置失败".to_string()
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// 重置团队成员的积分
    /// 通过移除成员再重新邀请来重置积分（与团队管理中的逻辑一致）
    pub async fn reset_member_credits(&self, master_ctx: &AuthContext, member_api_key: &str, member_name: &str, member_email: &str) -> AppResult<serde_json::Value> {
        println!("[ResetMemberCredits] 开始重置成员积分: {} ({})", member_name, member_email);
        
        // Step 1: 移除成员
        let remove_result = self.remove_user_from_team(master_ctx, member_api_key).await;
        if let Err(e) = &remove_result {
            println!("[ResetMemberCredits] 移除成员失败: {}", e);
            return Ok(serde_json::json!({
                "success": false,
                "step": "remove",
                "error": format!("移除成员失败: {}", e),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
        }
        
        let remove_data = remove_result.unwrap();
        if !remove_data.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Ok(serde_json::json!({
                "success": false,
                "step": "remove",
                "error": "移除成员失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
        }
        
        println!("[ResetMemberCredits] 成员已移除，开始重新邀请...");
        
        // Step 2: 重新邀请
        let invite_result = self.grant_preapproval(master_ctx, vec![(member_name.to_string(), member_email.to_string())]).await;
        if let Err(e) = &invite_result {
            println!("[ResetMemberCredits] 重新邀请失败: {}", e);
            return Ok(serde_json::json!({
                "success": false,
                "step": "invite",
                "error": format!("重新邀请失败: {}", e),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
        }
        
        let invite_data = invite_result.unwrap();
        if !invite_data.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Ok(serde_json::json!({
                "success": false,
                "step": "invite",
                "error": "重新邀请失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
        }
        
        println!("[ResetMemberCredits] 成员积分重置成功: {}", member_email);
        
        Ok(serde_json::json!({
            "success": true,
            "message": format!("{} 积分已重置，等待接受邀请", member_name),
            "member_email": member_email,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// 获取试用绑卡链接
    ///
    /// # Arguments
    /// * `token` - JWT token
    /// * `teams_tier` - 团队等级: 1=Teams, 2=Pro, 3=Enterprise
    /// * `payment_period` - 支付周期: 1=月付, 2=年付
    /// * `team_name` - 团队名称 (仅 Teams/Enterprise 需要)
    /// * `seats` - 席位数量 (仅 Teams/Enterprise 需要)
    /// * `turnstile_token` - Turnstile 验证令牌 (start_trial=true 时所有计划均必需)
    ///
    /// # Returns
    /// 返回包含Stripe Checkout链接的JSON对象
    pub async fn subscribe_to_plan(
        &self, 
        ctx: &AuthContext, 
        teams_tier: i32,
        payment_period: i32,
        start_trial: bool,
        team_name: Option<&str>,
        seats: Option<i32>,
        turnstile_token: Option<&str>
    ) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/SubscribeToPlan", WINDSURF_BASE_URL);

        // 调试日志
        println!("[SubscribeToPlan] teams_tier={}, payment_period={}, start_trial={}, team_name={:?}, seats={:?}, has_turnstile={}", 
            teams_tier, payment_period, start_trial, team_name, seats, turnstile_token.is_some());

        // 根据计划类型设置回调URL
        let plan_tier_str = if teams_tier == 1 { "teams" } else { "pro" };
        let success_url = format!("https://windsurf.com/billing/payment-success?plan_tier={}", plan_tier_str);
        let cancel_url = format!("https://windsurf.com/plan?plan_cancelled=true&plan_tier={}", plan_tier_str);

        let body = self.build_subscribe_to_plan_body(
            token, 
            &success_url, 
            &cancel_url, 
            teams_tier,
            payment_period,
            start_trial,
            team_name,
            seats,
            turnstile_token
        );
        
        println!("[SubscribeToPlan] 请求体大小: {} bytes", body.len());

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("authorization", format!("Bearer {}", token))
            .with_auth(ctx)
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await?;
        
        println!("[SubscribeToPlan] 响应状态码: {}, 响应体大小: {} bytes", status_code, response_body.len());

        if status_code == 200 {
            // 响应直接是Protobuf二进制数据
            match super::proto_parser::ProtobufParser::new(response_body.to_vec()).parse_message() {
                Ok(parsed) => {
                    // 提取string_1字段（Stripe Checkout链接）
                    let stripe_url = parsed.get("string_1")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if !stripe_url.is_empty() {
                        return Ok(serde_json::json!({
                            "success": true,
                            "status_code": status_code,
                            "stripe_url": stripe_url,
                            "teams_tier": teams_tier,
                            "payment_period": payment_period,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }));
                    } else {
                        return Ok(serde_json::json!({
                            "success": false,
                            "status_code": status_code,
                            "error": "响应中未找到Stripe链接",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }));
                    }
                },
                Err(e) => {
                    return Ok(serde_json::json!({
                        "success": false,
                        "status_code": status_code,
                        "error": format!("解析响应失败: {}", e),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }));
                }
            }
        } else {
            let error_msg = String::from_utf8_lossy(&response_body).to_string();
            println!("[SubscribeToPlan] 错误响应: status={}, body={}", status_code, error_msg);

            // 解析错误信息，提供更友好的提示
            let friendly_error = if status_code == 400 {
                if error_msg.contains("invalid_argument") {
                    "请求参数错误，可能是价格ID无效或账号不支持此操作".to_string()
                } else if error_msg.contains("turnstile") || error_msg.contains("Turnstile") {
                    "Turnstile 验证失败，请重新验证".to_string()
                } else {
                    format!("请求格式错误: {}", error_msg)
                }
            } else if status_code == 401 || status_code == 403 {
                "认证失败，请先刷新Token后重试".to_string()
            } else if status_code == 404 {
                "API接口不存在".to_string()
            } else {
                format!("获取支付链接失败: {}", error_msg)
            };

            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": friendly_error,
                "error_details": error_msg,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取团队配置
    pub async fn get_team_config(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetTeamConfigRecord", WINDSURF_BASE_URL);

        // 构建请求体 (field 1 = auth_token)
        let mut body = Vec::new();
        let token_bytes = token.as_bytes();
        body.push(0x0A); // field 1, wire type 2 (length-delimited)
        // 写入长度
        let len = token_bytes.len();
        if len < 128 {
            body.push(len as u8);
        } else {
            body.push((len & 0x7F | 0x80) as u8);
            body.push((len >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            // 解析响应为通用格式
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|_| serde_json::json!({}));
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_text = String::from_utf8_lossy(&response_body).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取团队配置失败",
                "error_details": error_text,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 更新团队配置
    pub async fn update_team_config(&self, ctx: &AuthContext, config: serde_json::Value) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/UpdateTeamConfigExternal", WINDSURF_BASE_URL);

        // 构建请求体
        let mut body = Vec::new();
        
        // field 1 = auth_token
        let token_bytes = token.as_bytes();
        body.push(0x0A);
        // 写入长度
        let len = token_bytes.len();
        if len < 128 {
            body.push(len as u8);
        } else {
            body.push((len & 0x7F | 0x80) as u8);
            body.push((len >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);

        // 根据 config 添加各个字段
        // field 2 = allow_auto_run_commands (bool)
        if let Some(val) = config.get("allow_auto_run_commands").and_then(|v| v.as_bool()) {
            body.push(0x10); // field 2, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 3 = allow_mcp_servers (bool)
        if let Some(val) = config.get("allow_mcp_servers").and_then(|v| v.as_bool()) {
            body.push(0x18); // field 3, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 4 = allow_app_deployments (bool)
        if let Some(val) = config.get("allow_app_deployments").and_then(|v| v.as_bool()) {
            body.push(0x20); // field 4, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 5 = allow_github_reviews (bool)
        if let Some(val) = config.get("allow_github_reviews").and_then(|v| v.as_bool()) {
            body.push(0x28); // field 5, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 6 = allow_github_description_edits (bool)
        if let Some(val) = config.get("allow_github_description_edits").and_then(|v| v.as_bool()) {
            body.push(0x30); // field 6, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 10 = allow_conversation_sharing (bool)
        if let Some(val) = config.get("allow_conversation_sharing").and_then(|v| v.as_bool()) {
            body.push(0x50); // field 10, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 11 = allow_sandbox_app_deployments (bool)
        if let Some(val) = config.get("allow_sandbox_app_deployments").and_then(|v| v.as_bool()) {
            body.push(0x58); // field 11, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 12 = allow_teams_app_deployments (bool)
        if let Some(val) = config.get("allow_teams_app_deployments").and_then(|v| v.as_bool()) {
            body.push(0x60); // field 12, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 13 = allow_attribution (bool)
        if let Some(val) = config.get("allow_attribution").and_then(|v| v.as_bool()) {
            body.push(0x68); // field 13, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 9 = allow_individual_level_analytics (bool)
        if let Some(val) = config.get("allow_individual_level_analytics").and_then(|v| v.as_bool()) {
            body.push(0x48); // field 9, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 16 = allow_browser_experimental_features (bool)
        if let Some(val) = config.get("allow_browser_experimental_features").and_then(|v| v.as_bool()) {
            body.push(0x80); body.push(0x01); // field 16, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 17 = allow_vibe_and_replace (bool)
        if let Some(val) = config.get("allow_vibe_and_replace").and_then(|v| v.as_bool()) {
            body.push(0x88); body.push(0x01); // field 17, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 18 = disable_deepwiki (bool)
        if let Some(val) = config.get("disable_deepwiki").and_then(|v| v.as_bool()) {
            body.push(0x90); body.push(0x01); // field 18, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 19 = disable_codemaps (bool)
        if let Some(val) = config.get("disable_codemaps").and_then(|v| v.as_bool()) {
            body.push(0x98); body.push(0x01); // field 19, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }
        
        // field 20 = allow_codemap_sharing (string)
        if let Some(val) = config.get("allow_codemap_sharing").and_then(|v| v.as_str()) {
            let val_bytes = val.as_bytes();
            body.push(0xA2); body.push(0x01); // field 20, wire type 2
            let len = val_bytes.len();
            if len < 128 {
                body.push(len as u8);
            } else {
                body.push((len & 0x7F | 0x80) as u8);
                body.push((len >> 7) as u8);
            }
            body.extend_from_slice(val_bytes);
        }
        
        // field 21 = disable_fast_context (bool)
        if let Some(val) = config.get("disable_fast_context").and_then(|v| v.as_bool()) {
            body.push(0xA8); body.push(0x01); // field 21, wire type 0
            body.push(if val { 0x01 } else { 0x00 });
        }

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "团队配置已更新",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "更新团队配置失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取可用模型列表
    pub async fn get_cascade_model_configs(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.api_server_pb.ApiServerService/GetCascadeModelConfigsForSite", WINDSURF_BASE_URL);

        // 构建请求体 (field 6 = auth_token)
        let mut body = Vec::new();
        let token_bytes = token.as_bytes();
        body.push(0x32); // field 6, wire type 2 (length-delimited)
        let len = token_bytes.len();
        if len < 128 {
            body.push(len as u8);
        } else {
            body.push((len & 0x7F | 0x80) as u8);
            body.push((len >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            println!("[GetCascadeModelConfigs] Response size: {} bytes", response_body.len());
            
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|e| {
                println!("[GetCascadeModelConfigs] Parse error: {}", e);
                serde_json::json!({})
            });
            
            println!("[GetCascadeModelConfigs] Parsed keys: {:?}", parsed.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            println!("[GetCascadeModelConfigs] default_off_models_for_teams (int_3): {:?}", parsed.get("int_3"));
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            println!("[GetCascadeModelConfigs] Error status: {}", status_code);
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取模型配置失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取 Command 模型列表
    pub async fn get_command_model_configs(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.api_server_pb.ApiServerService/GetCommandModelConfigsForSite", WINDSURF_BASE_URL);

        // 构建请求体 (field 1 = auth_token)
        let mut body = Vec::new();
        let token_bytes = token.as_bytes();
        body.push(0x0A); // field 1, wire type 2 (length-delimited)
        let len = token_bytes.len();
        if len < 128 {
            body.push(len as u8);
        } else {
            body.push((len & 0x7F | 0x80) as u8);
            body.push((len >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            println!("[GetCommandModelConfigs] Response size: {} bytes", response_body.len());
            
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|e| {
                println!("[GetCommandModelConfigs] Parse error: {}", e);
                serde_json::json!({})
            });
            
            println!("[GetCommandModelConfigs] Parsed keys: {:?}", parsed.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            println!("[GetCommandModelConfigs] Error status: {}", status_code);
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取 Command 模型配置失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取团队模型控制配置
    pub async fn get_team_organizational_controls(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.api_server_pb.ApiServerService/GetTeamOrganizationalControlsForSite", WINDSURF_BASE_URL);

        // 构建请求体 (field 1 = auth_token)
        let mut body = Vec::new();
        let token_bytes = token.as_bytes();
        body.push(0x0A); // field 1, wire type 2
        let len = token_bytes.len();
        if len < 128 {
            body.push(len as u8);
        } else {
            body.push((len & 0x7F | 0x80) as u8);
            body.push((len >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            println!("[GetTeamOrganizationalControls] Response size: {} bytes", response_body.len());
            
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|e| {
                println!("[GetTeamOrganizationalControls] Parse error: {}", e);
                serde_json::json!({})
            });
            
            println!("[GetTeamOrganizationalControls] Parsed keys: {:?}", parsed.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            println!("[GetTeamOrganizationalControls] Full response: {}", serde_json::to_string_pretty(&parsed).unwrap_or_default());
            
            // 检查 subMesssage_1 (controls) 中的字段
            if let Some(controls) = parsed.get("subMesssage_1") {
                println!("[GetTeamOrganizationalControls] Controls keys: {:?}", controls.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                println!("[GetTeamOrganizationalControls] team_id (string_1): {:?}", controls.get("string_1"));
                println!("[GetTeamOrganizationalControls] cascade_model_labels (string_2): {:?}", controls.get("string_2"));
                println!("[GetTeamOrganizationalControls] command_model_labels (string_3): {:?}", controls.get("string_3"));
            }
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            println!("[GetTeamOrganizationalControls] Error status: {}", status_code);
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取团队模型配置失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 更新团队模型控制配置
    pub async fn upsert_team_organizational_controls(
        &self, 
        ctx: &AuthContext, 
        team_id: &str,
        cascade_models: Vec<String>,
        command_models: Vec<String>,
        extension_models: Vec<String>,
    ) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        println!("[UpsertTeamOrgControls] team_id={}, cascade={:?}, command={:?}, extension={:?}", 
            team_id, cascade_models, command_models, extension_models);
        
        // 验证 team_id 不为空
        if team_id.is_empty() {
            return Ok(serde_json::json!({
                "success": false,
                "error": "团队ID为空，无法保存模型配置",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
        }
        
        let url = format!("{}/exa.api_server_pb.ApiServerService/UpsertTeamOrganizationalControlsForSite", WINDSURF_BASE_URL);

        // 构建请求体
        let mut body = Vec::new();
        
        // field 1 = TeamOrganizationalControls (嵌套消息)
        let mut controls = Vec::new();
        
        // TeamOrganizationalControls.team_id (field 1)
        let team_id_bytes = team_id.as_bytes();
        controls.push(0x0A); // field 1, wire type 2
        controls.push(team_id_bytes.len() as u8);
        controls.extend_from_slice(team_id_bytes);
        
        // TeamOrganizationalControls.cascade_model_labels (field 2, repeated)
        for model in &cascade_models {
            let model_bytes = model.as_bytes();
            controls.push(0x12); // field 2, wire type 2
            controls.push(model_bytes.len() as u8);
            controls.extend_from_slice(model_bytes);
        }
        
        // TeamOrganizationalControls.command_model_labels (field 3, repeated)
        for model in &command_models {
            let model_bytes = model.as_bytes();
            controls.push(0x1A); // field 3, wire type 2
            controls.push(model_bytes.len() as u8);
            controls.extend_from_slice(model_bytes);
        }
        
        // TeamOrganizationalControls.extension_model_labels (field 6, repeated)
        for model in &extension_models {
            let model_bytes = model.as_bytes();
            controls.push(0x32); // field 6, wire type 2
            controls.push(model_bytes.len() as u8);
            controls.extend_from_slice(model_bytes);
        }
        
        // 写入 controls 到 body (field 1)
        body.push(0x0A); // field 1, wire type 2
        let controls_len = controls.len();
        if controls_len < 128 {
            body.push(controls_len as u8);
        } else if controls_len < 16384 {
            body.push((controls_len & 0x7F | 0x80) as u8);
            body.push((controls_len >> 7) as u8);
        } else {
            body.push((controls_len & 0x7F | 0x80) as u8);
            body.push(((controls_len >> 7) & 0x7F | 0x80) as u8);
            body.push((controls_len >> 14) as u8);
        }
        body.extend_from_slice(&controls);
        
        // field 2 = auth_token
        let token_bytes = token.as_bytes();
        body.push(0x12); // field 2, wire type 2
        let len = token_bytes.len();
        if len < 128 {
            body.push(len as u8);
        } else {
            body.push((len & 0x7F | 0x80) as u8);
            body.push((len >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "模型配置已更新",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "更新模型配置失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取可用的 MCP 插件列表
    pub async fn get_available_mcp_plugins(&self, api_key: &str) -> AppResult<serde_json::Value> {
        let url = format!("{}/exa.cascade_plugins_pb.CascadePluginsService/GetAvailableCascadePlugins", WINDSURF_BASE_URL);

        let request_body = serde_json::json!({
            "metadata": {
                "ideName": "windsurf",
                "extensionVersion": "1.0.0",
                "apiKey": api_key,
                "os": "unknown",
                "ideVersion": "1.3.7"
            }
        });

        let response = self.client
            .post(&url)
            .json(&request_body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/json")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.text().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            // 解析 JSON 响应
            let parsed: serde_json::Value = serde_json::from_str(&response_body)
                .unwrap_or_else(|_| serde_json::json!({}));
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取MCP插件列表失败",
                "error_details": response_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 删除用户 (DeleteUser API)
    /// DeleteUserRequest: auth_token=1, api_key=3
    pub async fn delete_user(&self, ctx: &AuthContext, api_key: &str) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/DeleteUser", WINDSURF_BASE_URL);

        // 构造 protobuf 请求体
        // field 1: auth_token (string)
        // field 3: api_key (string)
        let mut request_body = Vec::new();
        
        // Field 1: auth_token
        let token_bytes = token.as_bytes();
        request_body.push(0x0a); // field 1, wire type 2 (length-delimited)
        let token_len = token_bytes.len();
        if token_len < 128 {
            request_body.push(token_len as u8);
        } else {
            request_body.push((token_len & 0x7F | 0x80) as u8);
            request_body.push((token_len >> 7) as u8);
        }
        request_body.extend_from_slice(token_bytes);
        
        // Field 3: api_key
        let api_key_bytes = api_key.as_bytes();
        request_body.push(0x1a); // field 3, wire type 2 (length-delimited)
        let api_key_len = api_key_bytes.len();
        if api_key_len < 128 {
            request_body.push(api_key_len as u8);
        } else {
            request_body.push((api_key_len & 0x7F | 0x80) as u8);
            request_body.push((api_key_len >> 7) as u8);
        }
        request_body.extend_from_slice(api_key_bytes);

        log::info!("[DeleteUser] Request body size: {} bytes", request_body.len());

        let response = self.client
            .post(&url)
            .body(request_body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("sec-ch-ua", r#""Chromium";v="136", "Google Chrome";v="136", "Not_A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("x-debug-email", "")
            .header("x-debug-team-name", "")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        log::info!("[DeleteUser] Response status: {}", status_code);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "用户已删除",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            
            log::error!("[DeleteUser] Error: {}", error_body);
            
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "删除用户失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    // ==================== 团队成员管理 API ====================

    /// 辅助方法：编码 varint 长度的字符串字段
    fn encode_string_field(&self, field_num: u8, value: &str) -> Vec<u8> {
        let mut result = Vec::new();
        let bytes = value.as_bytes();
        let len = bytes.len();
        
        // field tag: (field_num << 3) | 2 (wire type 2 = length-delimited)
        result.push((field_num << 3) | 2);
        
        // varint length
        if len < 128 {
            result.push(len as u8);
        } else {
            result.push((len & 0x7F | 0x80) as u8);
            result.push((len >> 7) as u8);
        }
        
        result.extend_from_slice(bytes);
        result
    }

    /// 获取团队成员列表 (GetUsers API)
    /// 需要管理员权限
    pub async fn get_team_members(&self, ctx: &AuthContext, group_id: Option<&str>) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetUsers", WINDSURF_BASE_URL);
        
        let mut body = self.encode_string_field(1, token);
        
        // field 2: group_id (optional)
        if let Some(gid) = group_id {
            body.extend(self.encode_string_field(2, gid));
        }
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        println!("[GetTeamMembers] Status: {}, Size: {} bytes", status_code, response_body.len());

        if status_code == 200 && !response_body.is_empty() {
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|_| serde_json::json!({}));
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            // 错误响应透明化：打印并回传原始响应体，方便定位权限/参数/认证问题
            let raw_body_text = String::from_utf8_lossy(&response_body).to_string();
            println!(
                "[GetTeamMembers] 错误响应: status={}, body={}",
                status_code, raw_body_text
            );
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取团队成员失败",
                "raw_response": raw_body_text,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 邀请成员加入团队 (GrantPreapproval API)
    /// 需要管理员权限
    pub async fn grant_preapproval(&self, ctx: &AuthContext, users: Vec<(String, String)>) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GrantPreapproval", WINDSURF_BASE_URL);
        
        let mut body = self.encode_string_field(1, token);
        
        // field 2: repeated PreapprovalUserItem
        for (name, email) in &users {
            let mut item = Vec::new();
            item.extend(self.encode_string_field(1, name));
            item.extend(self.encode_string_field(2, email));
            
            // 嵌入消息: field 2, wire type 2
            body.push(0x12);
            let item_len = item.len();
            if item_len < 128 {
                body.push(item_len as u8);
            } else {
                body.push((item_len & 0x7F | 0x80) as u8);
                body.push((item_len >> 7) as u8);
            }
            body.extend(item);
        }
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        println!("[GrantPreapproval] Status: {}, Size: {} bytes", status_code, response_body.len());

        if status_code == 200 {
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|_| serde_json::json!({}));
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "invited_count": users.len(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_text = String::from_utf8_lossy(&response_body).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "邀请成员失败",
                "error_details": error_text,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 从团队中移除成员 (RemoveUserFromTeam API)
    /// 需要管理员权限
    pub async fn remove_user_from_team(&self, ctx: &AuthContext, api_key: &str) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/RemoveUserFromTeam", WINDSURF_BASE_URL);
        
        let mut body = self.encode_string_field(1, token);
        body.extend(self.encode_string_field(2, api_key));
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[RemoveUserFromTeam] Status: {}", status_code);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "成员已移除",
                "removed_api_key": api_key,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "移除成员失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 撤销预审批邀请 (RevokePreapproval API)
    /// 需要管理员权限
    pub async fn revoke_preapproval(&self, ctx: &AuthContext, approval_id: &str) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/RevokePreapproval", WINDSURF_BASE_URL);
        
        let mut body = self.encode_string_field(1, token);
        body.extend(self.encode_string_field(2, approval_id));
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[RevokePreapproval] Status: {}", status_code);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "邀请已撤销",
                "approval_id": approval_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "撤销邀请失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取所有待处理的预审批邀请 (GetPreapprovals API)
    /// 需要管理员权限
    pub async fn get_preapprovals(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetPreapprovals", WINDSURF_BASE_URL);
        
        let body = self.encode_string_field(1, token);
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        println!("[GetPreapprovals] Status: {}, Size: {} bytes", status_code, response_body.len());

        if status_code == 200 {
            if response_body.is_empty() {
                return Ok(serde_json::json!({
                    "success": true,
                    "preapprovals": [],
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }));
            }
            
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|_| serde_json::json!({}));
            
            Ok(serde_json::json!({
                "success": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取预审批列表失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取当前用户的待处理邀请 (GetPreapprovalForUser API)
    /// 普通用户权限
    pub async fn get_preapproval_for_user(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetPreapprovalForUser", WINDSURF_BASE_URL);
        
        let body = self.encode_string_field(1, token);
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        println!("[GetPreapprovalForUser] Status: {}, Size: {} bytes", status_code, response_body.len());

        if status_code == 200 {
            if response_body.is_empty() {
                return Ok(serde_json::json!({
                    "success": true,
                    "has_pending_invitation": false,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }));
            }
            
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|_| serde_json::json!({}));
            
            Ok(serde_json::json!({
                "success": true,
                "has_pending_invitation": true,
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            // 404 通常表示没有待处理邀请
            if status_code == 404 {
                return Ok(serde_json::json!({
                    "success": true,
                    "has_pending_invitation": false,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }));
            }
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取邀请信息失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 接受团队邀请 (AcceptPreapproval API)
    /// 普通用户权限
    pub async fn accept_preapproval(&self, ctx: &AuthContext, approval_id: &str) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/AcceptPreapproval", WINDSURF_BASE_URL);
        
        let mut body = self.encode_string_field(1, token);
        body.extend(self.encode_string_field(2, approval_id));
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[AcceptPreapproval] Status: {}", status_code);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "已接受邀请，成功加入团队",
                "approval_id": approval_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "接受邀请失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 拒绝团队邀请 (RejectPreapproval API)
    /// 普通用户权限
    pub async fn reject_preapproval(&self, ctx: &AuthContext, approval_id: &str) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/RejectPreapproval", WINDSURF_BASE_URL);
        
        let mut body = self.encode_string_field(1, token);
        body.extend(self.encode_string_field(2, approval_id));
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[RejectPreapproval] Status: {}", status_code);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "已拒绝邀请",
                "approval_id": approval_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "拒绝邀请失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 申请加入团队 (RequestTeamAccess API)
    /// 普通用户通过邀请链接申请加入团队
    pub async fn request_team_access(&self, api_key: &str, invite_id: &str) -> AppResult<serde_json::Value> {
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/RequestTeamAccess", WINDSURF_BASE_URL);
        
        let mut body = self.encode_string_field(1, api_key);
        body.extend(self.encode_string_field(2, invite_id));
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let response_body = response.bytes().await
            .map_err(|e| AppError::Api(e.to_string()))?;

        println!("[RequestTeamAccess] Status: {}, Size: {} bytes", status_code, response_body.len());

        if status_code == 200 {
            if response_body.is_empty() {
                return Ok(serde_json::json!({
                    "success": true,
                    "message": "加入申请已提交，等待管理员审批",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }));
            }
            
            let mut parser = super::proto_parser::ProtobufParser::new(response_body.to_vec());
            let parsed = parser.parse_message().unwrap_or_else(|_| serde_json::json!({}));
            
            Ok(serde_json::json!({
                "success": true,
                "message": "加入申请已提交",
                "data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_text = String::from_utf8_lossy(&response_body).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "申请加入团队失败",
                "error_details": error_text,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 更新用户团队状态 (UpdateUserTeamStatus API)
    /// 管理员审批用户的加入申请
    /// status: 2=APPROVED(同意), 3=REJECTED(拒绝)
    pub async fn update_user_team_status(&self, ctx: &AuthContext, user_api_key: &str, status: u8) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/UpdateUserTeamStatus", WINDSURF_BASE_URL);
        
        // 构建嵌套消息: { api_key: string, status: int }
        let mut inner_msg = self.encode_string_field(1, user_api_key);
        // field 2 (status), wire type 0 (varint)
        inner_msg.push(0x10);
        inner_msg.push(status);
        
        // 构建外层消息
        let mut body = self.encode_string_field(1, token);
        // field 2, wire type 2 (嵌套消息)
        body.push(0x12);
        let inner_len = inner_msg.len();
        if inner_len < 128 {
            body.push(inner_len as u8);
        } else {
            body.push((inner_len & 0x7F | 0x80) as u8);
            body.push((inner_len >> 7) as u8);
        }
        body.extend(inner_msg);
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[UpdateUserTeamStatus] Status: {}", status_code);

        let status_text = match status {
            2 => "已批准加入",
            3 => "已拒绝加入",
            _ => "状态已更新",
        };

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": status_text,
                "user_api_key": user_api_key,
                "new_status": status,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "更新用户状态失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    // ==================== 自动充值管理 API ====================

    /// 辅助方法：编码 varint
    fn encode_varint(&self, value: u64) -> Vec<u8> {
        let mut result = Vec::new();
        let mut val = value;
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            result.push(byte);
            if val == 0 {
                break;
            }
        }
        result
    }

    /// 更新自动充值设置 (UpdateCreditTopUpSettings API)
    /// 需要管理员权限
    pub async fn update_credit_top_up_settings(
        &self,
        ctx: &AuthContext,
        enabled: bool,
        monthly_top_up_amount: i32,
        top_up_increment: i32,
    ) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/UpdateCreditTopUpSettings", WINDSURF_BASE_URL);
        
        // 构建 protobuf 消息
        let mut body = self.encode_string_field(1, token);
        
        // field 2: enabled (bool as varint)
        body.push(0x10); // field 2, wire type 0
        body.push(if enabled { 1 } else { 0 });
        
        // field 3: monthly_top_up_amount (int32 as varint)
        body.push(0x18); // field 3, wire type 0
        body.extend(self.encode_varint(monthly_top_up_amount as u64));
        
        // field 4: top_up_increment (int32 as varint)
        body.push(0x20); // field 4, wire type 0
        body.extend(self.encode_varint(top_up_increment as u64));
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[UpdateCreditTopUpSettings] Status: {}", status_code);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": if enabled { "自动充值已启用" } else { "自动充值已禁用" },
                "enabled": enabled,
                "monthly_top_up_amount": monthly_top_up_amount,
                "top_up_increment": top_up_increment,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "更新自动充值设置失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 获取自动充值设置（从 GetPlanStatus 响应的 subMessage_10 中提取）
    /// subMessage_10 包含:
    /// - int_1: 状态
    /// - int_2: enabled (1=启用)
    /// - int_3: monthly_top_up_amount (单位: 分/100)
    /// - int_5: top_up_increment (单位: 分)
    pub async fn get_credit_top_up_settings(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetPlanStatus", WINDSURF_BASE_URL);
        
        // 构建请求体
        let token_bytes = token.as_bytes();
        let token_length = token_bytes.len();
        
        let mut body = vec![0x0a];
        if token_length < 128 {
            body.push(token_length as u8);
        } else {
            body.push(((token_length & 0x7F) | 0x80) as u8);
            body.push((token_length >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("cache-control", "no-cache")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("pragma", "no-cache")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await?;
        
        let status_code = response.status().as_u16();
        let response_body = response.bytes().await?;
        
        println!("[GetCreditTopUpSettings] Status: {}", status_code);
        
        if status_code == 200 {
            // 解析 protobuf 响应
            match super::proto_parser::ProtobufParser::parse_get_plan_status_response(&response_body) {
                Ok(parsed) => {
                    // 从 raw_data.subMesssage_1.subMesssage_10 提取自动充值设置
                    let top_up_status = parsed
                        .get("raw_data")
                        .and_then(|d| d.get("subMesssage_1"))
                        .and_then(|s| s.get("subMesssage_10"));
                    
                    if let Some(top_up) = top_up_status {
                        let enabled = top_up["int_2"].as_i64().unwrap_or(0) == 1;
                        // API 返回的值单位已经是美分，直接使用
                        let monthly_top_up_amount = top_up["int_3"].as_i64().unwrap_or(0) as i32;
                        let top_up_increment = top_up["int_5"].as_i64().unwrap_or(0) as i32;
                        
                        return Ok(serde_json::json!({
                            "success": true,
                            "top_up_enabled": enabled,
                            "monthly_top_up_amount": monthly_top_up_amount,
                            "top_up_increment": top_up_increment as i32,
                            "top_up_spent": 0,
                            "raw_top_up_status": top_up,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }));
                    }
                    
                    // 如果没有 top_up_status，返回完整解析结果便于调试
                    Ok(serde_json::json!({
                        "success": true,
                        "top_up_enabled": false,
                        "monthly_top_up_amount": 0,
                        "top_up_increment": 0,
                        "top_up_spent": 0,
                        "note": "No top_up_status found in response",
                        "raw_parsed": parsed,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                },
                Err(e) => {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": format!("解析响应失败: {}", e),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                }
            }
        } else {
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "获取自动充值设置失败",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 更新成员的 Windsurf 访问权限 (UpdateCodeiumAccess API)
    /// disable_access: true = 禁用访问, false = 启用访问
    pub async fn update_codeium_access(&self, ctx: &AuthContext, api_key: &str, disable_access: bool) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/UpdateCodeiumAccess", WINDSURF_BASE_URL);
        
        // 构建请求体：auth_token(1) + api_key(2) + disable_codeium_access(3)
        let mut body = self.encode_string_field(1, token);
        body.extend(self.encode_string_field(2, api_key));
        // bool 字段编码：field_num << 3 | 0, 然后是值（0或1）
        body.push((3 << 3) | 0); // field 3, wire type 0 (varint)
        body.push(if disable_access { 1 } else { 0 });
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();

        if status_code == 200 {
            println!("[UpdateCodeiumAccess] Status: 200, disable={}", disable_access);
            Ok(serde_json::json!({
                "success": true,
                "message": if disable_access { "已禁用 Windsurf 访问" } else { "已启用 Windsurf 访问" },
                "api_key": api_key,
                "disabled": disable_access,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            // 透明化错误响应：打印状态码、上下文摘要、原始响应体
            // 上下文摘要仅暴露 token / api_key 前 8 位，避免日志泄漏完整凭证
            let raw_body = response.bytes().await
                .map_err(|e| AppError::Api(e.to_string()))?;
            let raw_body_text = String::from_utf8_lossy(&raw_body).to_string();

            let token_kind = if ctx.devin.is_some() { "devin" } else { "firebase" };
            let token_preview: String = token.chars().take(16).collect();
            let api_key_preview: String = api_key.chars().take(8).collect();

            println!(
                "[UpdateCodeiumAccess] 错误响应: status={}, disable={}, token_kind={}, token_prefix={}..., api_key_prefix={}..., body={}",
                status_code,
                disable_access,
                token_kind,
                token_preview,
                api_key_preview,
                raw_body_text
            );

            // Connect Protocol 错误响应通常是 JSON 形如 {"code":"permission_denied","message":"..."}
            // 尝试解析以便前端展示友好错误
            let parsed_error = serde_json::from_slice::<serde_json::Value>(&raw_body).ok();

            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "更新访问权限失败",
                "error_details": raw_body_text,
                "parsed_error": parsed_error,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 添加用户角色 (AddUserRole API)
    /// role: 角色名称，如 "admin", "billing.admin" 等
    pub async fn add_user_role(&self, ctx: &AuthContext, api_key: &str, role: &str, group_id: Option<&str>) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/AddUserRole", WINDSURF_BASE_URL);
        
        // 构建请求体：auth_token(1) + api_key(2) + role(3) + group_id(4, optional)
        let mut body = self.encode_string_field(1, token);
        body.extend(self.encode_string_field(2, api_key));
        body.extend(self.encode_string_field(3, role));
        if let Some(gid) = group_id {
            body.extend(self.encode_string_field(4, gid));
        }
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[AddUserRole] Status: {}, role={}", status_code, role);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": format!("已添加角色: {}", role),
                "api_key": api_key,
                "role": role,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "添加角色失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 移除用户角色 (RemoveUserRole API)
    pub async fn remove_user_role(&self, ctx: &AuthContext, api_key: &str, role: &str, group_id: Option<&str>) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/RemoveUserRole", WINDSURF_BASE_URL);
        
        // 构建请求体：auth_token(1) + api_key(2) + role(3) + group_id(4, optional)
        let mut body = self.encode_string_field(1, token);
        body.extend(self.encode_string_field(2, api_key));
        body.extend(self.encode_string_field(3, role));
        if let Some(gid) = group_id {
            body.extend(self.encode_string_field(4, gid));
        }
        
        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .with_auth(ctx)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        println!("[RemoveUserRole] Status: {}, role={}", status_code, role);

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": format!("已移除角色: {}", role),
                "api_key": api_key,
                "role": role,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "移除角色失败",
                "error_details": error_body,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        }
    }

    /// 检查Pro试用资格 (CheckProTrialEligibility)
    /// 请求: auth_token (field 1, string)
    /// 响应: is_eligible (field 1, bool)
    ///
    /// **安全解析策略**：Windsurf 后端对过期但结构合法的 Firebase ID Token 仍可能返回
    /// HTTP 200 + 业务响应（疑似仅解 JWT payload 取 uid 而未校验签名/exp），导致"失效 token
    /// 被判合格"的误报。本实现通过以下手段 fail-fast：
    /// 1. Content-Type 必须是 `application/proto` / `application/protobuf`，否则判为异常
    ///    （Connect 协议错误响应通常以 `application/json` 返回）
    /// 2. body 必须是合法的 protobuf 消息且 field 1 是 varint（wire_type=0），只有在严格
    ///    读到 `tag=0x08, value=0x01` 时才判 `is_eligible = true`
    /// 3. 其它非预期响应形态一律 `success=false` 并回传 body 前若干字节 hex 便于定位
    pub async fn check_pro_trial_eligibility(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let url = "https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/CheckProTrialEligibility";

        // 构建 protobuf body: field 1 = auth_token (string)
        let auth_token = ctx.token_str();
        let token_bytes = auth_token.as_bytes();
        let token_len = token_bytes.len();
        let mut body: Vec<u8> = Vec::new();
        body.push(0x0a); // field 1, wire type 2 (length-delimited)
        if token_len < 128 {
            body.push(token_len as u8);
        } else {
            body.push(((token_len & 0x7F) | 0x80) as u8);
            body.push((token_len >> 7) as u8);
        }
        body.extend_from_slice(token_bytes);

        // with_auth 设置正确的认证 headers（Firebase/Devin 自动区分）
        let response = self.client
            .post(url)
            .with_auth(ctx)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        // 非 200：直接当失败
        if status_code != 200 {
            let error_text = String::from_utf8_lossy(&body_bytes).to_string();
            return Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "检查试用资格失败",
                "error_details": error_text,
            }));
        }

        // Content-Type 校验：只接受 proto；其它（如 Connect 的 application/json 错误包装）判异常
        let is_proto_ct = content_type.starts_with("application/proto")
            || content_type.starts_with("application/protobuf")
            || content_type.starts_with("application/x-protobuf")
            || content_type.is_empty(); // 某些代理可能不带 header，暂放行由 body 解析兜底
        if !is_proto_ct {
            return Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": "响应 Content-Type 非 protobuf，疑似 token 失效或后端异常",
                "content_type": content_type,
                "error_details": String::from_utf8_lossy(&body_bytes).to_string(),
            }));
        }

        // body 解析：严格按 protobuf wire format 扫描 field 1 的 bool 值
        match parse_is_eligible_strict(&body_bytes) {
            Ok(is_eligible) => Ok(serde_json::json!({
                "success": true,
                "is_eligible": is_eligible,
                "message": if is_eligible { "您有资格免费试用Pro" } else { "您暂无Pro试用资格" },
            })),
            Err(reason) => {
                // 响应形态异常：不报"合格"，fail-fast
                let hex_preview: String = body_bytes
                    .iter()
                    .take(32)
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                Ok(serde_json::json!({
                    "success": false,
                    "status_code": status_code,
                    "error": format!("响应格式异常: {}", reason),
                    "body_len": body_bytes.len(),
                    "body_hex_preview": hex_preview,
                }))
            }
        }
    }

    // ==================== 用户API密钥管理 API ====================

    /// 获取用户API密钥摘要列表 (GetApiKeySummary)
    pub async fn get_api_key_summary(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetApiKeySummary", WINDSURF_BASE_URL);
        let body = self.encode_string_field(1, token);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let parsed = super::proto_parser::ProtobufParser::new(body_bytes.to_vec())
                .parse_message()
                .unwrap_or_else(|_| serde_json::json!({}));

            // 解析 api_keys：可能是数组或单个对象
            let field1 = parsed.get("1").or_else(|| parsed.get("subMesssage_1"));
            let api_keys = if let Some(field) = field1 {
                if let Some(arr) = field.as_array() {
                    arr.iter().map(|item| {
                        let key_id = item.get("1").or_else(|| item.get("string_1")).and_then(|v| v.as_str()).unwrap_or("");
                        let key_for_display = item.get("2").or_else(|| item.get("string_2")).and_then(|v| v.as_str()).unwrap_or("");
                        let created_at = item.get("3").or_else(|| item.get("subMesssage_3")).and_then(|v| {
                            v.get("1").or_else(|| v.get("int_1")).and_then(|s| s.as_i64())
                        }).unwrap_or(0);
                        let last_used_at = item.get("4").or_else(|| item.get("subMesssage_4")).and_then(|v| {
                            v.get("1").or_else(|| v.get("int_1")).and_then(|s| s.as_i64())
                        }).unwrap_or(0);
                        serde_json::json!({
                            "key_id": key_id,
                            "key_for_display": key_for_display,
                            "created_at": created_at,
                            "last_used_at": last_used_at
                        })
                    }).collect::<Vec<_>>()
                } else if field.is_object() {
                    let key_id = field.get("1").or_else(|| field.get("string_1")).and_then(|v| v.as_str()).unwrap_or("");
                    let key_for_display = field.get("2").or_else(|| field.get("string_2")).and_then(|v| v.as_str()).unwrap_or("");
                    let created_at = field.get("3").or_else(|| field.get("subMesssage_3")).and_then(|v| {
                        v.get("1").or_else(|| v.get("int_1")).and_then(|s| s.as_i64())
                    }).unwrap_or(0);
                    let last_used_at = field.get("4").or_else(|| field.get("subMesssage_4")).and_then(|v| {
                        v.get("1").or_else(|| v.get("int_1")).and_then(|s| s.as_i64())
                    }).unwrap_or(0);
                    vec![serde_json::json!({
                        "key_id": key_id,
                        "key_for_display": key_for_display,
                        "created_at": created_at,
                        "last_used_at": last_used_at
                    })]
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            Ok(serde_json::json!({
                "success": true,
                "api_keys": api_keys,
                "raw_data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = String::from_utf8_lossy(&body_bytes).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    /// 注册用户并获取新的API Key (RegisterUser)
    /// 通过 Firebase ID Token 注册并获取 sk-ws-01 格式的 API Key
    pub async fn register_user(&self, firebase_id_token: &str) -> AppResult<serde_json::Value> {
        // RegisterUser 使用 JSON 格式而非 Protobuf
        let url = "https://register.windsurf.com/exa.seat_management_pb.SeatManagementService/RegisterUser";

        let request_body = serde_json::json!({
            "firebase_id_token": firebase_id_token
        });

        let response = self.client
            .post(url)
            .json(&request_body)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_text = response.text().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let json_result: Result<serde_json::Value, _> = serde_json::from_str(&body_text);
            match json_result {
                Ok(data) => {
                    let api_key = data.get("api_key").or_else(|| data.get("apiKey"))
                        .and_then(|v| v.as_str()).unwrap_or("");
                    let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let api_server_url = data.get("api_server_url").or_else(|| data.get("apiServerUrl"))
                        .and_then(|v| v.as_str()).unwrap_or("");
                    Ok(serde_json::json!({
                        "success": true,
                        "api_key": api_key,
                        "name": name,
                        "api_server_url": api_server_url,
                        "raw_data": data,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                }
                Err(_) => Ok(serde_json::json!({
                    "success": false,
                    "error": "无法解析响应",
                    "raw_response": body_text,
                })),
            }
        } else {
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": body_text,
            }))
        }
    }

    /// 删除用户API密钥 (DeleteApiKey)
    ///
    /// target 可以是：key_id (field 2)、api_key (field 4) 或 group_id (field 5)
    pub async fn delete_api_key(&self, ctx: &AuthContext, key_id: Option<&str>, api_key: Option<&str>) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/DeleteApiKey", WINDSURF_BASE_URL);

        let mut body = self.encode_string_field(1, token);
        if let Some(kid) = key_id {
            body.extend(self.encode_string_field(2, kid));
        } else if let Some(ak) = api_key {
            body.extend(self.encode_string_field(4, ak));
        }

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let parsed = super::proto_parser::ProtobufParser::new(body_bytes.to_vec())
                .parse_message()
                .unwrap_or_else(|_| serde_json::json!({}));
            let num_deleted = parsed.get("1").or_else(|| parsed.get("int_1"))
                .and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(serde_json::json!({
                "success": true,
                "num_deleted": num_deleted,
                "message": format!("已删除 {} 个密钥", num_deleted),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = String::from_utf8_lossy(&body_bytes).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    // ==================== 第三方 API Provider Key 管理 ====================

    /// 获取已设置的第三方API Provider列表 (GetSetUserApiProviderKeys)
    pub async fn get_set_user_api_provider_keys(&self, ctx: &AuthContext) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetSetUserApiProviderKeys", WINDSURF_BASE_URL);
        let body = self.encode_string_field(1, token);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let parsed = super::proto_parser::ProtobufParser::new(body_bytes.to_vec())
                .parse_message()
                .unwrap_or_else(|_| serde_json::json!({}));

            // 解析 providers 数组（field 1，repeated enum）
            let providers = if let Some(arr) = parsed.get("1").and_then(|v| v.as_array()) {
                arr.iter().filter_map(|v| v.as_i64()).map(|v| self.provider_id_to_name(v as i32)).collect::<Vec<_>>()
            } else if let Some(v) = parsed.get("1").and_then(|v| v.as_i64()) {
                vec![self.provider_id_to_name(v as i32)]
            } else {
                vec![]
            };

            Ok(serde_json::json!({
                "success": true,
                "providers": providers,
                "raw_data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = String::from_utf8_lossy(&body_bytes).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    /// 设置第三方API Provider Key (SetUserApiProviderKey)
    pub async fn set_user_api_provider_key(&self, ctx: &AuthContext, provider: i32, provider_api_key: &str) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/SetUserApiProviderKey", WINDSURF_BASE_URL);

        let mut body = self.encode_string_field(1, token);
        // field 2: provider (enum as varint)
        body.push(0x10);
        body.extend(self.encode_varint(provider as u64));
        body.extend(self.encode_string_field(3, provider_api_key));

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "API Key已设置",
                "provider": self.provider_id_to_name(provider),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    /// 删除第三方API Provider Key (DeleteUserApiProviderKey)
    pub async fn delete_user_api_provider_key(&self, ctx: &AuthContext, provider: i32) -> AppResult<serde_json::Value> {
        let token = ctx.token_str();
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/DeleteUserApiProviderKey", WINDSURF_BASE_URL);

        let mut body = self.encode_string_field(1, token);
        body.push(0x10);
        body.extend(self.encode_varint(provider as u64));

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();

        if status_code == 200 {
            Ok(serde_json::json!({
                "success": true,
                "message": "API Key已删除",
                "provider": self.provider_id_to_name(provider),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = response.bytes().await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    /// Provider ID → 名称
    fn provider_id_to_name(&self, id: i32) -> String {
        match id {
            0 => "UNSPECIFIED".to_string(),
            1 => "INTERNAL".to_string(),
            2 => "OPENAI".to_string(),
            3 => "GOOGLE_VERTEX".to_string(),
            4 => "ANTHROPIC".to_string(),
            5 => "VLLM".to_string(),
            6 => "TOGETHER_AI".to_string(),
            7 => "HUGGING_FACE".to_string(),
            8 => "NOMIC".to_string(),
            9 => "TEI".to_string(),
            10 => "OPENAI_COMPATIBLE_EXTERNAL".to_string(),
            11 => "ANTHROPIC_COMPATIBLE_EXTERNAL".to_string(),
            12 => "VERTEX_COMPATIBLE_EXTERNAL".to_string(),
            13 => "BEDROCK_COMPATIBLE_EXTERNAL".to_string(),
            14 => "AZURE_COMPATIBLE_EXTERNAL".to_string(),
            15 => "ANTHROPIC_BEDROCK".to_string(),
            16 => "FIREWORKS".to_string(),
            17 => "OPEN_ROUTER".to_string(),
            18 => "XAI".to_string(),
            20 => "ANTHROPIC_BYOK".to_string(),
            21 => "CEREBRAS".to_string(),
            22 => "XAI_BYOK".to_string(),
            23 => "GEMINI_OPENAI".to_string(),
            24 => "GOOGLE_GEMINI".to_string(),
            25 => "GOOGLE_GENAI_VERTEX".to_string(),
            26 => "ANTHROPIC_VERTEX".to_string(),
            27 => "DATABRICKS".to_string(),
            28 => "OPEN_ROUTER_BYOK".to_string(),
            29 => "ANTHROPIC_DEVIN".to_string(),
            30 => "FIREWORKS_DEVIN".to_string(),
            31 => "GROQ".to_string(),
            32 => "OPENAI_DEVIN".to_string(),
            33 => "LLAMA_FT_DEEPWIKI".to_string(),
            _ => format!("UNKNOWN_{}", id),
        }
    }

    /// Provider 名称 → ID
    pub fn provider_name_to_id(&self, name: &str) -> i32 {
        match name.to_uppercase().as_str() {
            "OPENAI" => 2,
            "ANTHROPIC" => 4,
            "ANTHROPIC_BYOK" => 20,
            "GOOGLE_GEMINI" => 24,
            "XAI" | "GROK" => 18,
            "XAI_BYOK" => 22,
            "OPEN_ROUTER" => 17,
            "OPEN_ROUTER_BYOK" => 28,
            "FIREWORKS" => 16,
            "GROQ" => 31,
            "CEREBRAS" => 21,
            "TOGETHER_AI" => 6,
            "AZURE_COMPATIBLE_EXTERNAL" | "AZURE" => 14,
            "OPENAI_COMPATIBLE_EXTERNAL" => 10,
            "ANTHROPIC_COMPATIBLE_EXTERNAL" => 11,
            _ => 0,
        }
    }

    // ==================== 迁移 / 排行榜 / Devs API Key ====================

    /// 获取开发者主API Key (GetPrimaryApiKeyForDevsOnly)
    ///
    /// 通过 session_token 获取主 API Key
    pub async fn get_primary_api_key_for_devs(&self, session_token: &str) -> AppResult<serde_json::Value> {
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/GetPrimaryApiKeyForDevsOnly", WINDSURF_BASE_URL);
        let body = self.encode_string_field(1, session_token);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let parsed = super::proto_parser::ProtobufParser::new(body_bytes.to_vec())
                .parse_message()
                .unwrap_or_else(|_| serde_json::json!({}));
            let api_key = parsed.get("1")
                .or_else(|| parsed.get("string_1"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(serde_json::json!({
                "success": true,
                "api_key": api_key,
                "raw_data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = String::from_utf8_lossy(&body_bytes).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    /// 获取排行榜数据 (GetBigQueryAnalytics with leaderboard_request)
    pub async fn get_leaderboard(&self, api_key: &str) -> AppResult<serde_json::Value> {
        let url = "https://web-backend.windsurf.com/exa.user_analytics_pb.UserAnalyticsService/GetBigQueryAnalytics";

        // 构建 BigQueryRequest with leaderboard_request（field 1, empty message）
        let body: Vec<u8> = vec![0x0a, 0x00];

        let response = self.client
            .post(url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("X-Api-Key", api_key)
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let parsed = super::proto_parser::ProtobufParser::new(body_bytes.to_vec())
                .parse_message()
                .unwrap_or_else(|_| serde_json::json!({}));
            let model_stats = self.parse_leaderboard_result(&parsed);
            Ok(serde_json::json!({
                "success": true,
                "model_stats": model_stats,
                "raw_data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = String::from_utf8_lossy(&body_bytes).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    /// 解析排行榜结果
    ///
    /// BigQueryResult field 2 = leaderboard_result；leaderboard_result field 1 = model_stats (repeated)
    fn parse_leaderboard_result(&self, data: &serde_json::Value) -> Vec<serde_json::Value> {
        let mut results = Vec::new();

        if let Some(leaderboard) = data.get("2").or_else(|| data.get("message_2")) {
            if let Some(stats_array) = leaderboard.get("1").or_else(|| leaderboard.get("repeated_1")) {
                if let Some(arr) = stats_array.as_array() {
                    for stat in arr {
                        let model = stat.get("1").or_else(|| stat.get("string_1"))
                            .and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let elo_rating = stat.get("2").or_else(|| stat.get("varint_2"))
                            .and_then(|v| v.as_i64()).unwrap_or(0);
                        let votes = stat.get("3").or_else(|| stat.get("varint_3"))
                            .and_then(|v| v.as_i64()).unwrap_or(0);
                        let win_rate = stat.get("4").or_else(|| stat.get("double_4"))
                            .and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let confidence_lower = stat.get("6").or_else(|| stat.get("varint_6"))
                            .and_then(|v| v.as_i64()).unwrap_or(0);
                        let confidence_upper = stat.get("7").or_else(|| stat.get("varint_7"))
                            .and_then(|v| v.as_i64()).unwrap_or(0);
                        let model_speed = stat.get("8").or_else(|| stat.get("double_8"))
                            .and_then(|v| v.as_f64()).unwrap_or(0.0);
                        results.push(serde_json::json!({
                            "model": model,
                            "elo_rating": elo_rating,
                            "votes": votes,
                            "win_rate": win_rate,
                            "confidence_lower": confidence_lower,
                            "confidence_upper": confidence_upper,
                            "model_speed": model_speed,
                        }));
                    }
                }
            }
        }

        results
    }

    /// 获取全球排行榜API Key (GetGlobalLeaderboardApiKey)
    pub async fn get_global_leaderboard_api_key(&self, auth_token: Option<&str>) -> AppResult<serde_json::Value> {
        let url = "https://web-backend.windsurf.com/exa.user_analytics_pb.UserAnalyticsService/GetGlobalLeaderboardApiKey";
        let body: Vec<u8> = vec![];

        let mut request = self.client
            .post(url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("content-length", "0")
            .header("Referer", "https://windsurf.com/");

        if let Some(token) = auth_token {
            request = request.header("X-Auth-Token", token);
        }

        let response = request.send().await.map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let parsed = super::proto_parser::ProtobufParser::new(body_bytes.to_vec())
                .parse_message()
                .unwrap_or_else(|_| serde_json::json!({}));
            let api_key = parsed.get("1")
                .or_else(|| parsed.get("string_1"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(serde_json::json!({
                "success": true,
                "api_key": api_key,
                "raw_data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = String::from_utf8_lossy(&body_bytes).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }

    /// 迁移API Key (MigrateApiKey)
    ///
    /// 将旧的 sk-ws-01 API Key 迁移到新的会话 Token
    pub async fn migrate_api_key(&self, api_key: &str) -> AppResult<serde_json::Value> {
        let url = format!("{}/exa.seat_management_pb.SeatManagementService/MigrateApiKey", WINDSURF_BASE_URL);
        let body = self.encode_string_field(1, api_key);

        let response = self.client
            .post(&url)
            .body(body)
            .header("accept", "*/*")
            .header("connect-protocol-version", "1")
            .header("content-type", "application/proto")
            .header("Referer", "https://windsurf.com/")
            .send()
            .await
            .map_err(|e| AppError::Api(e.to_string()))?;

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await.map_err(|e| AppError::Api(e.to_string()))?;

        if status_code == 200 {
            let parsed = super::proto_parser::ProtobufParser::new(body_bytes.to_vec())
                .parse_message()
                .unwrap_or_else(|_| serde_json::json!({}));
            let session_token = parsed.get("1")
                .or_else(|| parsed.get("string_1"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(serde_json::json!({
                "success": true,
                "session_token": session_token,
                "raw_data": parsed,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            let error_body = String::from_utf8_lossy(&body_bytes).to_string();
            Ok(serde_json::json!({
                "success": false,
                "status_code": status_code,
                "error": error_body,
            }))
        }
    }
}

// ============================================================================
// CheckProTrialEligibility 响应严格解析
// ============================================================================

/// 严格按 protobuf wire format 解析 `CheckProTrialEligibilityResponse`
///
/// 只有在 body 是合法的 protobuf 消息、且其中包含 `field 1 (varint) = 0 或 1` 时才返回
/// 对应的 `bool`。空 body 返回 `Ok(false)`（protobuf 省略默认值）。任何不合法字节都会
/// 返回 `Err(reason)`，由调用方 fail-fast 处理，避免"随意凑巧以 0x08 0x01 开头"的
/// 响应（如 Connect 协议错误包装、HTML 错误页等）被误判为"有试用资格"。
fn parse_is_eligible_strict(body: &[u8]) -> Result<bool, String> {
    if body.is_empty() {
        return Ok(false);
    }

    let mut pos = 0usize;
    let mut is_eligible: Option<bool> = None;

    while pos < body.len() {
        let (tag, consumed) = read_varint(&body[pos..])
            .ok_or_else(|| format!("tag varint 解码失败 @offset={}", pos))?;
        pos += consumed;

        let field_num = tag >> 3;
        let wire_type = (tag & 0x07) as u8;

        match wire_type {
            0 => {
                // VARINT
                let (val, c) = read_varint(&body[pos..])
                    .ok_or_else(|| format!("varint 解码失败 @field={}", field_num))?;
                pos += c;
                if field_num == 1 {
                    // is_eligible 必须是合法 bool
                    if val > 1 {
                        return Err(format!("field 1 bool 值非法: {}", val));
                    }
                    is_eligible = Some(val != 0);
                }
            }
            1 => {
                // I64 fixed
                if pos + 8 > body.len() {
                    return Err(format!("i64 越界 @field={}", field_num));
                }
                pos += 8;
            }
            2 => {
                // LEN-delimited
                let (len, c) = read_varint(&body[pos..])
                    .ok_or_else(|| format!("len varint 解码失败 @field={}", field_num))?;
                pos += c;
                let len = len as usize;
                if pos + len > body.len() {
                    return Err(format!("len-delimited 越界 @field={}", field_num));
                }
                pos += len;
            }
            5 => {
                // I32 fixed
                if pos + 4 > body.len() {
                    return Err(format!("i32 越界 @field={}", field_num));
                }
                pos += 4;
            }
            other => {
                return Err(format!("未知 wire_type={} @field={}", other, field_num));
            }
        }
    }

    if pos != body.len() {
        return Err(format!("存在未消费字节: pos={}, len={}", pos, body.len()));
    }

    Ok(is_eligible.unwrap_or(false))
}

/// protobuf varint 解码，返回 `(value, consumed_bytes)`
///
/// 最多 10 字节（u64 最大长度）；遇到超限或截断返回 `None`，由调用方处理错误。
fn read_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &byte) in data.iter().enumerate() {
        if i >= 10 {
            return None;
        }
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}
