// ============================================================
// 本地账号管理类型
// ============================================================

/**
 * 带颜色的标签接口
 */
export interface TagWithColor {
  name: string;
  color: string; // RGBA格式，如 "rgba(255, 100, 100, 1)"
}

/**
 * 账户状态类型
 */
export type AccountStatusType = 'normal' | 'inactive' | 'disabled' | 'offline' | 'error';

/**
 * 账号筛选条件
 */
export interface AccountFilter {
  group?: string;
  tags?: string[];
  search?: string;
  // 高级筛选
  remainingQuotaMin?: number;  // 剩余额度最小值
  remainingQuotaMax?: number;  // 剩余额度最大值
  totalQuotaMin?: number;      // 总额度最小值
  totalQuotaMax?: number;      // 总额度最大值
  expiryDaysMin?: number;      // 剩余天数最小值
  expiryDaysMax?: number;      // 剩余天数最大值
  // 日/周配额剩余百分比筛选（0-100，仅 billing_strategy === 2 (QUOTA) 的账号参与）
  dailyQuotaPercentMin?: number;   // 日配额剩余% 最小值
  dailyQuotaPercentMax?: number;   // 日配额剩余% 最大值
  weeklyQuotaPercentMin?: number;  // 周配额剩余% 最小值
  weeklyQuotaPercentMax?: number;  // 周配额剩余% 最大值
  planNames?: string[];        // 套餐名称筛选
  domains?: string[];          // 域名筛选
  statuses?: AccountStatusType[];  // 状态筛选
}

/**
 * 分页配置
 */
export interface PaginationConfig {
  currentPage: number;
  pageSize: number;
  pageSizes: number[];
}

/**
 * 排序字段枚举
 */
export type SortField = 
  | 'email'               // 邮箱/账户名称
  | 'created_at'          // 创建时间
  | 'used_quota'          // 已用积分
  | 'remaining_quota'     // 剩余积分
  | 'token_expires_at'    // Token过期时间
  | 'subscription_expires_at'  // 订阅到期时间
  | 'plan_name'           // 套餐类型
  | 'daily_quota_remaining'    // 日配额剩余% (仅 billing_strategy=2(QUOTA) 有效)
  | 'weekly_quota_remaining';  // 周配额剩余% (仅 billing_strategy=2(QUOTA) 有效)

/**
 * 排序方向枚举
 */
export type SortDirection = 'asc' | 'desc';

/**
 * 排序配置
 */
export interface SortConfig {
  field: SortField;
  direction: SortDirection;
}

export interface Account {
  id: string;
  email: string;
  password?: string; // Optional for updates
  nickname: string;
  tags: string[];
  tagColors?: TagWithColor[]; // 带颜色的标签
  group?: string;
  token?: string;
  refresh_token?: string; // Refresh Token（用于刷新 access_token）
  token_expires_at?: string;
  last_seat_count?: number;
  created_at: string;
  last_login_at?: string;
  status: 'active' | 'inactive' | 'error';
  // 配额和套餐信息（从API获取）
  plan_name?: string;
  used_quota?: number;
  total_quota?: number;
  last_quota_update?: string;
  // 订阅到期时间
  subscription_expires_at?: string;
  // 订阅是否激活 (从 GetCurrentUser API 的 team_info.subscription_active 获取)
  subscription_active?: boolean;
  // Windsurf API Key (从 GetCurrentUser API 的 user.api_key 获取)
  windsurf_api_key?: string;
  // 账户是否被禁用 (从 GetCurrentUser API 的 user.disable_codeium 获取)
  is_disabled?: boolean;
  // 是否为团队所有者（Admin角色，有团队成员的主账号）
  is_team_owner?: boolean;
  // 计费策略 (0=UNSPECIFIED, 1=CREDITS, 2=QUOTA, 3=ACU)
  billing_strategy?: number;
  // 日配额剩余百分比 (0-100，仅 billing_strategy=2(QUOTA) 时有效)
  daily_quota_remaining_percent?: number;
  // 周配额剩余百分比 (0-100，仅 billing_strategy=2(QUOTA) 时有效)
  weekly_quota_remaining_percent?: number;
  // 日配额重置时间 (Unix时间戳秒)
  daily_quota_reset_at_unix?: number;
  // 周配额重置时间 (Unix时间戳秒)
  weekly_quota_reset_at_unix?: number;
  // 额外使用余额 (微美元，除以1e6得到美元)
  overage_balance_micros?: number;
  // 自定义排序顺序（用于拖拽排序）
  sortOrder?: number;

