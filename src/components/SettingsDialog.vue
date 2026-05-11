<template>
  <el-dialog
    v-model="uiStore.showSettingsDialog"
    title="设置"
    width="700px"
  >
    <el-tabs v-model="activeTab" type="border-card">
      <!-- 基础设置标签页 -->
      <el-tab-pane label="基础设置" name="basic" lazy>
        <el-form :model="settings" label-width="140px">
          <el-form-item label="自动刷新Token">
            <el-switch v-model="settings.auto_refresh_token" />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，Token过期时将自动刷新
            </div>
          </el-form-item>
          
          <el-form-item label="全量并发刷新" v-if="settings.auto_refresh_token">
            <el-switch v-model="settings.unlimitedConcurrentRefresh" />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，自动刷新Token时所有账号同时并发，不受并发限制，可大幅节省时间
            </div>
          </el-form-item>
          
          <!-- 座位数选项 - simple 版本已禁用
          <el-form-item label="座位数选项">
            <el-input
              v-model="seatCountOptionsInput"
              placeholder="例如: 18, 19, 20"
              style="width: 200px;"
              @blur="parseSeatCountOptions"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              积分重置时轮番使用的座位数，用逗号分隔（如：18, 19, 20）
            </div>
          </el-form-item>
          -->
          
          <el-form-item label="重试次数">
            <el-input-number
              v-model="settings.retry_times"
              :min="1"
              :max="5"
              :step="1"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              API调用失败时的重试次数
            </div>
          </el-form-item>
          
          <el-form-item label="并发限制">
            <el-input-number
              v-model="settings.concurrent_limit"
              :min="1"
              :max="10"
              :step="1"
              :disabled="settings.unlimitedConcurrentRefresh"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              {{ settings.unlimitedConcurrentRefresh ? '已开启全量并发刷新，此设置不影响自动刷新' : '批量操作时的最大并发数' }}
            </div>
          </el-form-item>
          
          <el-form-item label="界面主题">
            <ThemeSelector v-model="settings.theme" />
          </el-form-item>
          
          <el-form-item label="显示详细结果">
            <el-switch 
              v-model="settings.show_seats_result_dialog"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，积分重置时将显示详细的座位更新结果对话框
            </div>
          </el-form-item>
          
          <el-form-item label="隐私模式">
            <el-switch 
              v-model="settings.privacyMode"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，所有邮箱地址将显示为随机字符，保护隐私（适用于截图演示）
            </div>
          </el-form-item>
          
          <el-divider content-position="left">网络维护</el-divider>
          
          <el-form-item label="轻量级API">
            <el-switch 
              v-model="settings.useLightweightApi"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启时使用 GetPlanStatus 获取配额信息（更快），关闭时使用 GetCurrentUser（数据更完整）
            </div>
          </el-form-item>
          
          <el-form-item label="启用代理">
            <el-switch 
              v-model="settings.proxyEnabled"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，登录和刷新Token等 Google API 请求将通过代理进行
            </div>
          </el-form-item>
          
          <el-form-item label="代理地址" v-if="settings.proxyEnabled">
            <el-input
              v-model="settings.proxyUrl"
              placeholder="http://127.0.0.1:7890"
              style="width: 280px;"
              clearable
            >
              <template #prefix>
                <el-icon><Connection /></el-icon>
              </template>
            </el-input>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              支持 HTTP/HTTPS/SOCKS5 代理，格式：http://host:port 或 socks5://host:port
            </div>
          </el-form-item>
          
          <el-form-item label="重置网络连接">
            <el-button 
              type="warning" 
              @click="handleResetHttpClient"
              :loading="resettingHttp"
            >
              重置HTTP客户端
            </el-button>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              当遇到连续的API请求失败时，可点击此按钮重置网络连接池
            </div>
          </el-form-item>
        </el-form>
      </el-tab-pane>
      
      <!-- 支付设置标签页 -->
      <el-tab-pane label="支付设置" name="payment" lazy>
        <el-form :model="settings" label-width="140px">
          <el-divider content-position="left">订阅计划设置</el-divider>
          
          <el-form-item label="订阅计划">
            <el-select v-model="settings.subscriptionPlan" style="width: 100%;">
              <el-option-group label="Windsurf 常用">
                <el-option label="Pro 专业版" :value="2" />
                <el-option label="Max 旗舰版" :value="18" />
                <el-option label="Teams 团队版" :value="1" />
                <el-option label="Trial 试用版" :value="9" />
                <el-option label="Free 免费版" :value="0" />
              </el-option-group>
              <el-option-group label="Windsurf Ultimate">
                <el-option label="Pro Ultimate" :value="8" />
                <el-option label="Teams Ultimate" :value="7" />
              </el-option-group>
              <el-option-group label="Enterprise">
                <el-option label="Enterprise SaaS" :value="3" />
                <el-option label="Enterprise Self-Serve" :value="10" />
                <el-option label="Enterprise Self-Hosted" :value="5" />
                <el-option label="Enterprise SaaS Pooled" :value="11" />
                <el-option label="Hybrid" :value="4" />
              </el-option-group>
              <el-option-group label="Devin">
                <el-option label="Devin Pro" :value="16" />
                <el-option label="Devin Max" :value="17" />
                <el-option label="Devin Teams" :value="14" />
                <el-option label="Devin Teams V2" :value="15" />
                <el-option label="Devin Enterprise" :value="12" />
                <el-option label="Devin Free" :value="19" />
                <el-option label="Devin Trial" :value="20" />
              </el-option-group>
              <el-option-group label="其他">
                <el-option label="Waitlist Pro" :value="6" />
              </el-option-group>
            </el-select>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              选择要订阅的计划类型，Pro 计划需要完成 Turnstile 人机验证
            </div>
          </el-form-item>
          
          <el-form-item label="支付周期">
            <el-select v-model="settings.paymentPeriod" style="width: 100%;">
              <el-option label="月付" :value="1" />
              <el-option label="年付" :value="2" />
            </el-select>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              年付通常可享受优惠价格
            </div>
          </el-form-item>
          
          <el-form-item label="开启试用">
            <el-switch 
              v-model="settings.startTrial"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              以试用方式开始订阅，关闭则直接进入付费订阅
            </div>
          </el-form-item>
          
          <el-form-item label="团队名称" v-if="[1, 3, 4, 5, 7, 10, 11, 12, 14, 15].includes(settings.subscriptionPlan)">
            <el-input 
              v-model="settings.teamName" 
              placeholder="输入团队名称（Teams类计划必填）"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              Teams 类计划需要填写团队名称
            </div>
          </el-form-item>
          
          <el-form-item label="席位数量" v-if="[1, 3, 4, 5, 7, 10, 11, 12, 14, 15].includes(settings.subscriptionPlan)">
            <el-input-number 
              v-model="settings.seatCount" 
              :min="1" 
              :max="1000"
              style="width: 100%;"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              Teams 计划的席位数量
            </div>
          </el-form-item>
          
          <el-divider content-position="left">支付页面设置</el-divider>
          
          <el-form-item label="自动打开支付页面">
            <el-switch 
              v-model="settings.autoOpenPaymentLinkInWebview"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，获取绑卡链接成功时将自动在内置浏览器窗口中打开支付页面（隐私模式，不保存任何数据）
            </div>
          </el-form-item>
          
          <el-divider content-position="left">外部浏览器设置</el-divider>
          
          <el-form-item label="自动打开外部浏览器">
            <el-switch 
              v-model="settings.autoOpenBrowser"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，获取绑卡链接时将自动在外部浏览器中打开（无需点击确认）
            </div>
          </el-form-item>
          
          <el-form-item label="浏览器模式">
            <el-radio-group v-model="settings.browserMode">
              <el-radio-button label="incognito">无痕模式</el-radio-button>
              <el-radio-button label="normal">普通模式</el-radio-button>
            </el-radio-group>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              选择打开外部浏览器时使用的模式（无痕模式更安全，推荐使用）
            </div>
          </el-form-item>
          
          <el-divider content-position="left">自动填写设置</el-divider>
          
          <el-form-item label="自动填写支付表单">
            <el-switch 
              v-model="settings.autoFillPaymentForm"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，将自动使用虚拟卡信息填写Stripe支付表单（仅用于测试）
            </div>
          </el-form-item>
          
          <el-form-item label="显示虚拟卡信息">
            <el-switch 
              v-model="settings.showVirtualCardInfo"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，自动填写表单时会弹窗显示生成的虚拟卡信息
            </div>
          </el-form-item>
          
          <el-form-item label="自动提交表单">
            <el-switch 
              v-model="settings.autoSubmitPaymentForm"
              active-text="开启"
              inactive-text="关闭"
              :disabled="!settings.autoFillPaymentForm"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，表单填写完成后将自动提交（谨慎使用）
            </div>
          </el-form-item>
          
          <el-form-item label="支付页面延迟(秒)">
            <el-input-number
              v-model="settings.paymentPageDelay"
              :min="1"
              :max="10"
              :step="1"
              :disabled="!settings.autoFillPaymentForm"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              等待多少秒后开始自动填写表单
            </div>
          </el-form-item>
          
          <el-form-item label="自定义卡头">
            <el-input
              v-model="settings.customCardBin"
              placeholder="请输入4-12位数字"
              maxlength="12"
              @input="validateCardBin"
            >
              <template #append>
                <el-button @click="resetCardBin">恢复默认</el-button>
              </template>
            </el-input>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              设置虚拟卡的前缀（BIN），必须是4-12位数字，默认为626202
            </div>
          </el-form-item>
          
          <el-form-item label="卡段范围（可选）">
            <el-input
              v-model="settings.customCardBinRange"
              placeholder="如：626200-626300"
              @input="validateCardBinRange"
            >
              <template #append>
                <el-button @click="clearCardBinRange">清除</el-button>
              </template>
            </el-input>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              设置卡段范围后，绑卡时将从范围内随机选择一个BIN。格式：起始BIN-结束BIN
            </div>
          </el-form-item>
          
          <el-form-item label="绑卡失败重试次数">
            <el-input-number
              v-model="settings.cardBindRetryTimes"
              :min="0"
              :max="20"
              :step="1"
              controls-position="right"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              绑卡失败后自动重新生成卡号重试的次数，设为0则不重试
            </div>
          </el-form-item>
          
          <el-divider content-position="left">卡BIN池功能</el-divider>
          
          <el-form-item label="测试模式">
            <div style="display: flex; align-items: center; gap: 10px;">
              <el-switch v-model="settings.testModeEnabled" />
              <el-button 
                size="small" 
                type="warning" 
                @click="resetTestModeProgress"
                :disabled="!testModeProgress"
              >
                重置进度
              </el-button>
            </div>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，按顺序遍历卡BIN范围，并收集成功的BIN（池数量：{{ successBinCount }}）
              <span v-if="testModeProgress" style="color: #67C23A;">
                <br/>当前进度：{{ testModeProgress }}
              </span>
            </div>
          </el-form-item>
          
          <el-form-item label="使用本地BIN池">
            <el-switch v-model="settings.useLocalSuccessBins" :disabled="successBinCount === 0" />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，自动从本地成功BIN池中随机获取卡BIN生成卡号
            </div>
          </el-form-item>
          
          <el-form-item label="BIN池管理">
            <el-button-group>
              <el-button size="small" @click="viewSuccessBins" :disabled="successBinCount === 0">
                查看BIN池
              </el-button>
              <el-button size="small" type="danger" @click="clearSuccessBins" :disabled="successBinCount === 0">
                清空BIN池
              </el-button>
            </el-button-group>
          </el-form-item>
          
          <el-alert
            title="重要提示"
            type="warning"
            :closable="false"
            show-icon
            style="margin-top: 20px;"
          >
            <template #default>
              <div style="font-size: 12px; line-height: 1.6;">
                <p>🔒 内置浏览器使用隐私模式，不会保存任何浏览数据、Cookies或历史记录。</p>
                <p>⚠️ 虚拟卡信息生成功能仅用于测试目的，请勿用于实际支付。</p>
                <p>⚠️ 使用本功能时，请确保遵守Stripe及相关支付服务的使用条款。</p>
                <p>⚠️ 不要将生成的虚拟卡信息用于任何欺诈或非法用途。</p>
              </div>
            </template>
          </el-alert>
        </el-form>
      </el-tab-pane>
      
      <!-- 无感换号标签页 -->
      <el-tab-pane label="无感换号" name="seamless" lazy>
        <el-form :model="settings" label-width="140px">
          <el-form-item label="客户端类型">
            <el-select
              v-model="settings.windsurfClientType"
              style="width: 200px;"
              @change="handleClientTypeChange"
            >
              <el-option label="Windsurf" value="windsurf" />
              <el-option label="Windsurf - Next" value="windsurf-next" />
            </el-select>
          </el-form-item>
          
          <el-form-item label="安装路径">
            <el-input
              v-model="windsurfPath"
              placeholder="请输入或点击自动检测获取路径"
              @blur="handlePathChange"
            >
              <template #append>
                <el-button-group>
                  <el-button @click="detectWindsurfPath" :loading="detectingPath">
                    自动检测
                  </el-button>
                  <el-button @click="browseWindsurfPath">
                    浏览
                  </el-button>
                </el-button-group>
              </template>
            </el-input>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              可手动输入路径或从开始菜单自动检测 {{ settings.windsurfClientType === 'windsurf-next' ? 'Windsurf - Next' : 'Windsurf' }} 安装路径
            </div>
          </el-form-item>
          
          <el-form-item label="启用无感换号">
            <el-switch 
              v-model="settings.seamlessSwitchEnabled"
              active-text="开启"
              inactive-text="关闭"
              :loading="patchLoading"
              @change="handleSeamlessSwitch"
              :disabled="!windsurfPath"
            />
          </el-form-item>
          
          <el-form-item label="补丁状态">
            <div class="patch-status-block">
              <!-- 汇总 tag + 操作按钮 -->
              <div class="patch-status-header">
                <el-tag :type="patchSummary.type">{{ patchSummary.label }}</el-tag>
                <el-button
                  v-if="canUpgrade"
                  type="warning"
                  size="small"
                  :loading="patchLoading"
                  @click="handleUpgradePatch"
                >
                  升级补丁
                </el-button>
                <el-button
                  v-if="windsurfPath"
                  size="small"
                  @click="checkPatchStatus"
                >
                  重新检测
                </el-button>
              </div>
              <!-- 子项 checklist（有路径且无 IO 错误时展示） -->
              <div
                v-if="windsurfPath && !patchStatus.error"
                class="patch-checklist"
              >
                <div
                  v-for="item in patchItems"
                  :key="item.key"
                  class="patch-checklist-item"
                  :class="{ 'is-applied': item.applied }"
                >
                  <el-icon v-if="item.applied" class="patch-checklist-icon is-applied">
                    <Check />
                  </el-icon>
                  <el-icon v-else class="patch-checklist-icon">
                    <Close />
                  </el-icon>
                  <span>{{ item.label }}</span>
                </div>
              </div>
            </div>
          </el-form-item>
          
          <el-alert
            title="功能说明"
            type="info"
            :closable="false"
            show-icon
            style="margin-top: 20px;"
          >
            <template #default>
              <div style="font-size: 12px; line-height: 1.6;">
                <p>🚀 无感换号功能：实现 Windsurf / Windsurf - Next 账号无感切换</p>
                <p>⚠️ 注意：开启/关闭时若客户端正在运行则自动重启，未运行则不重启</p>
              </div>
            </template>
          </el-alert>
          
          <el-divider content-position="left">Windsurf 伟哥</el-divider>
          
          <el-form-item label="启用伟哥功能">
            <el-switch 
              v-model="settings.cunzhiEnabled"
              active-text="开启"
              inactive-text="关闭"
              :loading="cunzhiLoading"
              @change="handleCunzhiSwitch"
            />
          </el-form-item>
          
          <el-form-item label="寸止状态">
            <el-tag v-if="cunzhiStatus.installed" type="success">已安装</el-tag>
            <el-tag v-else-if="cunzhiStatus.error" type="danger">{{ cunzhiStatus.error }}</el-tag>
            <el-tag v-else type="info">未安装</el-tag>
            <el-button 
              v-if="cunzhiStatus.installed" 
              size="small" 
              style="margin-left: 10px;"
              @click="checkCunzhiStatus"
            >
              重新检测
            </el-button>
          </el-form-item>
          
          <el-alert
            title="伟哥功能说明"
            type="success"
            :closable="false"
            show-icon
            style="margin-top: 10px;"
          >
            <template #default>
              <div style="font-size: 12px; line-height: 1.6;">
                <p>💊 伟哥功能：防止 AI 擅自结束对话，让你掌控对话节奏</p>
                <p>⚠️ 注意：开启/关闭后需要重启 Windsurf 生效</p>
              </div>
            </template>
          </el-alert>
        </el-form>
      </el-tab-pane>

      <!-- 额度耗尽自动切换标签页（Fork 新增） -->
      <el-tab-pane label="自动切换" name="auto-switch" lazy>
        <el-form :model="settings" label-width="160px">
          <el-form-item label="启用自动切换">
            <el-switch
              v-model="settings.autoSwitchEnabled"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后，应用会周期性检查当前活跃 Windsurf 账号的额度，达到阈值时自动切换到下一个账号
            </div>
          </el-form-item>

          <el-alert
            v-if="settings.autoSwitchEnabled && !settings.seamlessSwitchEnabled"
            type="warning"
            :closable="false"
            show-icon
            style="margin-bottom: 16px;"
          >
            <template #title>
              建议先启用「无感换号」补丁
            </template>
            <template #default>
              <div style="font-size: 12px; line-height: 1.6;">
                未启用无感换号时，自动切换会触发 Windsurf 重启；启用补丁后即可在 IDE 内无感换号。
              </div>
            </template>
          </el-alert>

          <el-form-item label="检查间隔" v-if="settings.autoSwitchEnabled">
            <el-input-number
              v-model="settings.autoSwitchCheckInterval"
              :min="1"
              :max="1440"
              :step="1"
            />
            <span style="margin-left: 10px; color: #909399;">分钟</span>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              每隔多少分钟检查一次当前账号配额（建议 5 分钟，过短会增加 API 请求频率）
            </div>
          </el-form-item>

          <el-form-item label="使用率阈值" v-if="settings.autoSwitchEnabled">
            <el-input-number
              v-model="settings.autoSwitchThresholdPercent"
              :min="50"
              :max="100"
              :step="1"
            />
            <span style="margin-left: 10px; color: #909399;">%</span>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              当前账号使用率 ≥ 此值时触发切换（默认 95%，过低可能频繁切号）
            </div>
          </el-form-item>

          <el-form-item label="剩余积分阈值" v-if="settings.autoSwitchEnabled">
            <el-input-number
              v-model="settings.autoSwitchRemainingThreshold"
              :min="0"
              :max="100000"
              :step="100"
            />
            <span style="margin-left: 10px; color: #909399;">积分（0=禁用此条件）</span>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              额外限制：剩余积分 ≤ 此值时才触发切换（与使用率阈值同时满足才切）。设为 0 表示只看使用率
            </div>
          </el-form-item>

          <el-form-item label="选号策略" v-if="settings.autoSwitchEnabled">
            <el-radio-group v-model="settings.autoSwitchStrategy">
              <el-radio value="most_remaining">剩余配额最多</el-radio>
              <el-radio value="round_robin">轮换（最久未用）</el-radio>
            </el-radio-group>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              「剩余配额最多」优先选最富的账号；「轮换」按上次登录时间升序，最久没用的优先
            </div>
          </el-form-item>

          <el-form-item label="同体系优先" v-if="settings.autoSwitchEnabled">
            <el-switch
              v-model="settings.autoSwitchPreferSameProvider"
              active-text="开启"
              inactive-text="关闭"
            />
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              开启后优先选择与当前账号同认证体系（Firebase/Devin）的账号，无候选时自动降级到全部账号
            </div>
          </el-form-item>

          <el-divider content-position="left">手动测试</el-divider>

          <el-form-item label="立即检查">
            <el-button
              type="primary"
              :loading="autoSwitchTesting"
              :disabled="!settings.autoSwitchEnabled"
              @click="handleAutoSwitchTest"
            >
              立即检查并切换
            </el-button>
            <div style="margin-top: 5px; color: #909399; font-size: 12px;">
              手动触发一次检查，用于验证当前配置是否生效
            </div>
          </el-form-item>

          <el-alert
            type="info"
            :closable="false"
            show-icon
            style="margin-top: 16px;"
          >
            <template #title>
              <span style="font-weight: bold;">工作原理</span>
            </template>
            <template #default>
              <div style="font-size: 12px; line-height: 1.8;">
                <p>1. 定时器读取当前 Windsurf 中活跃的账号</p>
                <p>2. 调用 GetPlanStatus API 实时查询配额（不读本地缓存）</p>
                <p>3. 同时满足「使用率 ≥ 阈值」和「剩余 ≤ 阈值」时触发切换</p>
                <p>4. 按选号策略从账号库挑出下一个可用账号，复用切号流程</p>
                <p>5. 切换成功/失败都会写入操作日志，可在「日志」中查看</p>
              </div>
            </template>
          </el-alert>
        </el-form>
      </el-tab-pane>

      <!-- 备份设置标签页 -->
      <el-tab-pane label="备份设置" name="backup" lazy>
        <el-form :model="settings" label-width="140px">
          <el-form-item label="自动备份">
            <el-switch v-model="settings.autoBackupEnabled" />
            <span style="margin-left: 10px; color: #909399; font-size: 12px;">
              启用后将按设定间隔自动备份数据
            </span>
          </el-form-item>
          
          <el-form-item label="备份间隔">
            <el-input-number
              v-model="settings.backupInterval"
              :min="1"
              :max="1440"
              :step="5"
              :disabled="!settings.autoBackupEnabled"
            />
            <span style="margin-left: 10px; color: #909399;">分钟</span>
          </el-form-item>
          
          <el-form-item label="最大备份数">
            <el-input-number
              v-model="settings.backupMaxCount"
              :min="1"
              :max="100"
            />
            <span style="margin-left: 10px; color: #909399;">份（超出后自动删除最早的备份）</span>
          </el-form-item>
          
          <el-divider content-position="left">手动操作</el-divider>
          
          <el-form-item label="立即备份">
            <el-button type="primary" @click="handleManualBackup" :loading="backupLoading">
              创建备份
            </el-button>
          </el-form-item>
          
          <el-form-item label="备份列表">
            <el-button @click="handleShowBackups" :loading="loadingBackups">
              查看备份
            </el-button>
          </el-form-item>
          
          <el-alert type="info" :closable="false" style="margin-top: 15px;">
            <template #title>
              <span style="font-weight: bold;">备份说明</span>
            </template>
            <template #default>
              <div style="line-height: 1.8;">
                <p>备份文件保存在应用数据目录的 <code>backups</code> 文件夹中</p>
                <p>包含以下数据：账号信息、分组、标签、设置等</p>
              </div>
            </template>
          </el-alert>
        </el-form>
        
        <!-- 备份列表对话框 -->
        <el-dialog
          v-if="showBackupsDialog"
          v-model="showBackupsDialog"
          title="备份列表"
          width="600px"
          append-to-body
        >
          <el-table :data="backupList" v-loading="loadingBackups" max-height="400">
            <el-table-column prop="name" label="文件名" />
            <el-table-column label="大小" width="100">
              <template #default="{ row }">
                {{ formatFileSize(row.size) }}
              </template>
            </el-table-column>
            <el-table-column label="创建时间" width="180">
              <template #default="{ row }">
                {{ formatBackupTime(row.name) }}
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120">
              <template #default="{ row }">
                <el-button type="primary" size="small" link @click="handleRestoreBackup(row)">
                  恢复
                </el-button>
                <el-button type="danger" size="small" link @click="handleDeleteBackup(row)">
                  删除
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-dialog>
      </el-tab-pane>
    </el-tabs>
    
    <template #footer>
      <el-button @click="handleClose">取消</el-button>
      <el-button type="primary" @click="handleSave" :loading="loading">
        保存
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, onMounted, computed } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Connection, Check, Close } from '@element-plus/icons-vue';
import { useSettingsStore, useUIStore } from '@/store';
import { invoke } from '@tauri-apps/api/core';
import { systemApi, apiService } from '@/api';
import ThemeSelector from '@/components/ThemeSelector.vue';

