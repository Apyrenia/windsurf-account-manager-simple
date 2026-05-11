import { invoke } from '@tauri-apps/api/core';
import type { Account, Settings, OperationLog, UpdateSeatsResult, BillingInfo, BatchResult, GlobalTag, SortField, SortDirection, SortConfig, DevinLoginResult, WindsurfOrg, CheckUserLoginMethodResult, LoginMethodSniffResult, EmailStartResponse, ConnectionsResponse, DevinPasswordLoginResponse } from '@/types';
import type { AnalyticsData } from '@/types/analytics';

// 账号管理API
export const accountApi = {
  async addAccount(data: {
    email: string;
    password: string;
    nickname: string;
    tags: string[];
    group?: string;
  }): Promise<Account> {
    return await invoke('add_account', data);
  },

  async getAllAccounts(): Promise<Account[]> {
    return await invoke('get_all_accounts');
  },

  async getAccount(id: string): Promise<Account> {
    return await invoke('get_account', { id });
  },

  async updateAccount(account: Account): Promise<void> {
    return await invoke('update_account', { account });
  },

  async deleteAccount(id: string): Promise<void> {
    return await invoke('delete_account', { id });
  },

  async deleteAccountsBatch(ids: string[]): Promise<{ success_count: number; failed_ids: string[] }> {
    return await invoke('delete_accounts_batch', { ids });
  },

  async searchAccounts(query: string): Promise<Account[]> {
    return await invoke('search_accounts', { query });
  },

  async filterAccountsByGroup(group: string): Promise<Account[]> {
    return await invoke('filter_accounts_by_group', { group });
  },

  async filterAccountsByTags(tags: string[]): Promise<Account[]> {
    return await invoke('filter_accounts_by_tags', { tags });
  },

  // v1.7.8 方案 B：分页查询 + 聚合统计

  async getAccountsPage(request: AccountPageRequest): Promise<AccountPageResponse> {
    return await invoke('get_accounts_page', { request });
  },

  async getAccountAggregates(): Promise<AccountAggregates> {
    return await invoke('get_account_aggregates');
  },

  async getAllAccountIds(group?: string): Promise<string[]> {
    return await invoke('get_all_account_ids', { group: group || null });
  },

  /** 按 ID 列表精准查询账号（跨页选中操作需完整数据时使用） */
  async getAccountsByIds(ids: string[]): Promise<Account[]> {
    return await invoke('get_accounts_by_ids', { ids });
  },

  /** 按 ID 列表批量更改分组（跨页选中批量操作） */
  async batchUpdateGroupByIds(ids: string[], group: string): Promise<number> {
    return await invoke('batch_update_group_by_ids', { ids, group });
  },
};

export interface AccountPageRequest {
  page?: number;
  page_size?: number;
  search?: string;
  group?: string;
  tags?: string[];
  plan_names?: string[];
  domains?: string[];
  statuses?: string[];
  remaining_quota_min?: number;
  remaining_quota_max?: number;
  total_quota_min?: number;
  total_quota_max?: number;
  expiry_days_min?: number;
  expiry_days_max?: number;
  daily_quota_percent_min?: number;
  daily_quota_percent_max?: number;
  weekly_quota_percent_min?: number;
  weekly_quota_percent_max?: number;
  sort_field?: string;
  sort_direction?: string;
}

export interface AccountPageResponse {
  accounts: Account[];
  total: number;
  page: number;
  page_size: number;
}

export interface AccountAggregates {
  total_count: number;
  groups: string[];
  plan_names: string[];
  domains: string[];
  tags: string[];
  active_count: number;
  group_counts: Record<string, number>;
  tag_counts: Record<string, number>;
}