  // ==================== Devin Session 认证字段 ====================
  /** Devin 一级认证令牌（可用于再次换取 session_token） */
  devin_auth1_token?: string;
  /** Devin 账号 ID（格式：account-<32 字符十六进制>） */
  devin_account_id?: string;
  /** Devin 主组织 ID */
  devin_primary_org_id?: string;
  /** 认证提供方："firebase"（默认旧体系）或 "devin"（Devin Session 新体系） */
  auth_provider?: 'firebase' | 'devin';
}

/**
 * Devin 组织条目
 */
export interface WindsurfOrg {
  id: string;
  name: string;
}

/**
 * `devin_email_start` 的响应结构：发送邮箱验证码的底层接口
 *
 * 服务端向邮箱发送 6 位验证码，并回传 `email_verification_token`，供后续
 * `/email/complete` 流程（登录 / 注册 / 忘记密码）作为会话凭证使用。
 */
export interface EmailStartResponse {
  email_verification_token: string;
  [key: string]: any;
}

/**
 * `devin_check_connections` / `devin_app_check_connections` 的响应结构
 *
 * 查询邮箱在 Devin 侧（Windsurf 同源或 app.devin.ai 原生）可用的连接方式。
 * 原始 JSON 透传在 `connections` / `auth_method` 等字段中；服务端可能返回额外字段。
 */
export interface ConnectionsResponse {
  /** 可用的连接方式列表（`email` / `google` / `github` / `windsurf-bridge` / SSO 等） */
  connections?: Array<{
    id: string | null;
    type: string;
    enabled: boolean;
    client_id: string | null;
    [key: string]: any;
  }>;
  /** 邮箱的认证方法判定 */
  auth_method?: {
    /** `"not_found"` | `"auth1"` | 其他 */
    method?: string;
    has_password?: boolean;
    sso_connections?: any[] | null;
    [key: string]: any;
  };
  [key: string]: any;
}

/**
 * `devin_password_login` / `devin_email_complete` / `devin_app_email_complete` 的响应结构
 *
 * 实测响应：`{ "token": "auth1_<52>", "user_id": "user-<32>", "email": "..." }`
 * 字段别名同时兼容 `auth1_token` / `account_id` 等历史命名。
 */
export interface DevinPasswordLoginResponse {
  /** 一级认证令牌（格式：`auth1_<52 字符随机>`） */
  auth1_token: string;
  /** Devin 账号/用户 ID（格式：`user-<32 字符>`） */
  account_id?: string | null;
  /** 服务端回显的邮箱 */
  email?: string | null;
  [key: string]: any;
}

/**
 * add_account_by_devin_login / add_account_by_devin_email_login 的响应结构
 */
export interface DevinLoginResult {
  success: boolean;
  requires_org_selection?: boolean;
  auth1_token?: string;
  orgs?: WindsurfOrg[];
  account?: Account;
  email?: string;
  plan_name?: string;
  used_quota?: number;
  total_quota?: number;
  devin_account_id?: string;
  primary_org_id?: string;
  message?: string;
}

/**
 * CheckUserLoginMethod 的响应（Firebase 侧对邮箱的登录方式判断）
 *
 * 对应 `exa.seat_management_pb.SeatManagementService/CheckUserLoginMethod`
 */
export interface CheckUserLoginMethodResult {
  redirect_url: string;
  disallow_enterprise_user_login: boolean;
  user_exists: boolean;
  is_migrated: boolean;
  has_password: boolean;
}