const settingsStore = useSettingsStore();
const uiStore = useUIStore();

const loading = ref(false);
const activeTab = ref('basic');  // 当前激活的标签页
const seatCountOptionsInput = ref('18, 19, 20');  // 座位数选项输入框
const resettingHttp = ref(false);  // HTTP客户端重置中
const autoSwitchTesting = ref(false);  // 自动切换手动测试中（Fork 新增）

// 解析座位数选项
function parseSeatCountOptions() {
  const input = seatCountOptionsInput.value.trim();
  if (!input) {
    settings.seat_count_options = [18, 19, 20];
    seatCountOptionsInput.value = '18, 19, 20';
    return;
  }
  
  const numbers = input.split(/[,，\s]+/)
    .map(s => parseInt(s.trim(), 10))
    .filter(n => !isNaN(n) && n > 0);
  
  if (numbers.length === 0) {
    ElMessage.warning('请输入有效的座位数');
    settings.seat_count_options = [18, 19, 20];
    seatCountOptionsInput.value = '18, 19, 20';
  } else {
    settings.seat_count_options = numbers;
    seatCountOptionsInput.value = numbers.join(', ');
  }
}

const settings = reactive<{
  auto_refresh_token: boolean;
  seat_count_options: number[];
  retry_times: number;
  theme: string;
  concurrent_limit: number;
  show_seats_result_dialog: boolean;
  autoOpenPaymentLinkInWebview: boolean;
  autoFillPaymentForm: boolean;
  autoSubmitPaymentForm: boolean;
  paymentPageDelay: number;
  showVirtualCardInfo: boolean;
  customCardBin: string;
  customCardBinRange: string;
  cardBindRetryTimes: number;
  testModeEnabled: boolean;
  useLocalSuccessBins: boolean;
  seamlessSwitchEnabled: boolean;
  windsurfClientType: 'windsurf' | 'windsurf-next';
  windsurfPath: string | null;
  patchBackupPath: string | null;
  autoOpenBrowser: boolean;
  browserMode: 'incognito' | 'normal';
  privacyMode: boolean;
  unlimitedConcurrentRefresh: boolean;
  proxyEnabled: boolean;
  proxyUrl: string | null;
  useLightweightApi: boolean;
  subscriptionPlan: number;
  paymentPeriod: number;
  startTrial: boolean;
  teamName: string;
  seatCount: number;
  cunzhiEnabled: boolean;
  autoBackupEnabled: boolean;
  backupInterval: number;
  backupMaxCount: number;
  // 额度耗尽自动切换（Fork 新增）
  autoSwitchEnabled: boolean;
  autoSwitchCheckInterval: number;
  autoSwitchThresholdPercent: number;
  autoSwitchRemainingThreshold: number;
  autoSwitchStrategy: 'most_remaining' | 'round_robin';
  autoSwitchPreferSameProvider: boolean;
}>({
  auto_refresh_token: true,
  seat_count_options: [18, 19, 20],
  retry_times: 2,
  theme: 'original-light',
  concurrent_limit: 5,
  show_seats_result_dialog: false,  // 默认关闭
  autoOpenPaymentLinkInWebview: false,  // 默认关闭自动打开支付页面
  autoFillPaymentForm: false,  // 默认关闭自动填写表单
  autoSubmitPaymentForm: false,  // 默认关闭自动提交
  paymentPageDelay: 2,  // 默认延迟2秒
  showVirtualCardInfo: false,  // 默认关闭虚拟卡信息弹窗
  customCardBin: '626202',  // 默认卡头
  customCardBinRange: '',  // 默认不使用卡段范围
  cardBindRetryTimes: 5,  // 默认绑卡重试5次
  testModeEnabled: false,  // 默认关闭测试模式
  useLocalSuccessBins: false,  // 默认不使用本地BIN池
  seamlessSwitchEnabled: false,  // 默认关闭无感换号
  windsurfClientType: 'windsurf',  // 默认 Windsurf 客户端
  windsurfPath: null,  // Windsurf路径
  patchBackupPath: null,  // 补丁备份路径
  autoOpenBrowser: true,  // 默认自动打开浏览器
  browserMode: 'incognito',  // 默认无痕模式
  privacyMode: false,  // 默认关闭隐私模式
  unlimitedConcurrentRefresh: false,  // 默认关闭全量并发刷新
  proxyEnabled: false,  // 默认关闭代理
  proxyUrl: null,  // 默认无代理地址
  useLightweightApi: true,  // 默认使用轻量级API
  subscriptionPlan: 2,  // 默认 Pro 计划
  paymentPeriod: 1,  // 默认月付
  startTrial: true,  // 默认开启试用
  teamName: '',  // 默认空团队名称
  seatCount: 1,  // 默认1个席位
  cunzhiEnabled: false,  // 默认关闭伟哥功能
  autoBackupEnabled: true,  // 默认启用自动备份
  backupInterval: 10,  // 默认10分钟
  backupMaxCount: 10,  // 默认最多10份
  // 额度耗尽自动切换默认值
  autoSwitchEnabled: false,
  autoSwitchCheckInterval: 5,
  autoSwitchThresholdPercent: 95,
  autoSwitchRemainingThreshold: 0,
  autoSwitchStrategy: 'most_remaining',
  autoSwitchPreferSameProvider: true,
});

