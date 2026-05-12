<template>
  <el-dialog
    v-model="visible"
    title=""
    width="480px"
    class="welcome-dialog"
    :close-on-click-modal="false"
    @closed="handleClosed"
  >
    <div class="welcome-content">
      <!-- 背景装饰 -->
      <div class="bg-decoration">
        <div class="circle circle-1"></div>
        <div class="circle circle-2"></div>
      </div>
      
      <!-- 警告图标 -->
      <div class="warning-section">
        <div class="warning-icon">
          <el-icon :size="48"><WarningFilled /></el-icon>
        </div>
        <h2 class="warning-title">重要提醒</h2>
      </div>

      <!-- 免费声明 -->
      <div class="free-notice">
        <div class="notice-card">
          <div class="notice-icon">
            <el-icon :size="24"><InfoFilled /></el-icon>
          </div>
          <div class="notice-content">
            <p class="notice-main">本软件为 <span class="highlight">完全免费</span> 的开源软件</p>
            <p class="notice-sub">如果你是花钱购买的，那么 <span class="warning-text">你被骗了！</span></p>
          </div>
        </div>
      </div>

      <!-- 提示列表 -->
      <div class="tips-section">
        <div class="tip-item">
          <el-icon class="tip-icon"><CircleCheck /></el-icon>
          <span>本软件仅供学习交流使用</span>
        </div>
        <div class="tip-item">
          <el-icon class="tip-icon"><CircleCheck /></el-icon>
          <span>请勿用于任何商业用途</span>
        </div>
        <div class="tip-item">
          <el-icon class="tip-icon"><CircleCheck /></el-icon>
          <span>禁止倒卖，发现必究</span>
        </div>
      </div>

      <!--
        fork v1.7.11：原上游有微信群二维码欢迎弹窗，本 fork 作者不运营任何社群，
        改为引导到 GitHub Issues，避免新用户扫码加入 chaogei 官方群后就 fork 问题
        咨询官方维护者，造成互相困扰。如需恢复原版，从 git history 取 v1.7.10 的版本。
      -->
      <div class="group-section">
        <h3 class="section-title">
          <el-icon><ChatDotRound /></el-icon>
          <span>反馈与讨论</span>
        </h3>
        <div class="feedback-info">
          <p class="feedback-line">
            🐛 发现 Bug / 建议：<a href="https://github.com/Apyrenia/windsurf-account-manager-simple/issues" target="_blank">GitHub Issues</a>
          </p>
          <p class="feedback-line">
            💡 使用讨论：<a href="https://github.com/Apyrenia/windsurf-account-manager-simple/discussions" target="_blank">GitHub Discussions</a>
          </p>
          <p class="feedback-tip">本 fork 作者不运营任何微信/QQ 群，请勿扫码加群咨询。</p>
        </div>
      </div>

      <!-- 确认按钮 -->
      <div class="action-section">
        <el-button type="primary" size="large" @click="handleConfirm" class="confirm-btn">
          <el-icon><Select /></el-icon>
          <span>我已知晓</span>
        </el-button>
      </div>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { 
  WarningFilled,
  InfoFilled,
  CircleCheck,
  ChatDotRound,
  Select
} from '@element-plus/icons-vue';

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
});

function handleConfirm() {
  emit('update:modelValue', false);
}

function handleClosed() {
  emit('update:modelValue', false);
}
</script>

<style scoped>
.welcome-dialog :deep(.el-dialog__header) {
  display: none;
}

.welcome-dialog :deep(.el-dialog__body) {
  padding: 0;
}

.welcome-content {
  position: relative;
  padding: 30px;
  background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
  overflow: hidden;
  border-radius: 8px;
}

/* 背景装饰 */
.bg-decoration {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  pointer-events: none;
  overflow: hidden;
}

.circle {
  position: absolute;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.1);
}

.circle-1 {
  width: 200px;
  height: 200px;
  top: -100px;
  right: -100px;
}

.circle-2 {
  width: 150px;
  height: 150px;
  bottom: -75px;
  left: -75px;
}