/**
 * 登录流派嗅探推荐值
 *
 * - `"firebase"`                — 老 Firebase 账号 + 已设密码，走 Firebase 邮箱密码登录
 * - `"devin"`                   — 已迁移或新 Auth1 账号，走 Devin 账密登录（Windsurf 桥接）
 * - `"devin_native"`            — 纯 Devin 原生账号（仅 app.devin.ai 侧有记录）且已设密码（v1.7.6 新增）
 * - `"devin_native_no_password"`— 纯 Devin 原生账号未设密码，需改用邮箱验证码登录（v1.7.6 新增）
 * - `"sso"`                     — 挂接企业 SSO，必须在浏览器中完成 SSO 跳转
 * - `"no_password"`             — 老账号仅用过 Google/GitHub，需用 OAuth 或先重置密码
 * - `"not_found"`               — 邮箱三侧都不存在，需先注册
 * - `"blocked"`                 — 企业用户被限制普通登录
 */
export type LoginMethodRecommendation =
  | 'firebase'
  | 'devin'
  | 'sso'
  | 'no_password'
  | 'not_found'
  | 'blocked'
  | 'devin_native'
  | 'devin_native_no_password';

/**
 * 登录流派嗅探聚合结果
 *
 * 由后端 `sniff_login_method` Tauri 命令返回，聚合三路探测：
 * - Firebase(WS) 侧 `CheckUserLoginMethod`
 * - Windsurf 同源桥接侧 `windsurf.com/_devin-auth/connections`
 * - Devin 原生侧 `app.devin.ai/api/auth1/connections`（v1.7.6 新增）
 */
export interface LoginMethodSniffResult {
  /** 建议的登录流派 */
  recommended: LoginMethodRecommendation;
  /** 面向人的理由说明，可直接展给 UI */
  reason: string;

  // ==== Firebase(WS) 侧原始判定 ====
  user_exists: boolean;
  is_migrated: boolean;
  has_password: boolean;
  redirect_url: string | null;
  disallow_enterprise: boolean;

  // ==== Windsurf 桥接侧原始判定（windsurf.com/_devin-auth/connections）====
  /** Windsurf 桥接侧 `/connections` 的原始 JSON，接口失败或邮箱不存在时为 null */
  devin_connections: Record<string, any> | null;
  /** Windsurf 桥接侧 `auth_method.method`：`"auth1"` | `"not_found"` | null */
  devin_method: string | null;
  /** Windsurf 桥接侧 `auth_method.has_password` 字段 */
  devin_has_password: boolean | null;
  /** Windsurf 桥接侧 `auth_method.sso_connections` 数组是否非空 */
  has_sso_connection: boolean;

  // ==== Devin 原生侧原始判定（app.devin.ai/api/auth1/connections，v1.7.6 新增）====
  /** Devin 原生侧 `/connections` 的原始 JSON，接口失败或邮箱不存在时为 null */
  devin_native_connections: Record<string, any> | null;
  /** Devin 原生侧 `auth_method.method` 字段 */
  devin_native_method: string | null;
  /** Devin 原生侧 `auth_method.has_password` 字段 */
  devin_native_has_password: boolean | null;
}

// ============================================================
// GetCurrentUser API 响应类型（与后端 proto_parser.rs 保持一致）
// ============================================================

/**
 * 用户基本信息 (seat_management_pb.User)
 * 对应后端 UserBasicInfo 结构体
 */
export interface UserBasicInfo {
  api_key: string;           // field 1: API Key (UUID格式，用于API调用身份识别)
  name: string;              // field 2: 用户显示名称
  email: string;             // field 3: 邮箱
  id: string;                // field 6: Firebase UID (用户唯一标识)
  team_id: string;           // field 7: 所属团队ID
  team_status: number;       // field 8: UserTeamStatus (0=未指定,1=待定,2=已批准,3=已拒绝)
  username: string;          // field 9: 用户名 (如 righteously-handsome-kite-82267)
  timezone: string;          // field 10: 时区 (如 Asia/Shanghai)
  public_profile_enabled: boolean;  // field 11: 是否公开资料
  pro: boolean;              // field 13: 是否Pro用户
  disable_codeium: boolean;  // field 16: 是否禁用Codeium
  newsletter: boolean;       // field 19: 是否订阅邮件
  disabled_telemetry: boolean; // field 20: 是否禁用遥测
  signup_stage?: string;     // field 22: 注册阶段
  used_trial: boolean;       // field 25: 是否已使用试用
  used_prompt_credits: number; // field 28: 已用Prompt积分
  used_flow_credits: number;   // field 29: 已用Flow积分
  referral_code?: string;    // field 30: 推荐码
  // Timestamp fields (Unix秒级时间戳)
  signup_time?: number;              // field 4: 注册时间
  last_update_time?: number;         // field 5: 最后更新时间
  first_windsurf_use_time?: number;  // field 26: 首次使用Windsurf时间
  windsurf_pro_trial_end_time?: number; // field 27: Pro试用结束时间
}