// 备份相关
const backupLoading = ref(false);
const loadingBackups = ref(false);
const showBackupsDialog = ref(false);
const backupList = ref<Array<{ name: string; path: string; size: number }>>([]);

interface BackupInfo {
  name: string;
  path: string;
  size: number;
}

async function handleManualBackup() {
  backupLoading.value = true;
  try {
    const result = await invoke<{ success: boolean; path: string; message: string }>('create_backup');
    if (result.success) {
      ElMessage.success('备份创建成功');
    }
  } catch (e: any) {
    ElMessage.error(`备份失败: ${e}`);
  } finally {
    backupLoading.value = false;
  }
}

async function handleShowBackups() {
  loadingBackups.value = true;
  showBackupsDialog.value = true;
  try {
    backupList.value = await invoke<BackupInfo[]>('list_backups');
  } catch (e: any) {
    ElMessage.error(`获取备份列表失败: ${e}`);
    backupList.value = [];
  } finally {
    loadingBackups.value = false;
  }
}

async function handleRestoreBackup(backup: BackupInfo) {
  try {
    await ElMessageBox.confirm(
      `确定要从备份 "${backup.name}" 恢复数据吗？当前数据将被覆盖（会先自动备份当前数据）。`,
      '确认恢复',
      { type: 'warning' }
    );
    
    await invoke('restore_backup', { backupPath: backup.path });
    ElMessage.success('恢复成功，请刷新页面');
    showBackupsDialog.value = false;
    await settingsStore.loadSettings();
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(`恢复失败: ${e}`);
    }
  }
}