/* 警告区域 */
.warning-section {
  text-align: center;
  margin-bottom: 20px;
  position: relative;
}

.warning-icon {
  width: 80px;
  height: 80px;
  margin: 0 auto 15px;
  background: rgba(255, 255, 255, 0.2);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { transform: scale(1); box-shadow: 0 0 0 0 rgba(255, 255, 255, 0.4); }
  50% { transform: scale(1.05); box-shadow: 0 0 0 15px rgba(255, 255, 255, 0); }
}

.warning-title {
  font-size: 24px;
  font-weight: 700;
  color: white;
  margin: 0;
  text-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

/* 免费声明 */
.free-notice {
  margin-bottom: 20px;
}

.notice-card {
  background: rgba(255, 255, 255, 0.95);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  align-items: flex-start;
  gap: 15px;
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
}

.notice-icon {
  width: 44px;
  height: 44px;
  min-width: 44px;
  background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.notice-content {
  flex: 1;
}

.notice-main {
  font-size: 16px;
  color: #333;
  margin: 0 0 8px 0;
  font-weight: 500;
}

.notice-sub {
  font-size: 14px;
  color: #666;
  margin: 0;
}

.highlight {
  color: #67c23a;
  font-weight: 700;
  font-size: 18px;
}

.warning-text {
  color: #f56c6c;
  font-weight: 700;
}

/* 提示列表 */
.tips-section {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 10px;
  padding: 15px 20px;
  margin-bottom: 20px;
}

.tip-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  color: white;
  font-size: 14px;
}

.tip-item:not(:last-child) {
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.tip-icon {
  color: #a5d6a7;
  font-size: 18px;
}

/* 交流群 */
.group-section {
  background: rgba(255, 255, 255, 0.95);
  border-radius: 12px;
  padding: 20px;
  margin-bottom: 20px;
  text-align: center;
}

.section-title {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  color: #333;
  margin: 0 0 15px 0;
}

.section-title .el-icon {
  color: #07c160;
}

/* fork v1.7.11：新 feedback section 样式（替代原二维码） */
.feedback-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 4px;
}

.feedback-line {
  margin: 0;
  font-size: 14px;
  color: #374151;
  line-height: 1.6;
}

.feedback-line a {
  color: #2563eb;
  text-decoration: none;
  font-weight: 500;
}

.feedback-line a:hover {
  text-decoration: underline;
}

.feedback-tip {
  margin: 4px 0 0;
  font-size: 12px;
  color: #6b7280;
  font-style: italic;
}

.qr-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

.qr-image {
  width: 180px;
  height: 180px;
  border-radius: 8px;
  border: 2px solid #07c160;
  padding: 5px;
  background: white;
}

.qr-tip {
  font-size: 12px;
  color: #999;
  margin: 0;
}

/* 确认按钮 */
.action-section {
  text-align: center;
}

.confirm-btn {
  width: 100%;
  height: 46px;
  font-size: 16px;
  font-weight: 600;
  border-radius: 10px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border: none;
  box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4);
  transition: all 0.3s ease;
}

.confirm-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(102, 126, 234, 0.5);
}

.confirm-btn .el-icon {
  margin-right: 8px;
}

/* 暗色主题适配 */
:root.dark .welcome-content {
  background: linear-gradient(135deg, #c0392b 0%, #8e44ad 100%);
}

:root.dark .notice-card,
:root.dark .group-section {
  background: rgba(30, 30, 46, 0.95);
}

:root.dark .notice-main,
:root.dark .section-title {
  color: #e4e4e7;
}

:root.dark .notice-sub {
  color: #a0a0a0;
}

:root.dark .qr-tip {
  color: #888;
}

/* fork v1.7.11：feedback section 深色主题 */
:root.dark .feedback-line {
  color: #d1d5db;
}
:root.dark .feedback-line a {
  color: #60a5fa;
}
:root.dark .feedback-tip {
  color: #9ca3af;
}
</style>