/**
 * 团队信息 (seat_management_pb.Team)
 * 对应后端 TeamInfo 结构体
 */
export interface TeamInfo {
  id: string;                        // field 1: 团队ID
  name: string;                      // field 2: 团队名称
  signup_time?: number;              // field 3: 团队创建时间
  invite_id?: string;                // field 4: 邀请码ID
  used_trial: boolean;               // field 5: 是否已使用试用
  stripe_subscription_id?: string;   // field 6: Stripe订阅ID
  subscription_active: boolean;      // field 7: 订阅是否激活
  stripe_customer_id?: string;       // field 8: Stripe客户ID
  current_billing_period_start?: number; // field 9: 计费周期开始时间
  num_seats_current_billing_period: number; // field 10: 当前计费周期席位数
  attribution_enabled: boolean;      // field 11: 是否启用归因
  sso_provider_id?: string;          // field 12: SSO提供商ID
  offers_enabled: boolean;           // field 13: 是否启用优惠
  teams_tier: number;                // field 14: TeamsTier (1=Teams,2=Pro,3=Enterprise...)
  flex_credit_quota: number;         // field 15: Flex积分配额
  used_flow_credits: number;         // field 16: 已用Flow积分
  used_prompt_credits: number;       // field 17: 已用Prompt积分
  current_billing_period_end?: number; // field 18: 计费周期结束时间
  num_cascade_seats: number;         // field 19: Cascade席位数
  cascade_usage_month_start?: number; // field 20: Cascade使用月开始
  cascade_usage_month_end?: number;   // field 21: Cascade使用月结束
  cascade_seat_type: number;         // field 22: CascadeSeatType枚举
  top_up_enabled: boolean;           // field 23: 是否启用充值
  monthly_top_up_amount: number;     // field 24: 月度充值金额
  top_up_spent: number;              // field 25: 已花费充值
  top_up_increment: number;          // field 26: 充值增量
  used_flex_credits: number;         // field 27: 已用Flex积分
  num_users: number;                 // 团队成员数量
}

/**
 * 套餐信息 (codeium_common_pb.PlanInfo)
 * 对应后端 PlanInfo 结构体
 */
export interface PlanInfo {
  teams_tier: number;                // field 1: TeamsTier枚举
  plan_name: string;                 // field 2: 套餐名称 (如 "Teams")
  has_autocomplete_fast_mode: boolean;  // field 3: 快速自动补全
  allow_sticky_premium_models: boolean; // field 4: 允许使用高级模型
  has_forge_access: boolean;         // field 5: Forge访问权限
  max_num_premium_chat_messages: number; // field 6: 最大高级聊天消息数
  max_num_chat_input_tokens: number;    // field 7: 最大聊天输入tokens
  max_custom_chat_instruction_characters: number; // field 8: 最大自定义指令字符
  max_num_pinned_context_items: number;  // field 9: 最大固定上下文项数
  max_local_index_size: number;      // field 10: 最大本地索引大小
  disable_code_snippet_telemetry: boolean; // field 11: 禁用代码片段遥测
  monthly_prompt_credits: number;    // field 12: 月度Prompt积分
  monthly_flow_credits: number;      // field 13: 月度Flow积分
  monthly_flex_credit_purchase_amount: number; // field 14: 月度Flex积分购买额度
  allow_premium_command_models: boolean; // field 15: 允许高级命令模型
  is_enterprise: boolean;            // field 16: 是否企业版
  is_teams: boolean;                 // field 17: 是否团队版
  can_buy_more_credits: boolean;     // field 18: 是否可购买更多积分
  cascade_web_search_enabled: boolean; // field 19: Cascade网络搜索
  can_customize_app_icon: boolean;   // field 20: 可自定义应用图标
  cascade_can_auto_run_commands: boolean; // field 22: Cascade可自动运行命令
  has_tab_to_jump: boolean;          // field 23: Tab跳转功能
  can_generate_commit_messages: boolean; // field 25: 可生成提交消息
  max_unclaimed_sites: number;       // field 26: 最大未认领站点数
  knowledge_base_enabled: boolean;   // field 27: 知识库功能
  can_share_conversations: boolean;  // field 28: 可分享对话
  can_allow_cascade_in_background: boolean; // field 29: 允许Cascade后台运行
  browser_enabled: boolean;          // field 31: 浏览器功能
}