async function handleDeleteBackup(backup: BackupInfo) {
  try {
    await ElMessageBox.confirm(
      `确定要删除备份 "${backup.name}" 吗？此操作不可恢复。`,
      '确认删除',
      { type: 'warning' }
    );
    
    await invoke('delete_backup', { backupName: backup.name });
    ElMessage.success('备份已删除');
    await handleShowBackups();
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(`删除失败: ${e}`);
    }
  }
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
}

function formatBackupTime(name: string): string {
  // 从文件名 accounts_20260109_231500.json 提取时间
  const match = name.match(/accounts_(\d{4})(\d{2})(\d{2})_(\d{2})(\d{2})(\d{2})/);
  if (match) {
    return `${match[1]}-${match[2]}-${match[3]} ${match[4]}:${match[5]}:${match[6]}`;
  }
  return name;
}

// 成功BIN池相关
const successBinCount = ref(0);
const testModeProgress = ref<string | null>(null);

async function loadSuccessBinCount() {
  try {
    const bins = await invoke<string[]>('get_success_bins');
    successBinCount.value = bins.length;
  } catch (e) {
    successBinCount.value = 0;
  }
}

async function loadTestModeProgress() {
  try {
    testModeProgress.value = await invoke<string | null>('get_test_mode_progress');
  } catch (e) {
    testModeProgress.value = null;
  }
}