// API操作
export const apiService = {
  async loginAccount(id: string): Promise<{ 
    success: boolean; 
    expires_at: string;
    plan_name?: string;
    used_quota?: number;
    total_quota?: number;
    subscription_expires_at?: string;
    is_disabled?: boolean;
    billing_strategy?: number;
    daily_quota_remaining_percent?: number;
    weekly_quota_remaining_percent?: number;
    daily_quota_reset_at_unix?: number;
    weekly_quota_reset_at_unix?: number;
    overage_balance_micros?: number;
  }> {
    return await invoke('login_account', { id });
  },

  async refreshToken(id: string): Promise<{ 
    success: boolean; 
    token?: string;
    expires_at?: string; 
    old_expires_at?: string; 
    message?: string;
    plan_name?: string;
    used_quota?: number;
    total_quota?: number;
    subscription_expires_at?: string;
    is_disabled?: boolean;
    is_team_owner?: boolean;
    billing_strategy?: number;
    daily_quota_remaining_percent?: number;
    weekly_quota_remaining_percent?: number;
    daily_quota_reset_at_unix?: number;
    weekly_quota_reset_at_unix?: number;
    overage_balance_micros?: number;
  }> {
    return await invoke('refresh_token', { id });
  },

  async resetCredits(id: string, seatCount?: number): Promise<any> {
    return await invoke('reset_credits', { id, seatCount });
  },

  async updateSeats(id: string, seatCount: number, retryTimes: number): Promise<UpdateSeatsResult> {
    return await invoke('update_seats', { id, seatCount, retryTimes });
  },

  async getBilling(id: string): Promise<BillingInfo> {
    return await invoke('get_billing', { id });
  },

  async batchResetCredits(ids: string[], seatCount?: number): Promise<BatchResult> {
    return await invoke('batch_reset_credits', { ids, seatCount });
  },

  async batchRefreshTokens(ids: string[]): Promise<BatchResult> {
    return await invoke('batch_refresh_tokens', { ids });
  },

  /**
   * 更新订阅计划
   * @param id 账号ID
   * @param planType 计划类型
   * @param paymentPeriod 付款周期（1=月付, 2=年付，默认1）
   * @param preview 预览模式（true=仅预览不实际执行，默认false）
   */
  async updatePlan(id: string, planType: string, paymentPeriod: number = 1, preview: boolean = false): Promise<{
    success: boolean;
    preview?: boolean;
    plan_type?: string;
    payment_period?: number;
    payment_period_name?: string;
    status_code?: number;
    applied_changes?: boolean;
    payment_failure_reason?: string | null;
    billing_update?: {
      amount_due_immediately?: number;
      price_per_seat?: number;
      num_seats?: number;
      sub_interval?: number;
      sub_interval_name?: string;
      amount_per_interval?: number;
      billing_start?: string;
      billing_end?: string;
      unused_plan_refunded?: boolean;
      has_sso_add_on?: boolean;
    };
    requires_password_reset?: boolean;
    raw_response?: string;
    timestamp: string;
  }> {
    return await invoke('update_plan', { id, planType, paymentPeriod, preview });
  },

  /**
   * 取消订阅
   * @param id 账号ID
   * @param reason 取消原因
   * @returns 包含操作结果的响应
   */
  async cancelSubscription(id: string, reason: string): Promise<{
    success: boolean;
    reason?: string;
    status_code?: number;
    raw_response?: string;
    timestamp: string;
  }> {
    return await invoke('cancel_subscription', { id, reason });
  },

  /**
   * 恢复订阅
   * @param id 账号ID
   * @returns 包含操作结果的响应
   */
  async resumeSubscription(id: string): Promise<{
    success: boolean;
    status_code?: number;
    raw_response?: string;
    timestamp: string;
  }> {
    return await invoke('resume_subscription', { id });
  },

  async getAccountInfo(id: string): Promise<any> {
    return await invoke('get_account_info', { id });
  },

  async getCurrentUser(id: string): Promise<any> {
    return await invoke('get_current_user', { id });
  },

  /**
   * 获取套餐状态（积分/配额信息）
   * 比 getCurrentUser 更轻量，专用于刷新积分状态
   * @param id 账号ID
   * @returns 包含套餐状态和积分信息的响应
   */
  async getPlanStatus(id: string): Promise<{
    success: boolean;
    plan_status?: {
      plan_name?: string;
      teams_tier?: number;
      teams_tier_name?: string;
      used_prompt_credits?: number;
      used_flow_credits?: number;
      used_flex_credits?: number;
      available_prompt_credits?: number;
      available_flow_credits?: number;
      available_flex_credits?: number;
      monthly_prompt_credits?: number;
      monthly_flow_credits?: number;
      plan_start?: number;
      plan_end?: number;
    };
    status_code?: number;
    error?: string;
    timestamp: string;
  }> {
    return await invoke('get_plan_status', { id });
  },

  /**
   * 获取试用绑卡链接
   * @param id 账号ID
   * @param teamsTier 团队等级: 1=Teams, 2=Pro, 3=Enterprise
   * @param paymentPeriod 支付周期: 1=月付, 2=年付
   * @param teamName 团队名称 (仅 Teams/Enterprise 需要)
   * @param seatCount 席位数量 (仅 Teams/Enterprise 需要)
   * @param turnstileToken Turnstile 验证令牌 (startTrial=true 时所有计划均必需)
   * @returns 包含Stripe Checkout链接的响应
   */
  async getTrialPaymentLink(
    id: string, 
    teamsTier?: number,
    paymentPeriod?: number,
    startTrial?: boolean,
    teamName?: string,
    seatCount?: number,
    turnstileToken?: string
  ): Promise<{
    success: boolean;
    stripe_url?: string;
    teams_tier?: number;
    payment_period?: number;
    status_code?: number;
    error?: string;
    timestamp: string;
  }> {
    return await invoke('get_trial_payment_link', { id, teamsTier, paymentPeriod, startTrial, teamName, seatCount, turnstileToken });
  },

  // === Protobuf解析API（返回解析后的结构化数据） ===
  
  /**
   * 获取当前用户信息（自动解析Protobuf）
   * 返回解析后的用户数据结构
   */
  async getCurrentUserParsed(id: string): Promise<{
    success: boolean;
    data?: {
      user: {
        id: string;
        name: string;
        email: string;
        firebase_uid: string;
        subscription_id: string;
        team: string;
        timezone: string;
      };
      subscription?: {
        id: string;
        email: string;
        stripe_subscription_id: string;
        stripe_customer_id: string;
        seats: number;
        usage: number;
        quota: number;
        used_quota: number;
      };
      plan?: {
        name: string;
        level: number;
        bandwidth_limit: number;
        cache_limit: number;
      };
      admin?: {
        id: string;
        username: string;
        role: string;
      };
      is_root_admin: boolean;
    };
    parsed_data?: any;
    timestamp: string;
    error?: string;
  }> {
    return await invoke('get_current_user_parsed', { id });
  },

  /**
   * 获取账单信息（自动解析Protobuf）
   * TODO: 实现Protobuf解析
   */
  async getBillingParsed(id: string): Promise<any> {
    return await invoke('get_billing_parsed', { id });
  },

  /**
   * 批量获取用户信息（自动解析Protobuf）
   */
  async batchGetUsersParsed(ids: string[]): Promise<{
    success: boolean;
    results: Array<{
      id: string;
      success: boolean;
      data?: any;
      error?: string;
    }>;
    total: number;
    timestamp: string;
  }> {
    return await invoke('batch_get_users_parsed', { ids });
  },

  /**
   * 一键切换账号到Windsurf
   * @param id 账号ID
   * @returns 切换结果
   */
  async switchAccount(id: string): Promise<{ 
    success: boolean; 
    message?: string;
    auth_token?: string;
    machine_id_reset?: boolean;
    seamless_patch_active?: boolean;
    auto_enabled_seamless?: boolean;
    error?: string;
  }> {
    return await invoke('switch_account', { id });
  },
  
  /**
   * 重置机器ID
   * @returns 重置结果
   */
  async resetMachineId(): Promise<{
    success: boolean;
    message?: string;
    error?: string;
    requires_admin?: boolean;
  }> {
    return await invoke('reset_machine_id');
  },

  /**
   * 检查并执行额度耗尽自动切换（Fork 新增）
   *
   * 后端会按 Settings 中的 `autoSwitchEnabled / autoSwitchThresholdPercent /
   * autoSwitchRemainingThreshold` 三个开关决定是否触发切换。前端定时器只需周期性调用，
   * 不需要在前端做任何阈值判断。
   *
   * 后端 emit 事件 `auto-switch-result`，前端可通过 `listen` 订阅以显示 toast。
   */
  async checkAndAutoSwitch(): Promise<{
    triggered: boolean;
    current_email?: string;
    current_usage_percent?: number;
    current_remaining?: number;
    switched_to_email?: string;
    switched_to_remaining?: number;
    reason?: string;
  }> {
    return await invoke('check_and_auto_switch');
  },
};