/**
 * 用户角色信息 (seat_management_pb.UserRole)
 * 对应后端 UserRole 结构体
 */
export interface UserRole {
  api_key: string;           // field 1: API Key
  roles: string[];           // field 2: 角色列表
  role_id: string;           // field 3: 角色ID (如 "root.admin")
  role_name: string;         // field 4: 角色名称 (如 "Admin")
}

/**
 * 订阅信息
 * 对应后端 SubscriptionInfo 结构体
 */
export interface SubscriptionInfo {
  id: string;
  email: string;
  stripe_subscription_id: string;
  stripe_customer_id: string;
  seats: number;
  usage: number;
  quota: number;
  used_quota: number;
  expires_at?: number;       // Unix时间戳（秒）
  subscription_active: boolean;
  on_trial: boolean;
}

/**
 * GetCurrentUser API 完整响应
 * 对应后端 UserInfo 结构体
 */
export interface UserDetails {
  user: UserBasicInfo;
  roles?: string;                    // 角色字符串 (如 "root.admin")
  subscription?: SubscriptionInfo;
  plan?: PlanInfo;
  role?: UserRole;                   // 角色详情
  admin?: UserRole;                  // 兼容旧代码
  is_root_admin: boolean;
  team?: TeamInfo;
  permissions?: any;                 // 权限对象
  plan_features?: any;               // 功能配置
}

// ============================================================
// 枚举类型定义
// ============================================================

/**
 * 团队层级枚举 (codeium_common_pb.TeamsTier)
 */
export enum TeamsTier {
  UNSPECIFIED = 0,
  TEAMS = 1,
  PRO = 2,
  ENTERPRISE_SAAS = 3,
  HYBRID = 4,
  ENTERPRISE_SELF_HOSTED = 5,
  WAITLIST_PRO = 6,
  TEAMS_ULTIMATE = 7,
  PRO_ULTIMATE = 8,
  TRIAL = 9,
  ENTERPRISE_SELF_SERVE = 10
}

/**
 * 用户团队状态枚举 (codeium_common_pb.UserTeamStatus)
 */
export enum UserTeamStatus {
  UNSPECIFIED = 0,
  PENDING = 1,
  APPROVED = 2,
  REJECTED = 3
}

/**
 * Cascade席位类型枚举 (seat_management_pb.CascadeSeatType)
 */
export enum CascadeSeatType {
  UNSPECIFIED = 0,
  ENTRY = 1,
  STANDARD = 2
}

export type AccountStatus = 'active' | 'inactive' | { error: string };