async function resetTestModeProgress() {
  try {
    await ElMessageBox.confirm('确定要重置测试模式进度吗？下次将从范围起始位置开始。', '确认重置', {
      type: 'warning'
    });
    await invoke('reset_test_mode_progress');
    testModeProgress.value = null;
    ElMessage.success('进度已重置');
  } catch (e) {
    // 用户取消
  }
}

async function viewSuccessBins() {
  try {
    const bins = await invoke<string[]>('get_success_bins');
    if (bins.length === 0) {
      ElMessage.info('BIN池为空');
      return;
    }
    ElMessageBox.alert(
      `<div style="max-height: 300px; overflow-y: auto;">
        <p><b>共 ${bins.length} 个成功BIN：</b></p>
        <p style="font-family: monospace; word-break: break-all;">${bins.join(', ')}</p>
      </div>`,
      '成功BIN池',
      { dangerouslyUseHTMLString: true }
    );
  } catch (e) {
    ElMessage.error('获取BIN池失败');
  }
}

async function clearSuccessBins() {
  try {
    await ElMessageBox.confirm('确定要清空所有成功的卡BIN吗？', '确认清空', {
      type: 'warning'
    });
    await invoke('clear_success_bins');
    successBinCount.value = 0;
    ElMessage.success('BIN池已清空');
  } catch (e) {
    // 用户取消
  }
}