// 设置管理API
export const settingsApi = {
  async getSettings(): Promise<Settings> {
    return await invoke('get_settings');
  },

  async updateSettings(settings: Settings): Promise<void> {
    return await invoke('update_settings', { settings });
  },

  async getGroups(): Promise<string[]> {
    return await invoke('get_groups');
  },

  async addGroup(name: string): Promise<void> {
    return await invoke('add_group', { name });
  },

  async deleteGroup(name: string): Promise<void> {
    return await invoke('delete_group', { name });
  },

  async renameGroup(oldName: string, newName: string): Promise<void> {
    return await invoke('rename_group', { oldName, newName });
  },

  async getLogs(limit?: number): Promise<OperationLog[]> {
    return await invoke('get_logs', { limit });
  },

  async clearLogs(): Promise<void> {
    return await invoke('clear_logs');
  },

  async exportData(): Promise<any> {
    return await invoke('export_data');
  },

  async getStats(): Promise<any> {
    return await invoke('get_stats');
  },

  async getCurrentWindsurfInfo(): Promise<{
    email?: string;
    name?: string;
    api_key?: string;
    plan_name?: string;
    team_id?: string;
    version?: string;
    is_active: boolean;
    /** 活跃客户端类型："windsurf" | "windsurf-next" */
    client_type: string;
    /** 客户端展示名："Windsurf" | "Windsurf - Next" */
    client_display_name: string;
    /** 活跃客户端进程是否正在运行 */
    is_running: boolean;
  }> {
    return await invoke('get_current_windsurf_info');
  },

  // 标签管理
  async getTags(): Promise<GlobalTag[]> {
    return await invoke('get_tags');
  },

  async addTag(tag: GlobalTag): Promise<void> {
    return await invoke('add_tag', { tag });
  },

  async updateTag(oldName: string, tag: GlobalTag): Promise<void> {
    return await invoke('update_tag', { oldName, tag });
  },

  async deleteTag(name: string): Promise<void> {
    return await invoke('delete_tag', { name });
  },

  async batchUpdateAccountTags(accountIds: string[], addTags: string[], removeTags: string[]): Promise<{
    success_count: number;
    failed_count: number;
  }> {
    return await invoke('batch_update_account_tags', { accountIds, addTags, removeTags });
  },

  // Team Settings API
  async getTeamConfig(id: string): Promise<{
    success: boolean;
    data?: {
      team_id?: string;
      allow_auto_run_commands?: boolean;
      allow_mcp_servers?: boolean;
      allow_app_deployments?: boolean;
      allow_sandbox_app_deployments?: boolean;
      allow_teams_app_deployments?: boolean;
      allow_github_reviews?: boolean;
      allow_github_description_edits?: boolean;
      allow_conversation_sharing?: boolean;
      allow_individual_level_analytics?: boolean;
      allow_attribution?: boolean;
      allow_vibe_and_replace?: boolean;
      allow_browser_experimental_features?: boolean;
      disable_deepwiki?: boolean;
      allowed_mcp_servers?: string[];
    };
    error?: string;
  }> {
    return await invoke('get_team_config', { id });
  },

  async updateTeamConfig(id: string, config: {
    allow_auto_run_commands?: boolean;
    allow_mcp_servers?: boolean;
    allow_app_deployments?: boolean;
    allow_sandbox_app_deployments?: boolean;
    allow_teams_app_deployments?: boolean;
    allow_github_reviews?: boolean;
    allow_github_description_edits?: boolean;
    allow_conversation_sharing?: boolean;
    allow_individual_level_analytics?: boolean;
    allow_attribution?: boolean;
    allow_vibe_and_replace?: boolean;
    allow_browser_experimental_features?: boolean;
    disable_deepwiki?: boolean;
    allowed_mcp_servers?: string;
  }): Promise<{ success: boolean; error?: string }> {
    return await invoke('update_team_config', { id, config });
  },

  // 排序管理
  async getSortedAccounts(sortField: SortField, sortDirection: SortDirection): Promise<Account[]> {
    return await invoke('get_sorted_accounts', { sortField, sortDirection });
  },

  async updateAccountsOrder(accountIds: string[]): Promise<void> {
    return await invoke('update_accounts_order', { accountIds });
  },

  async updateSortConfig(sortConfig: SortConfig): Promise<void> {
    return await invoke('update_sort_config', { sortConfig });
  },

  async getSortConfig(): Promise<SortConfig> {
    return await invoke('get_sort_config');
  }
};