export interface Settings {
  auto_refresh_token: boolean;
  seat_count_options: number[];
  retry_times: number;
  theme: string;
  concurrent_limit: number;
  show_seats_result_dialog: boolean;  // 是否显示座位更新结果对话框
  autoOpenPaymentLinkInWebview?: boolean;  // 是否自动在内置浏览器中打开支付链接
  autoFillPaymentForm?: boolean;  // 是否自动填写支付表单
  autoSubmitPaymentForm?: boolean;  // 是否自动提交支付表单
  paymentPageDelay?: number;  // 支付页面加载延迟（秒）
  showVirtualCardInfo?: boolean;  // 是否显示虚拟卡信息弹窗
  customCardBin?: string;  // 自定义卡头（4-12位数字）
  customCardBinRange?: string;  // 卡段范围（如 626200-626300）
  cardBindRetryTimes?: number;  // 绑卡失败重试次数
  testModeEnabled?: boolean;  // 测试模式：自动收集成功的卡BIN
  useLocalSuccessBins?: boolean;  // 使用本地成功BIN池
  testModeLastBin?: string | null;  // 测试模式下上次使用的BIN（用于顺序遍历）
  seamlessSwitchEnabled?: boolean;  // 是否启用无感换号
  windsurfClientType?: 'windsurf' | 'windsurf-next';  // 客户端类型
  windsurfPath?: string | null;  // Windsurf安装路径
  patchBackupPath?: string | null;  // 补丁备份文件路径
  autoOpenBrowser?: boolean;  // 是否自动打开浏览器
  browserMode?: 'incognito' | 'normal';  // 浏览器模式
  privacyMode?: boolean;  // 隐私模式，隐藏邮箱地址
  unlimitedConcurrentRefresh?: boolean;  // 自动刷新Token时不限制并发数
  proxyEnabled?: boolean;  // 是否启用代理
  proxyUrl?: string | null;  // 代理地址 (如 http://127.0.0.1:7890)
  useLightweightApi?: boolean;  // 使用轻量级API(GetPlanStatus)获取配额信息
  subscriptionPlan?: number;  // 订阅计划: 0=Free, 1=Teams, 2=Pro, 3=Enterprise SaaS, 4=Hybrid, 5=Enterprise Self-Hosted, 6=Waitlist Pro, 7=Teams Ultimate, 8=Pro Ultimate, 9=Trial, 10=Enterprise Self-Serve, 11=Enterprise SaaS Pooled, 12=Devin Enterprise, 14=Devin Teams, 15=Devin Teams V2, 16=Devin Pro, 17=Devin Max, 18=Max, 19=Devin Free, 20=Devin Trial
  paymentPeriod?: number;  // 支付周期: 1=月付, 2=年付
  startTrial?: boolean;  // 是否以试用方式开始订阅
  teamName?: string;  // Teams 计划的团队名称
  seatCount?: number;  // Teams 计划的席位数量
  // ==================== 额度耗尽自动切换（Fork 新增） ====================
  autoSwitchEnabled?: boolean;  // 是否启用额度耗尽自动切换
  autoSwitchCheckInterval?: number;  // 检查间隔（分钟）
  autoSwitchThresholdPercent?: number;  // 使用率阈值（%），>= 时触发切换
  autoSwitchRemainingThreshold?: number;  // 剩余积分阈值，0 表示忽略此条件
  autoSwitchStrategy?: 'most_remaining' | 'round_robin';  // 选号策略
  autoSwitchPreferSameProvider?: boolean;  // 优先选择同认证体系的账号
}

/**
 * 全局标签定义（带默认颜色）
 */
export interface GlobalTag {
  name: string;
  color: string; // 默认颜色，RGBA或HEX格式
}

export interface OperationLog {
  id: string;
  timestamp: string;
  account_id?: string;
  account_email?: string;
  operation_type: OperationType;
  status: OperationStatus;
  message: string;
  details?: any;
}

export type OperationType = 
  | 'login'
  | 'refresh_token'
  | 'reset_credits'
  | 'update_seats'
  | 'get_billing'
  | 'update_plan'
  | 'add_account'
  | 'delete_account'
  | 'edit_account'
  | 'batch_operation';

export type OperationStatus = 'success' | 'failed' | 'pending' | 'processing';

export interface UpdateSeatsResult {
  success: boolean;
  attempts: AttemptResult[];
}

export interface AttemptResult {
  attempt: number;
  status_code?: number;
  raw_response?: string;
  error?: string;
  timestamp: string;
}

export interface BillingInfo {
  success: boolean;
  status_code?: number;
  raw_response?: string;
  error?: string;
  timestamp: string;
}

export interface BatchResult {
  results: Array<{
    id: string;
    success: boolean;
    data?: any;
    error?: string;
  }>;
  success_count?: number;
  total_count?: number;
}