// 无感换号相关
const windsurfPath = ref('');
const detectingPath = ref(false);
const patchLoading = ref(false);
// 补丁状态（字段与后端 check_patch_status 返回一一对应）
// - installed: 三项子补丁全部应用 = true（由后端汇总）
// - oauthHandler / timeoutRemoved / promptBypassApplied: 三项子补丁各自是否已应用
// - currentVersion: 文件里是否含有"当前版本注入代码"的特征字符串，用于区分
//   当前工具 vs 历史/第三方工具打的补丁（见 CURRENT_VERSION_MARKER）
const patchStatus = reactive({
  installed: false,
  error: '',
  oauthHandler: false,
  timeoutRemoved: false,
  promptBypassApplied: false,
  currentVersion: false,
});

// 补丁三项子内容的结构化数据，驱动 UI checklist 渲染
// 顺序与后端 apply 分支保持一致（6.1 / 6.2 / 6.3），方便用户对照
const patchItems = computed(() => [
  { key: 'oauthHandler', label: 'OAuth 回调处理器', applied: patchStatus.oauthHandler },
  { key: 'timeoutRemoved', label: '移除 180 秒超时限制', applied: patchStatus.timeoutRemoved },
  { key: 'promptBypassApplied', label: '跳过切号确认对话框', applied: patchStatus.promptBypassApplied },
]);

// 已应用的子项数（0 ~ 3），UI 汇总文案用
const patchAppliedCount = computed(() =>
  patchItems.value.filter(item => item.applied).length
);

// 是否显示"升级补丁"按钮：文件里已经是"当前版本注入代码"，但某些子项还没应用。
// 典型场景：用户之前打过旧版工具生成的 1+2 补丁，新工具版本又加了第 3 条，需一键补齐。
const canUpgrade = computed(() =>
  patchStatus.currentVersion &&
  !patchStatus.installed &&
  patchAppliedCount.value > 0
);

// 汇总 tag：根据三项子状态 + current_version 派生四种呈现
// - error: 后端上报读取/规则错误
// - 未安装: 0/3
// - 已安装: 3/3（installed=true）
// - 可升级: 是当前版本补丁但不完整 → canUpgrade=true
// - 第三方补丁: 部分应用但无当前版本特征 → 可能是历史版本或其他工具打的
const patchSummary = computed<{ type: 'success' | 'info' | 'warning' | 'danger'; label: string }>(() => {
  if (patchStatus.error) {
    return { type: 'danger', label: patchStatus.error };
  }
  if (patchStatus.installed) {
    return { type: 'success', label: '已安装' };
  }
  if (patchAppliedCount.value === 0) {
    return { type: 'info', label: '未安装' };
  }
  if (canUpgrade.value) {
    return { type: 'warning', label: `可升级 ${patchAppliedCount.value}/3` };
  }
  return { type: 'warning', label: `第三方补丁 ${patchAppliedCount.value}/3` };
});

// 伟哥(寸止)相关
const cunzhiLoading = ref(false);
const cunzhiStatus = reactive({
  installed: false,
  error: '',
});

watch(() => uiStore.showSettingsDialog, async (show) => {
  if (show && settingsStore.settings) {
    Object.assign(settings, settingsStore.settings);
    windsurfPath.value = settings.windsurfPath || '';
    // 同步座位数选项到输入框
    if (settings.seat_count_options && settings.seat_count_options.length > 0) {
      seatCountOptionsInput.value = settings.seat_count_options.join(', ');
    }
    // 检查补丁状态
    if (windsurfPath.value) {
      await checkPatchStatus();
    }
    // 检查伟哥状态
    await checkCunzhiStatus();
    // 加载成功BIN池数量和测试模式进度
    await loadSuccessBinCount();
    await loadTestModeProgress();
  }
}, { immediate: true });

onMounted(async () => {
  // 如果已有路径，检查状态
  const storedPath = (settingsStore.settings as any)?.windsurfPath;
  if (storedPath) {
    settings.windsurfPath = storedPath;
    windsurfPath.value = storedPath;
    await checkPatchStatus();
  }
});

async function handleSave() {
  loading.value = true;
  try {
    // 确保保存路径设置
    if (windsurfPath.value) {
      settings.windsurfPath = windsurfPath.value;
    }
    await settingsStore.updateSettings(settings);
    uiStore.setTheme(settings.theme);
    ElMessage.success('设置保存成功');
    handleClose();
  } catch (error) {
    ElMessage.error(`保存失败: ${error}`);
  } finally {
    loading.value = false;
  }
}

function handleClose() {
  uiStore.showSettingsDialog = false;
}

// 验证卡头输入
function validateCardBin(value: string) {
  // 只允许数字
  const cleaned = value.replace(/[^\d]/g, '');
  settings.customCardBin = cleaned;
  
  // 检查长度
  if (cleaned.length > 0 && cleaned.length < 4) {
    ElMessage.warning('卡头必须是4-12位数字');
  }
}

// 恢复默认卡头
function resetCardBin() {
  settings.customCardBin = '626202';
  ElMessage.success('已恢复默认卡头');
}

