import { defineStore } from 'pinia';
import { ref } from 'vue';
import { applyTheme, DEFAULT_THEME_ID, normalizeThemeId, type ThemeId } from '@/theme';

const LEGACY_DEFAULT_THEME_ID = 'aurora';
const THEME_DEFAULT_MIGRATION_KEY = 'theme-default-migrated-to-original-light';

function getInitialTheme(): ThemeId {
  const storedTheme = localStorage.getItem('theme');
  const migrationCompleted = localStorage.getItem(THEME_DEFAULT_MIGRATION_KEY) === 'true';

  localStorage.setItem(THEME_DEFAULT_MIGRATION_KEY, 'true');

  if (storedTheme === LEGACY_DEFAULT_THEME_ID && !migrationCompleted) {
    return DEFAULT_THEME_ID;
  }

  return normalizeThemeId(storedTheme);
}

export const useUIStore = defineStore('ui', () => {
  const sidebarCollapsed = ref(true);  // 默认收缩
  const savedTheme = getInitialTheme();
  const theme = ref<ThemeId>(savedTheme);
  
  applyTheme(savedTheme);
  const showAddAccountDialog = ref(false);
  const showEditAccountDialog = ref(false);
  const showSettingsDialog = ref(false);
  // 打开设置对话框时希望激活的 tab 名（用于侧边栏"自动切换"等快捷入口直接定位）
  // 注意：每次 openSettingsDialog(tab) 都会重写此值；SettingsDialog 内部通过 watch 同步 activeTab。
  const settingsInitialTab = ref<string | null>(null);
  const showLogsDialog = ref(false);
  const showBatchOperationDialog = ref(false);
  const showStatsDialog = ref(false);
  const showAccountInfoDialog = ref(false);
  const showBillingDialog = ref(false);
  const currentEditingAccountId = ref<string | null>(null);
  const currentViewingAccountId = ref<string | null>(null);
  
  // 通知相关
  const notifications = ref<Array<{
    id: string;
    type: 'success' | 'warning' | 'error' | 'info';
    title: string;
    message?: string;
    duration?: number;
  }>>([]);

  // Actions
  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  }

  async function setTheme(newTheme: string) {
    const normalizedTheme = applyTheme(newTheme);
    theme.value = normalizedTheme;
    
    // 保存主题设置到后端
    try {
      const { settingsApi } = await import('@/api');
      // 获取当前设置并更新主题
      const currentSettings = await settingsApi.getSettings();
      await settingsApi.updateSettings({ ...currentSettings, theme: normalizedTheme });
    } catch (error) {
      console.error('Failed to save theme setting:', error);
    }
  }

  function showNotification(notification: Omit<typeof notifications.value[0], 'id'>) {
    const id = Date.now().toString();
    notifications.value.push({ ...notification, id });
    
    if (notification.duration !== 0) {
      setTimeout(() => {
        removeNotification(id);
      }, notification.duration || 3000);
    }
  }

  function removeNotification(id: string) {
    const index = notifications.value.findIndex(n => n.id === id);
    if (index > -1) {
      notifications.value.splice(index, 1);
    }
  }

  function openAddAccountDialog() {
    showAddAccountDialog.value = true;
  }

  function closeAddAccountDialog() {
    showAddAccountDialog.value = false;
  }

  function openEditAccountDialog(accountId: string) {
    currentEditingAccountId.value = accountId;
    showEditAccountDialog.value = true;
  }

  function closeEditAccountDialog() {
    showEditAccountDialog.value = false;
    currentEditingAccountId.value = null;
  }

  /**
   * 打开设置对话框
   * @param initialTab 可选，希望直接激活的 tab name（如 `'auto-switch'`、`'seamless'`），
   *                   SettingsDialog 内部 watch 此值并切换 `activeTab`
   */
  function openSettingsDialog(initialTab?: string) {
    if (initialTab) {
      settingsInitialTab.value = initialTab;
    }
    showSettingsDialog.value = true;
  }

  function closeSettingsDialog() {
    showSettingsDialog.value = false;
  }

  function openLogsDialog() {
    showLogsDialog.value = true;
  }

  function closeLogsDialog() {
    showLogsDialog.value = false;
  }

  function openBatchOperationDialog() {
    showBatchOperationDialog.value = true;
  }

  function closeBatchOperationDialog() {
    showBatchOperationDialog.value = false;
  }

  function openStatsDialog() {
    showStatsDialog.value = true;
  }

  function closeStatsDialog() {
    showStatsDialog.value = false;
  }

  function openAccountInfoDialog(accountId: string) {
    currentViewingAccountId.value = accountId;
    showAccountInfoDialog.value = true;
  }

  function closeAccountInfoDialog() {
    showAccountInfoDialog.value = false;
    currentViewingAccountId.value = null;
  }

  function openBillingDialog(accountId: string) {
    currentViewingAccountId.value = accountId;
    showBillingDialog.value = true;
  }

  function closeBillingDialog() {
    showBillingDialog.value = false;
    currentViewingAccountId.value = null;
  }

  return {
    // State
    sidebarCollapsed,
    theme,
    showAddAccountDialog,
    showEditAccountDialog,
    showSettingsDialog,
    settingsInitialTab,
    showLogsDialog,
    showBatchOperationDialog,
    showStatsDialog,
    showAccountInfoDialog,
    showBillingDialog,
    currentEditingAccountId,
    currentViewingAccountId,
    notifications,

    // Actions
    toggleSidebar,
    setTheme,
    openAddAccountDialog,
    closeAddAccountDialog,
    openEditAccountDialog,
    closeEditAccountDialog,
    openSettingsDialog,
    closeSettingsDialog,
    openLogsDialog,
    closeLogsDialog,
    openBatchOperationDialog,
    closeBatchOperationDialog,
    openStatsDialog,
    closeStatsDialog,
    openAccountInfoDialog,
    closeAccountInfoDialog,
    openBillingDialog,
    closeBillingDialog,
    showNotification,
    removeNotification,
  };
});