// Analytics API
export const analyticsApi = {
  /**
   * 获取账户的使用分析数据（最近30天）
   * @param id 账户ID
   * @returns 分析数据
   */
  async getAccountAnalytics(id: string): Promise<AnalyticsData> {
    return await invoke('get_account_analytics', { id });
  }
};

// Devin Session 认证 API
export const devinApi = {
  /**
   * 登录流派智能嗅探（方案 B 统一入口）
   *
   * 后端并发调：
   * - Firebase 侧 `CheckUserLoginMethod`　（user_exists / is_migrated / has_password 等）
   * - Devin 侧 `/_devin-auth/connections`　（method / sso_connections 等）
   *
   * 返回 `recommended` 字段标注建议的登录流派，前端据此自动分派。
   */
  async sniffLoginMethod(email: string): Promise<LoginMethodSniffResult> {
    return await invoke('sniff_login_method', { email });
  },

  /**
   * 单独调用 Firebase 侧 `CheckUserLoginMethod`（调试/明细展示）
   *
   * 日常智能登录请直接使用 `sniffLoginMethod`。
   */
  async checkUserLoginMethod(email: string): Promise<CheckUserLoginMethodResult> {
    return await invoke('devin_check_user_login_method', { email });
  },

  /**
   * 查询指定邮箱可用的登录方式（可选，用于 UI 预判断）
   */
  async checkConnections(email: string): Promise<any> {
    return await invoke('devin_check_connections', { email });
  },

  /**
   * 仅账密登录（底层接口），返回 auth1_token
   */
  async passwordLogin(email: string, password: string): Promise<{
    auth1_token: string;
    account_id?: string;
    [key: string]: any;
  }> {
    return await invoke('devin_password_login', { email, password });
  },

  /**
   * 使用 auth1_token 换取 session_token（底层接口）
   */
  async windsurfPostAuth(auth1Token: string, orgId?: string): Promise<{
    session_token: string;
    auth1_token?: string;
    account_id?: string;
    primary_org_id?: string;
    orgs: WindsurfOrg[];
  }> {
    return await invoke('devin_windsurf_post_auth', { auth1Token, orgId });
  },

  /**
   * 完整流程：账密登录 + 建账号（主流程）
   *
   * 当账号属于多个组织时，返回 `requires_org_selection=true` + orgs 列表，
   * UI 需要让用户选择 org 后调用 `addAccountWithOrg`
   */
  async addAccountByLogin(params: {
    email: string;
    password: string;
    nickname?: string;
    tags: string[];
    group?: string;
    orgId?: string;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_login', {
      email: params.email,
      password: params.password,
      nickname: params.nickname,
      tags: params.tags,
      group: params.group,
      orgId: params.orgId,
    });
  },

  /**
   * **Devin 原生账密登录 + 建账号**（v1.7.6 新增）
   *
   * 与 `addAccountByLogin` 的关键区别：
   * - 端口走 `app.devin.ai/api/auth1/password/login`（**Devin 原生侧**），而非 Windsurf 桥接侧
   * - 适用场景：`sniffLoginMethod` 返回 `recommended="devin_native"` 的纯 Devin 原生账号
   *   （在 Firebase + Windsurf 桥接两侧都查不到，仅在 app.devin.ai 侧有注册记录）
   * - 内部仍会调 `WindsurfPostAuth` 桥接产出 session_token，落库字段与 `addAccountByLogin` 完全对齐
   *
   * 多组织场景下返回 `requires_org_selection=true` + orgs，UI 调 `addAccountWithOrg` 二次完成。
   */
  async addAccountByDevinNativeLogin(params: {
    email: string;
    password: string;
    nickname?: string;
    tags: string[];
    group?: string;
    orgId?: string;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_native_login', {
      email: params.email,
      password: params.password,
      nickname: params.nickname,
      tags: params.tags,
      group: params.group,
      orgId: params.orgId,
    });
  },

  /**
   * 多组织场景下的二次选择：在已有 auth1_token 的基础上，指定 org_id 完成账号创建
   *
   * `password` 可选：账密流首次选 org 失败后二次调用时传入用户原始密码，让账号卡能回显；
   * 纯凭证迁入 / 邮箱无密登录场景可省略。
   */
  async addAccountWithOrg(params: {
    email: string;
    auth1Token: string;
    orgId: string;
    nickname?: string;
    tags: string[];
    group?: string;
    password?: string;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_with_org', {
      email: params.email,
      auth1Token: params.auth1Token,
      orgId: params.orgId,
      nickname: params.nickname,
      tags: params.tags,
      group: params.group,
      password: params.password ?? null,
    });
  },

  /**
   * 使用已存储的 auth1_token 刷新 session_token
   */
  async refreshSession(id: string): Promise<{
    success: boolean;
    session_token: string;
    primary_org_id?: string;
    message?: string;
  }> {
    return await invoke('refresh_devin_session', { id });
  },

  /**
   * 通过已有的 `devin-session-token$...` 前缀 session_token 直接导入 Devin 账号
   *
   * 适用场景：用户从浏览器 localStorage / cookie 拷出有效 session_token 的迁入路径。
   * 仅需 `sessionToken`，后端调 GetCurrentUser 反查 email / api_key / 配额等信息。
   * Devin 扩展字段（account_id / auth1_token / primary_org_id）留空——日常 API 仍可工作，
   * 仅 `refreshSession` 会失败（到期需用户重新获取 session_token）。
   */
  async addAccountBySessionToken(params: {
    sessionToken: string;
    nickname?: string;
    tags: string[];
    group?: string;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_session_token', {
      sessionToken: params.sessionToken,
      nickname: params.nickname,
      tags: params.tags,
      group: params.group,
    });
  },

  /**
   * 通过已有的 Devin `auth1_token`（格式 `auth1_<52字符>`）直接导入账号
   *
   * 适用场景：用户从浏览器 localStorage 的 `devin_auth1_token` 键拷出的迁入路径。
   * 与 `addAccountBySessionToken` 对称，但多保留 auth1_token，让后续 `refreshSession` 能正常工作。
   *
   * 后端内部：`windsurf_post_auth(auth1_token, org_id)` → `GetCurrentUser` 反查 email → 落库。
   *
   * 多组织处理：
   * - `autoSelectPrimaryOrg` 省略或 false：返回 `requires_org_selection=true` + email/auth1_token/orgs，
   *   前端需调 `addAccountWithOrg` 完成二次选 org
   * - `autoSelectPrimaryOrg: true`（批量导入场景）：自动用 primary org 落库
   */
  async addAccountByAuth1Token(params: {
    auth1Token: string;
    orgId?: string;
    nickname?: string;
    tags: string[];
    group?: string;
    autoSelectPrimaryOrg?: boolean;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_auth1_token', {
      auth1Token: params.auth1Token,
      orgId: params.orgId,
      nickname: params.nickname,
      tags: params.tags,
      group: params.group,
      autoSelectPrimaryOrg: params.autoSelectPrimaryOrg,
    });
  },

  // ========== 邮箱验证码（无密码登录） ==========

  /**
   * 发送邮箱验证码（底层接口）
   *
   * - `mode`：`"signup"` 或 `"login"`（无密码邮件登录），默认 `"login"`
   * - `product`：默认 `"Windsurf"`；服务端对 `/email/start` 强制 literal 校验，
   *   仅接受 `"Devin"` / `"Windsurf"`（首字母大写），传小写会返回 422。
   *
   * 服务端向邮箱发送 6 位验证码，并返回 `email_verification_token`，
   * 供后续 `addAccountByEmailLogin` 回传使用。
   */
  async emailStart(
    email: string,
    mode: 'signup' | 'login' = 'login',
    product: 'Windsurf' | 'Devin' = 'Windsurf'
  ): Promise<EmailStartResponse> {
    return await invoke('devin_email_start', { email, mode, product });
  },

  /**
   * 完整流程：邮箱验证码注册新账号 + 建账号
   *
   * 前置：调用方已通过 `emailStart(email, "signup")` 拿到 `email_verification_token`，
   * 并引导用户读取邮件中的 6 位验证码。
   *
   * 服务端 `mode=signup` 完成注册并返回新账号的 auth1_token，
   * 多组织场景下返回 `requires_org_selection=true` + orgs，UI 需调 `addAccountWithOrg` 二次完成。
   */
  async addAccountByRegister(params: {
    email: string;
    emailVerificationToken: string;
    code: string;
    password: string;
    name: string;
    nickname?: string;
    tags: string[];
    group?: string;
    orgId?: string;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_register', params);
  },

  /**
   * 完整流程：无密码邮件验证码登录 + 建账号
   *
   * 用于从 SSO 迁移且无密码、或忘记密码的已存在 Devin 账号。
   * 前置：调用方已通过 `emailStart(email, "login")` 拿到 `email_verification_token`，
   * 并引导用户读取邮件中的 6 位验证码。
   *
   * 服务端 `mode=login` 时**不会创建新账号**，仅返回已有账号的 auth1_token。
   * 多组织场景下返回 `requires_org_selection=true` + orgs，UI 需调 `addAccountWithOrg` 二次完成。
   */
  async addAccountByEmailLogin(params: {
    email: string;
    emailVerificationToken: string;
    code: string;
    nickname?: string;
    tags: string[];
    group?: string;
    orgId?: string;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_email_login', params);
  },

  // ========== Devin 原生站点（app.devin.ai）注册通道 ==========
  //
  // 与上面 Windsurf 侧 `_devin-auth` 通道的关键区别：
  // - 端口：`https://app.devin.ai/api/auth1/*`（Devin 官方后端直连）
  // - `email/start` 请求体不携带 `product` 字段
  // - `email/complete` 请求体不携带 `password` / `name` 字段（纯邮箱验证码建号）
  // - 注册出的账号 JWT 中 `product == "Devin"`，主归属 Devin 产品侧
  // - 后端 `addAccountByNativeRegister` 在注册成功后自动调 WindsurfPostAuth 桥接到 Windsurf，
  //   落库账号既可用 Devin 产品功能（auth1_token），也可用 Windsurf 产品 API（session_token）

  /**
   * 查询 Devin 原生侧（app.devin.ai）邮箱可用的连接方式（可选预检）
   *
   * 响应的 `connections` 数组会额外包含 `windsurf-bridge` 条目，
   * 响应的 `auth_method.method` 字段指示邮箱是否已注册（`"not_found"` = 未注册）。
   */
  async nativeCheckConnections(email: string): Promise<ConnectionsResponse> {
    return await invoke('devin_app_check_connections', { email });
  },

  /**
   * 向 Devin 原生侧发送邮箱验证码（底层接口）
   *
   * @param email 目标邮箱
   * @param mode  `"signup"` 注册新账号；`"login"` 已有 Devin 账号的无密码邮件登录。默认 `"signup"`
   *
   * 返回 `EmailStartResponse`，供后续 `addAccountByNativeRegister` 回传 `email_verification_token`。
   */
  async nativeEmailStart(
    email: string,
    mode: 'signup' | 'login' = 'signup'
  ): Promise<EmailStartResponse> {
    return await invoke('devin_app_email_start', { email, mode });
  },

  /**
   * 提交验证码完成 Devin 原生侧邮件流程（底层接口）
   *
   * 仅在需要纯粹调用 `/api/auth1/email/complete`（不落库、不桥接）时使用；
   * 日常注册请直接用 `addAccountByNativeRegister` 一键完成。
   */
  async nativeEmailComplete(params: {
    emailVerificationToken: string;
    code: string;
    mode: 'signup' | 'login';
  }): Promise<DevinPasswordLoginResponse> {
    return await invoke('devin_app_email_complete', params);
  },

  /**
   * 完整流程：Devin 原生注册 → 自动桥接 Windsurf → 落库为新账号
   *
   * 前置：调用方已通过 `nativeEmailStart(email, "signup")` 拿到 `email_verification_token`，
   * 并引导用户读取邮件中的 6 位验证码。
   *
   * 与 `addAccountByRegister`（Windsurf 侧注册）的差异：
   * - 不需要 `password` 与 `name` 入参（Devin 原生注册是"纯邮箱验证码"建号）
   * - 账号落库时 `password` 字段留空，用户可后续在 Devin 产品侧自行设置密码
   * - JWT 归属为 Devin，后续 Devin 产品功能（auth1_token）可直接使用
   *
   * 多组织场景下返回 `requires_org_selection=true` + orgs，UI 需调 `addAccountWithOrg` 二次完成（与 Windsurf 侧一致）。
   */
  async addAccountByNativeRegister(params: {
    email: string;
    emailVerificationToken: string;
    code: string;
    nickname?: string;
    tags: string[];
    group?: string;
    orgId?: string;
  }): Promise<DevinLoginResult> {
    return await invoke('add_account_by_devin_native_register', params);
  },

  // ==================== Firebase ↔ Devin 账号互转 ====================

  /**
   * 把 Firebase 账号转换为 Devin 登录方式
   *
   * 场景：官方将老 Firebase 账号迁移到 Devin 体系（密码未变），本地帐号卡仍是
   * Firebase 配置。调用后复用账号已存的明文密码走 Devin 登录流程。
   *
   * 返回：
   * - success=true：已切换到 Devin 体系
   * - success=false, already_converted=true：账号已是 Devin 体系
   * - success=false, requires_org_selection=true：多组织需选择，由调用方
   *   弹出组织选择后再次调本方法并传入 orgId
   */
  async convertAccountToDevin(params: {
    id: string;
    orgId?: string;
  }): Promise<{
    success: boolean;
    already_converted?: boolean;
    requires_org_selection?: boolean;
    orgs?: Array<{ id: string; name: string }>;
    email?: string;
    account?: any;
    message?: string;
  }> {
    return await invoke('convert_account_to_devin', {
      id: params.id,
      orgId: params.orgId,
    });
  },

  /**
   * 把 Devin 账号转换为 Firebase 登录方式
   *
   * 场景：官方回调某些帐号到 Firebase 体系、或用户误转后需要还原。
   * 调用后复用账号已存的明文密码走 Firebase signInWithPassword。
   *
   * 返回：
   * - success=true：已切换到 Firebase 体系
   * - success=false, already_converted=true：账号已是 Firebase 体系
   */
  async convertAccountToFirebase(params: {
    id: string;
  }): Promise<{
    success: boolean;
    already_converted?: boolean;
    email?: string;
    account?: any;
    message?: string;
  }> {
    return await invoke('convert_account_to_firebase', { id: params.id });
  },
};

// 系统维护 API
export const systemApi = {
  /**
   * 重置HTTP客户端（用于从网络故障中恢复）
   * 当遇到连续的API请求失败时，可以调用此方法重置HTTP连接池
   */
  async resetHttpClient(): Promise<{ success: boolean; message: string }> {
    return await invoke('reset_http_client');
  }
};