// 验证卡段范围格式
function validateCardBinRange(value: string) {
  // 只允许数字和连字符
  const cleaned = value.replace(/[^\d-]/g, '');
  settings.customCardBinRange = cleaned;
  
  // 如果输入了内容，验证格式
  if (cleaned && cleaned.includes('-')) {
    const parts = cleaned.split('-');
    if (parts.length === 2) {
      const [start, end] = parts;
      // 验证两端长度相同且都是数字
      if (start && end && start.length === end.length) {
        const startNum = parseInt(start, 10);
        const endNum = parseInt(end, 10);
        if (startNum > endNum) {
          ElMessage.warning('起始BIN必须小于或等于结束BIN');
        }
      } else if (start && end && start.length !== end.length) {
        ElMessage.warning('起始和结束BIN的长度必须相同');
      }
    }
  }
}

// 清除卡段范围
function clearCardBinRange() {
  settings.customCardBinRange = '';
  ElMessage.success('已清除卡段范围');
}

// 切换客户端类型时清空路径并重新检测
async function handleClientTypeChange() {
  windsurfPath.value = '';
  settings.windsurfPath = null;
  settings.seamlessSwitchEnabled = false;
  patchStatus.installed = false;
  patchStatus.error = '';
  patchStatus.oauthHandler = false;
  patchStatus.timeoutRemoved = false;
  patchStatus.promptBypassApplied = false;
  patchStatus.currentVersion = false;
  await settingsStore.updateSettings(settings);
  // 自动检测新客户端路径
  await detectWindsurfPath();
}

// 检测Windsurf路径
async function detectWindsurfPath() {
  detectingPath.value = true;
  const clientLabel = settings.windsurfClientType === 'windsurf-next' ? 'Windsurf - Next' : 'Windsurf';
  try {
    const path = await invoke<string>('get_windsurf_path', {
      clientType: settings.windsurfClientType
    });
    windsurfPath.value = path;
    settings.windsurfPath = path;
    ElMessage.success(`已找到 ${clientLabel} 安装路径`);
    // 检查补丁状态
    await checkPatchStatus();
    // 保存路径设置到本地
    await settingsStore.updateSettings(settings);
  } catch (error) {
    ElMessage.error(`检测失败: ${error}`);
    windsurfPath.value = '';
  } finally {
    detectingPath.value = false;
  }
}

// 检查补丁状态
async function checkPatchStatus() {
  if (!windsurfPath.value) return;
  
  try {
    const status = await invoke<any>('check_patch_status', {
      windsurfPath: windsurfPath.value
    });
    patchStatus.installed = status.installed;
    patchStatus.error = status.error || '';
    patchStatus.oauthHandler = !!status.oauth_handler;
    patchStatus.timeoutRemoved = !!status.timeout_removed;
    patchStatus.promptBypassApplied = !!status.prompt_bypass_applied;
    patchStatus.currentVersion = !!status.current_version;
    
    // 同步开关状态与实际补丁状态
    if (status.installed !== settings.seamlessSwitchEnabled) {
      settings.seamlessSwitchEnabled = status.installed;
      // 保存同步后的状态
      await settingsStore.updateSettings(settings);
    }
  } catch (error) {
    patchStatus.installed = false;
    patchStatus.oauthHandler = false;
    patchStatus.timeoutRemoved = false;
    patchStatus.promptBypassApplied = false;
    patchStatus.currentVersion = false;
    patchStatus.error = error as string;
  }
}

// 处理路径变化
function handlePathChange() {
  if (windsurfPath.value) {
    settings.windsurfPath = windsurfPath.value;
    // 检查新路径的补丁状态
    checkPatchStatus();
  }
}

// 浏览选择路径
async function browseWindsurfPath() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择Windsurf安装目录'
    });
    
    if (selected && typeof selected === 'string') {
      // 验证选择的路径是否包含extension.js文件
      const isValid = await invoke<boolean>('validate_windsurf_path', {
        path: selected
      });
      
      if (isValid) {
        windsurfPath.value = selected;
        settings.windsurfPath = selected;
        ElMessage.success('已选择Windsurf路径');
        await checkPatchStatus();
        // 保存路径设置到本地
        await settingsStore.updateSettings(settings);
      } else {
        ElMessage.error('所选目录不是有效的Windsurf安装目录');
      }
    }
  } catch (error) {
    ElMessage.error(`选择路径失败: ${error}`);
  }
}

// 处理无感换号开关
async function handleSeamlessSwitch(value: boolean) {
  if (!windsurfPath.value) {
    ElMessage.error('请先检测或设置客户端路径');
    settings.seamlessSwitchEnabled = !value;
    return;
  }
  
  // 确认对话框
  const action = value ? '开启' : '关闭';
  const clientLabel = settings.windsurfClientType === 'windsurf-next' ? 'Windsurf - Next' : 'Windsurf';
  const message = value 
    ? `开启无感换号将修改 ${clientLabel} 的 extension.js 文件，若客户端正在运行则自动重启，是否继续？`
    : `关闭无感换号将还原原始文件，若客户端正在运行则自动重启，是否继续？`;
  
  try {
    await ElMessageBox.confirm(
      message,
      `${action}无感换号`,
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );
  } catch {
    // 用户取消，恢复开关状态
    settings.seamlessSwitchEnabled = !value;
    return;
  }
  
  patchLoading.value = true;
  try {
    let result;
    if (value) {
      // 应用补丁
      result = await invoke<any>('apply_seamless_patch', {
        windsurfPath: windsurfPath.value
      });
    } else {
      // 还原补丁
      result = await invoke<any>('restore_seamless_patch');
    }
    
    if (result.success) {
      ElMessage.success(result.message || `无感换号已${action}`);
      if (result.already_patched) {
        ElMessage.info('补丁已经应用过了');
      }
      // 更新状态
      await checkPatchStatus();
      // 保存设置到本地
      settings.windsurfPath = windsurfPath.value;
      settings.patchBackupPath = result.backup_file || settings.patchBackupPath;
      // 立即保存到本地文件
      await settingsStore.updateSettings(settings);
    } else {
      ElMessage.error(result.message || `${action}失败`);
      settings.seamlessSwitchEnabled = !value;
    }
  } catch (error) {
    ElMessage.error(`${action}失败: ${error}`);
    settings.seamlessSwitchEnabled = !value;
  } finally {
    patchLoading.value = false;
  }
}

// 升级补丁（仅在 canUpgrade=true 时显示按钮）
// 本质就是再跑一次 apply：后端 dry-run 发现三条 pattern 还有尚未替换的，
// 会走 apply 分支对剩余子项做增量替换；已经改写过的 pattern 会自动跳过，
// 因此不会重复打已完成部分，也不会产生无效备份（原始结构已不匹配的分支 no-op）。
async function handleUpgradePatch() {
  if (!windsurfPath.value) return;
  patchLoading.value = true;
  try {
    const result = await invoke<any>('apply_seamless_patch', {
      windsurfPath: windsurfPath.value
    });
    if (result.success) {
      const mods: string[] = result.modifications || [];
      if (mods.length > 0) {
        ElMessage.success(`补丁已升级：${mods.join('、')}`);
      } else {
        ElMessage.info(result.message || '补丁已是最新');
      }
      await checkPatchStatus();
      settings.windsurfPath = windsurfPath.value;
      settings.patchBackupPath = result.backup_file || settings.patchBackupPath;
      await settingsStore.updateSettings(settings);
    } else {
      ElMessage.error(result.message || '升级失败');
    }
  } catch (error) {
    ElMessage.error(`升级失败: ${error}`);
  } finally {
    patchLoading.value = false;
  }
}

// 重置HTTP客户端
async function handleResetHttpClient() {
  resettingHttp.value = true;
  try {
    const result = await systemApi.resetHttpClient();
    if (result.success) {
      ElMessage.success(result.message || 'HTTP客户端已重置');
    } else {
      ElMessage.error('重置失败');
    }
  } catch (error) {
    ElMessage.error(`重置失败: ${error}`);
  } finally {
    resettingHttp.value = false;
  }
}

// 手动触发一次自动切换检查（Fork 新增，用于"立即检查并切换"按钮）
async function handleAutoSwitchTest() {
  // 用户可能改了设置但还没保存，先把当前 reactive 状态写入 store
  // 避免后端读到的还是旧设置
  if (settings.windsurfPath !== windsurfPath.value && windsurfPath.value) {
    settings.windsurfPath = windsurfPath.value;
  }
  await settingsStore.updateSettings(settings);

  autoSwitchTesting.value = true;
  try {
    const result = await apiService.checkAndAutoSwitch();
    if (result.triggered) {
      ElMessage.success(
        `已自动切换：${result.current_email} → ${result.switched_to_email}` +
          (result.switched_to_remaining != null
            ? `（新账号剩余 ${result.switched_to_remaining}）`
            : '')
      );
    } else if (result.current_email) {
      ElMessage.info(
        `当前账号 ${result.current_email}：使用率 ${result.current_usage_percent ?? '?'}%，剩余 ${result.current_remaining ?? '?'}。${result.reason ?? ''}`
      );
    } else {
      ElMessage.warning(result.reason || '未触发切换');
    }
  } catch (error) {
    ElMessage.error(`检查失败: ${error}`);
  } finally {
    autoSwitchTesting.value = false;
  }
}

// 检查伟哥(寸止)状态
async function checkCunzhiStatus() {
  try {
    const status = await invoke<any>('check_cunzhi_status');
    cunzhiStatus.installed = status.installed;
    cunzhiStatus.error = status.error || '';
    
    // 同步开关状态与实际状态
    if (status.installed !== settings.cunzhiEnabled) {
      settings.cunzhiEnabled = status.installed;
      await settingsStore.updateSettings(settings);
    }
  } catch (error) {
    cunzhiStatus.installed = false;
    cunzhiStatus.error = error as string;
  }
}

// 处理伟哥开关
async function handleCunzhiSwitch(value: boolean) {
  const action = value ? '开启' : '关闭';
  const message = value 
    ? '开启伟哥功能将安装 MCP 服务器和全局规则，是否继续？'
    : '关闭伟哥功能将删除 MCP 配置和全局规则，是否继续？';
  
  try {
    await ElMessageBox.confirm(
      message,
      `${action}伟哥功能`,
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );
  } catch {
    // 用户取消，恢复开关状态
    settings.cunzhiEnabled = !value;
    return;
  }
  
  cunzhiLoading.value = true;
  try {
    let result;
    if (value) {
      // 安装伟哥
      result = await invoke<any>('install_cunzhi', { windsurfPath: settings.windsurfPath || null });
    } else {
      // 卸载伟哥
      result = await invoke<any>('uninstall_cunzhi', { windsurfPath: settings.windsurfPath || null });
    }
    
    if (result.success) {
      ElMessage.success(result.message || `伟哥功能已${action}`);
      // 更新状态
      await checkCunzhiStatus();
      // 保存设置
      await settingsStore.updateSettings(settings);
      // 提示重启
      ElMessage.warning('请重启 Windsurf 以使更改生效');
    } else {
      ElMessage.error(result.message || `${action}失败`);
      settings.cunzhiEnabled = !value;
    }
  } catch (error) {
    ElMessage.error(`${action}失败: ${error}`);
    settings.cunzhiEnabled = !value;
  } finally {
    cunzhiLoading.value = false;
  }
}

// simple 版本已禁用的功能
void parseSeatCountOptions;
</script>

<style scoped>
/* 深色模式下的描述文字 */
:root.dark .el-form-item > div[style*="color: #909399"] {
  color: #94a3b8 !important;
}

/* 深色模式下的标签页内容 */
:root.dark .el-tabs__content {
  background-color: transparent;
}

/* 深色模式下的表单项标签 */
:root.dark .el-form-item__label {
  color: #cfd3dc;
}

/* 深色模式下的alert */
:root.dark .el-alert--warning {
  background-color: rgba(230, 162, 60, 0.1);
  border-color: rgba(230, 162, 60, 0.3);
}

:root.dark .el-alert--warning .el-alert__description {
  color: #cfd3dc;
}

/* ==================== 补丁状态区块 ==================== */
.patch-status-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}

.patch-status-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.patch-checklist {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 12px;
  background-color: rgba(0, 0, 0, 0.02);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 4px;
}

.patch-checklist-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  line-height: 1.6;
  color: #909399;
}

.patch-checklist-item.is-applied {
  color: #303133;
}

.patch-checklist-icon {
  font-size: 14px;
  color: #C0C4CC;
}

.patch-checklist-icon.is-applied {
  color: #67C23A;
}

/* 深色模式适配 */
:root.dark .patch-checklist {
  background-color: rgba(255, 255, 255, 0.03);
  border-color: rgba(255, 255, 255, 0.08);
}

:root.dark .patch-checklist-item {
  color: #7a8394;
}

:root.dark .patch-checklist-item.is-applied {
  color: #cfd3dc;
}
</style>
