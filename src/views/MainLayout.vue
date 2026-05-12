<template>
  <el-container class="main-container">
    <!-- 侧边栏 -->
    <el-aside :width="sidebarWidth" class="sidebar" :class="{ 'is-collapsed': uiStore.sidebarCollapsed }" :style="{ overflow: 'hidden' }">
      <div class="app-title">
        <el-icon size="24"><Connection /></el-icon>
        <div class="app-title-text" :aria-hidden="uiStore.sidebarCollapsed">
          <span>Windsurf Manager</span>
          <span class="version-text">v{{ appVersion }}</span>
        </div>
      </div>
      
      <el-menu
        :collapse="menuCollapsed"
        :default-active="activeMenu"
        :default-openeds="[]"
        class="sidebar-menu"
        :collapse-transition="false"
      >
        <el-menu-item index="accounts" @click="setActiveMenu('accounts')">
          <el-icon><User /></el-icon>
          <template #title>账号管理</template>
        </el-menu-item>
        
        <el-sub-menu
          index="groups"
          class="groups-submenu"
          popper-class="groups-submenu-popper"
        >
          <template #title>
            <el-icon><Folder /></el-icon>
            <span>分组管理</span>
          </template>
          <el-menu-item 
            v-for="group in settingsStore.groups" 
            :key="group"
            :index="`group-${group}`"
            class="group-item"
          >
            <div class="group-item-content">
              <span @click="filterByGroup(group)" class="group-name">{{ group }} <span class="group-count">({{ getGroupAccountCount(group) }})</span></span>
              <div class="group-actions" v-if="group !== '默认分组'">
                <el-icon @click.stop="showRenameGroupDialog(group)" class="group-action-icon">
                  <Edit />
                </el-icon>
                <el-icon @click.stop="showDeleteGroupConfirm(group)" class="group-action-icon delete">
                  <Delete />
                </el-icon>
              </div>
            </div>
          </el-menu-item>
          <el-menu-item index="add-group" class="group-add-action" @click="showAddGroupDialog">
            <el-icon><Plus /></el-icon>
            添加分组
          </el-menu-item>
        </el-sub-menu>
        
        <el-menu-item index="logs" @click="uiStore.openLogsDialog">
          <el-icon><Document /></el-icon>
          <template #title>操作日志</template>
        </el-menu-item>
        
        <el-menu-item index="stats" @click="uiStore.openStatsDialog">
          <el-icon><DataAnalysis /></el-icon>
          <template #title>统计信息</template>
        </el-menu-item>
        
        <!--
          ⚠️ 「自动重置」入口已隐藏（fork 个人版决定）
          原因：本 fork 维护者认为该功能在新版 Windsurf 配额体系下意义有限，
                而且其调用的 `update_seat_count` 接口被频繁调用容易触发 rate limit。
          代码保留：`AutoResetDialog.vue` / `auto_reset_commands.rs` / 数据库字段 / 设置项 etc.
                未删除以保持向上游兼容；如需重新启用，恢复下面 el-menu-item 即可。
        <el-menu-item index="auto-reset" @click="showAutoResetDialog = true">
          <el-icon><Timer /></el-icon>
          <template #title>自动重置</template>
        </el-menu-item>
        -->

        <el-menu-item index="auto-switch" @click="uiStore.openSettingsDialog('auto-switch')">
          <el-icon><Switch /></el-icon>
          <template #title>
            自动切换
            <el-badge
              v-if="settingsStore.settings?.autoSwitchEnabled"
              value="ON"
              type="success"
              class="auto-switch-badge"
            />
          </template>
        </el-menu-item>

        <el-menu-item index="card-generator" @click="showCardGeneratorDialog = true">
          <el-icon><CreditCard /></el-icon>
          <template #title>虚拟卡生成</template>
        </el-menu-item>
        
        <el-menu-item index="about" @click="showAboutDialog">
          <el-icon><InfoFilled /></el-icon>
          <template #title>关于</template>
        </el-menu-item>
        
        <el-menu-item index="settings" @click="uiStore.openSettingsDialog">
          <el-icon><Setting /></el-icon>
          <template #title>设置</template>
        </el-menu-item>
      </el-menu>
      
      <div class="sidebar-footer">
        <el-button 
          :icon="uiStore.sidebarCollapsed ? ArrowRight : ArrowLeft"
          circle
          @click="toggleSidebar"
        />
      </div>
    </el-aside>

    <!-- 主内容区 -->
    <el-container>
      <!-- 顶部操作栏 -->
      <el-header class="header" :class="{ 'has-selected-actions': accountsStore.selectedAccounts.size > 0 }">
        <div class="header-left">
          <el-input
            v-model="searchQuery"
            placeholder="搜索账号..."
            :prefix-icon="Search"
            clearable
            class="search-input"
            @input="handleSearch"
          />
          <el-tooltip content="高级筛选" placement="bottom">
            <el-button
              :icon="Filter"
              circle
              :type="hasActiveFilter ? 'primary' : 'default'"
              @click="showFilterPanel = !showFilterPanel"
              class="filter-toggle-btn"
            />
          </el-tooltip>
          
          <!-- 排序选择器 -->
          <el-select
            v-model="currentSortField"
            placeholder="排序方式"
            size="default"
            class="sort-select"
            @change="handleSortChange"
          >
            <el-option label="邮箱名称" value="email" />
            <el-option label="创建时间" value="created_at" />
            <el-option label="已用积分" value="used_quota" />
            <el-option label="剩余积分" value="remaining_quota" />
            <el-option label="Token过期" value="token_expires_at" />
            <el-option label="订阅到期" value="subscription_expires_at" />
            <el-option label="套餐类型" value="plan_name" />
            <el-option label="日配额剩余%" value="daily_quota_remaining" />
            <el-option label="周配额剩余%" value="weekly_quota_remaining" />
          </el-select>
          <el-tooltip :content="sortDirection === 'asc' ? '升序' : '降序'" placement="bottom">
            <el-button
              :icon="sortDirection === 'asc' ? SortUp : SortDown"
              circle
              @click="toggleSortDirection"
            />
          </el-tooltip>
        </div>
        
        <div class="header-right">
          <el-tooltip content="主题配色" placement="bottom">
            <ThemeSelector v-model="uiStore.theme" @change="uiStore.setTheme" />
          </el-tooltip>

          <!-- 批量删除 -->
          <el-tooltip content="批量删除" placement="bottom" v-if="accountsStore.selectedAccounts.size > 0">
            <el-badge :value="accountsStore.selectedAccounts.size" :offset="[12, -8]">
              <el-button
                type="danger"
                :icon="Delete"
                circle
                @click="handleBatchDelete"
              />
            </el-badge>
          </el-tooltip>
          
          <el-tooltip content="批量转让订阅" placement="bottom" v-if="accountsStore.selectedAccounts.size > 0">
            <el-button
              type="success"
              :icon="Switch"
              circle
              @click="showBatchTransferDialog = true"
            />
          </el-tooltip>
          
          <!-- 批量刷新状态 -->
          <el-tooltip content="批量刷新状态" placement="bottom" v-if="accountsStore.selectedAccounts.size > 0">
            <el-button
              type="warning"
              :icon="RefreshRight"
              circle
              @click="handleBatchRefresh"
            />
          </el-tooltip>
          
          <el-tooltip content="批量更换订阅" placement="bottom" v-if="accountsStore.selectedAccounts.size > 0">
            <el-button
              type="primary"
              :icon="Trophy"
              circle
              @click="openBatchUpdatePlanDialog"
            />
          </el-tooltip>
          
          <!-- 导出选中账号 -->
          <el-tooltip content="导出选中账号" placement="bottom" v-if="accountsStore.selectedAccounts.size > 0">
            <el-button
              type="info"
              :icon="Download"
              circle
              @click="handleExportAccounts(true)"
            />
          </el-tooltip>
          
          <!-- 批量更改分组 -->
          <el-tooltip content="批量更改分组" placement="bottom" v-if="accountsStore.selectedAccounts.size > 0">
            <el-button
              type="primary"
              :icon="FolderOpened"
              circle
              @click="showBatchGroupDialog = true"
            />
          </el-tooltip>
          
          <!-- 取消已选 -->
          <el-tooltip content="取消已选" placement="bottom" v-if="accountsStore.selectedAccounts.size > 0">
            <el-button
              :icon="Close"
              circle
              style="background-color: #909399; border-color: #909399; color: white;"
              @click="accountsStore.clearSelection()"
            />
          </el-tooltip>
          
          <!-- 选择本页账号 -->
          <el-tooltip content="选择本页账号" placement="bottom">
            <el-button
              :icon="DocumentChecked"
              circle
              type="default"
              @click="selectCurrentPageAccounts"
            />
          </el-tooltip>
          
          <!-- 全选按钮（带分隔线） -->
          <el-tooltip content="全选" placement="bottom" class="select-all-button">
            <el-button
              :icon="Select"
              circle
              :type="isAllSelected ? 'primary' : 'default'"
              @click="toggleSelectAll"
            />
          </el-tooltip>
          
          <!-- 添加账号 -->
          <el-tooltip content="添加账号" placement="bottom">
            <el-button 
              type="default" 
              :icon="Plus" 
              circle 
              @click="uiStore.openAddAccountDialog" 
            />
          </el-tooltip>
          
          <!-- 批量添加 -->
          <el-tooltip content="批量导入" placement="bottom">
            <el-button 
              type="default" 
              :icon="Upload"
              circle 
              @click="handleBatchImport"
            />
          </el-tooltip>
          
          <!-- 导出账号 -->
          <el-tooltip content="导出账号" placement="bottom">
            <el-button 
              :icon="Download"
              circle 
              type="default"
              @click="handleExportAccounts"
            />
          </el-tooltip>
          
          <!-- 标签管理 -->
          <el-tooltip content="标签管理" placement="bottom">
            <el-button 
              :icon="PriceTag"
              circle 
              type="default"
              @click="showTagManageDialog = true"
            />
          </el-tooltip>
          
          <!-- 全局刷新 -->
          <el-tooltip content="刷新全部" placement="bottom">
            <el-button 
              :icon="RefreshRight" 
              circle 
              type="default"
              @click="refreshAccounts" 
            />
          </el-tooltip>
        </div>
      </el-header>

      <!-- 账号卡片区域 -->
      <el-main class="main-content">
        <!-- 筛选面板 -->
        <transition name="filter-slide">
          <div v-if="showFilterPanel" class="filter-panel">
            <div class="filter-panel-header">
              <span class="filter-title">高级筛选</span>
              <div class="filter-header-actions">
                <el-button size="small" @click="clearAllFilters">清除筛选</el-button>
                <el-button size="small" type="primary" @click="applyFilters">应用筛选</el-button>
                <el-button :icon="Close" circle size="small" @click="showFilterPanel = false" />
              </div>
            </div>
            <div class="filter-panel-body">
              <!-- 第一行：数值范围筛选 -->
              <div class="filter-row">
                <div class="filter-item filter-item-range">
                  <span class="filter-label">剩余额度</span>
                  <div class="filter-range">
                    <el-input-number v-model="filterForm.remainingQuotaMin" :min="0" :controls="false" placeholder="最小" size="small" />
                    <span class="range-separator">-</span>
                    <el-input-number v-model="filterForm.remainingQuotaMax" :min="0" :controls="false" placeholder="最大" size="small" />
                  </div>
                </div>
                <div class="filter-item filter-item-range">
                  <span class="filter-label">总额度</span>
                  <div class="filter-range">
                    <el-input-number v-model="filterForm.totalQuotaMin" :min="0" :controls="false" placeholder="最小" size="small" />
                    <span class="range-separator">-</span>
                    <el-input-number v-model="filterForm.totalQuotaMax" :min="0" :controls="false" placeholder="最大" size="small" />
                  </div>
                </div>
                <div class="filter-item filter-item-range">
                  <span class="filter-label">剩余天数</span>
                  <div class="filter-range">
                    <el-input-number v-model="filterForm.expiryDaysMin" :controls="false" placeholder="最小" size="small" />
                    <span class="range-separator">-</span>
                    <el-input-number v-model="filterForm.expiryDaysMax" :controls="false" placeholder="最大" size="small" />
                  </div>
                </div>
              </div>
              <!-- 第二行：日/周配额剩余百分比（仅 billing_strategy === 2 (QUOTA) 的账号参与） -->
              <div class="filter-row">
                <div class="filter-item filter-item-range">
                  <span class="filter-label">日配额剩余%</span>
                  <div class="filter-range">
                    <el-input-number v-model="filterForm.dailyQuotaPercentMin" :min="0" :max="100" :controls="false" placeholder="最小" size="small" />
                    <span class="range-separator">-</span>
                    <el-input-number v-model="filterForm.dailyQuotaPercentMax" :min="0" :max="100" :controls="false" placeholder="最大" size="small" />
                  </div>
                </div>
                <div class="filter-item filter-item-range">
                  <span class="filter-label">周配额剩余%</span>
                  <div class="filter-range">
                    <el-input-number v-model="filterForm.weeklyQuotaPercentMin" :min="0" :max="100" :controls="false" placeholder="最小" size="small" />
                    <span class="range-separator">-</span>
                    <el-input-number v-model="filterForm.weeklyQuotaPercentMax" :min="0" :max="100" :controls="false" placeholder="最大" size="small" />
                  </div>
                </div>
              </div>
              <!-- 第三行：选择器筛选 -->
              <div class="filter-row filter-row-select">
                <div class="filter-item filter-item-select">
                  <span class="filter-label">套餐</span>
                  <el-select v-model="filterForm.selectedPlans" multiple collapse-tags collapse-tags-tooltip placeholder="全部" size="small">
                    <el-option v-for="plan in accountsStore.allPlanNames" :key="plan" :label="plan" :value="plan" />
                  </el-select>
                </div>
                <div class="filter-item filter-item-select">
                  <span class="filter-label">标签</span>
                  <el-select v-model="filterForm.selectedTags" multiple collapse-tags collapse-tags-tooltip placeholder="全部" size="small">
                    <el-option v-for="tag in accountsStore.allTags" :key="tag" :label="tag" :value="tag" />
                  </el-select>
                </div>
                <div class="filter-item filter-item-select">
                  <span class="filter-label">域名</span>
                  <el-select v-model="filterForm.selectedDomains" multiple collapse-tags collapse-tags-tooltip placeholder="全部" size="small">
                    <el-option v-for="domain in accountsStore.allDomains" :key="domain" :label="domain" :value="domain" />
                  </el-select>
                </div>
                <div class="filter-item filter-item-select">
                  <span class="filter-label">状态</span>
                  <el-select v-model="filterForm.selectedStatuses" multiple collapse-tags collapse-tags-tooltip placeholder="全部" size="small">
                    <el-option v-for="status in statusOptions" :key="status.value" :label="status.label" :value="status.value" />
                  </el-select>
                </div>
              </div>
            </div>
          </div>
        </transition>

        <div v-if="accountsStore.loading && accountsStore.paginatedAccounts.length === 0" class="loading-container">
          <el-icon class="is-loading" size="32"><Loading /></el-icon>
        </div>
        
        <div v-else-if="!accountsStore.loading && accountsStore.filteredAccounts.length === 0" class="empty-container">
          <el-empty description="暂无账号数据">
            <el-button type="primary" @click="uiStore.openAddAccountDialog">
              添加第一个账号
            </el-button>
          </el-empty>
        </div>
        
        <div v-else class="accounts-container" :class="{ 'is-refreshing': accountsStore.loading }">
          <div v-if="accountsStore.loading" class="accounts-loading-overlay">
            <el-icon class="is-loading" size="22"><Loading /></el-icon>
            <span>正在刷新...</span>
          </div>

          <div class="accounts-grid">
            <AccountCard
              v-for="account in renderedAccounts"
              :key="account.id"
              :account="account"
              :is-selected="accountsStore.selectedAccounts.has(account.id)"
              :current-email="currentWindsurfEmail"
              @select="handleAccountSelect(account.id, $event)"
              @update="handleAccountUpdate"
            />
          </div>

          <div v-if="isRenderingAccountBatch" class="account-rendering-hint">
            正在渲染更多账号...
          </div>
          
          <!-- 分页组件 -->
          <div class="pagination-container" v-if="accountsStore.totalCount > accountsStore.pagination.pageSize">
            <el-pagination
              :current-page="accountsStore.pagination.currentPage"
              :page-size="accountsStore.pagination.pageSize"
              :page-sizes="accountsStore.pagination.pageSizes"
              :total="accountsStore.totalCount"
              layout="total, sizes, prev, pager, next, jumper"
              background
              @size-change="handlePageSizeChange"
              @current-change="handleCurrentPageChange"
            />
          </div>
        </div>
      </el-main>
    </el-container>

    <!-- 对话框组件 -->
    <AddAccountDialog v-if="uiStore.showAddAccountDialog" />
    <EditAccountDialog v-if="uiStore.showEditAccountDialog" />
    <SettingsDialog v-if="uiStore.showSettingsDialog" />
    <BatchImportDialog 
      v-if="showBatchImportDialog"
      v-model="showBatchImportDialog" 
      @import="handleBatchImportConfirm" 
      ref="batchImportDialogRef"
    />
    <BatchExportDialog
      v-if="showBatchExportDialog"
      v-model="showBatchExportDialog"
      :accounts="batchExportAccounts"
    />
    <LogsDialog v-if="uiStore.showLogsDialog" />
    <StatsDialog v-if="uiStore.showStatsDialog" />
    <AccountInfoDialog v-if="uiStore.showAccountInfoDialog" />
    
    <!-- 关于对话框 -->
    <AboutDialog 
      v-if="showAbout"
      v-model="showAbout"
      :current-email="currentWindsurfEmail"
      :windsurf-version="windsurfVersion"
      :client-display-name="windsurfClientDisplayName"
      @open-update-dialog="showUpdateDialog = true"
    />

    <!-- 自动更新对话框 -->
    <UpdateDialog v-if="showUpdateDialog" v-model="showUpdateDialog" :current-version="appVersion" />

    <AutoResetDialog v-if="showAutoResetDialog" v-model="showAutoResetDialog" />
    
    <!-- 虚拟卡生成对话框 -->
    <CardGeneratorDialog v-if="showCardGeneratorDialog" v-model="showCardGeneratorDialog" />
    
    <!-- 账单对话框（传入当前查看的账号ID和数据） -->
    <BillingDialog 
      v-if="uiStore.showBillingDialog && uiStore.currentViewingAccountId"
      v-model="uiStore.showBillingDialog"
      :account-id="uiStore.currentViewingAccountId"
      :billing-data="currentBillingData"
      :loading="billingLoading"
      @refresh="refreshBillingData"
    />
    
    <!-- 批量更换订阅对话框 -->
    <BatchUpdatePlanDialog 
      v-if="showBatchUpdatePlanDialog"
      v-model="showBatchUpdatePlanDialog"
      :selected-account-ids="Array.from(accountsStore.selectedAccounts)"
      :accounts="batchPlanAccounts"
      @success="accountsStore.loadAccounts()"
    />
    
    <!-- 标签管理对话框 -->
    <TagManageDialog 
      v-if="showTagManageDialog"
      v-model="showTagManageDialog"
      :selected-account-ids="Array.from(accountsStore.selectedAccounts)"
      @refresh="accountsStore.loadAccounts()"
    />
    
    <!-- 批量更改分组对话框 -->
    <el-dialog
      v-if="showBatchGroupDialog"
      v-model="showBatchGroupDialog"
      title="批量更改分组"
      width="400px"
      :close-on-click-modal="false"
      @close="closeBatchGroupDialog"
    >
      <div class="batch-group-content">
        <p class="batch-group-hint">
          将选中的 <strong>{{ accountsStore.selectedAccounts.size }}</strong> 个账号移动到指定分组：
        </p>
        <el-select
          v-model="batchGroupTarget"
          placeholder="选择目标分组"
          style="width: 100%;"
          size="large"
        >
          <el-option
            v-for="group in settingsStore.groups"
            :key="group"
            :label="group"
            :value="group"
          />
        </el-select>
      </div>
      <template #footer>
        <el-button @click="closeBatchGroupDialog">取消</el-button>
        <el-button
          type="primary"
          :disabled="!batchGroupTarget"
          :loading="isBatchUpdatingGroup"
          @click="handleBatchUpdateGroup"
        >
          确认更改
        </el-button>
      </template>
    </el-dialog>

    <!-- 批量转让订阅对话框 -->
    <el-dialog
      v-if="showBatchTransferDialog"
      v-model="showBatchTransferDialog"
      title="批量转让订阅"
      width="600px"
      :close-on-click-modal="false"
    >
      <el-alert
        title="批量转让说明"
        type="warning"
        :closable="false"
        show-icon
        style="margin-bottom: 20px"
      >
        <template #default>
          <p>已选中 <strong>{{ accountsStore.selectedAccounts.size }}</strong> 个源账户需要转让订阅。</p>
          <p>请在下方输入对应数量的目标邮箱（每行一个），转让后源账户将被移出团队。</p>
          <p style="color: #e6a23c; margin-top: 8px;">⚠️ 此操作不可撤销！</p>
        </template>
      </el-alert>
      
      <el-form label-width="100px">
        <el-form-item label="目标邮箱">
          <el-input
            v-model="batchTransferEmails"
            type="textarea"
            :rows="8"
            :placeholder="'请输入 ' + accountsStore.selectedAccounts.size + ' 个目标邮箱，每行一个\n例如：\nuser1@example.com\nuser2@example.com'"
            name="batch-transfer-emails-no-autofill"
            autocomplete="off"
          />
        </el-form-item>
        <el-form-item>
          <div class="email-count-hint">
            已输入: {{ parsedTransferEmails.length }} / {{ accountsStore.selectedAccounts.size }} 个邮箱
            <span v-if="parsedTransferEmails.length !== accountsStore.selectedAccounts.size" style="color: #e6a23c;">
              （数量不匹配）
            </span>
            <span v-else style="color: #67c23a;">
              （数量匹配 ✓）
            </span>
          </div>
        </el-form-item>
      </el-form>
      
      <!-- 转让进度显示 -->
      <div v-if="batchTransferring" class="batch-transfer-progress">
        <el-progress
          :percentage="Math.round((batchTransferProgress.current / batchTransferProgress.total) * 100)"
          :stroke-width="12"
        />
        <div class="progress-status">
          {{ batchTransferProgress.status }}
          ({{ batchTransferProgress.current }}/{{ batchTransferProgress.total }})
        </div>
      </div>
      
      <template #footer>
        <el-button @click="showBatchTransferDialog = false" :disabled="batchTransferring">取消</el-button>
        <el-button
          type="danger"
          :loading="batchTransferring"
          :disabled="parsedTransferEmails.length !== accountsStore.selectedAccounts.size"
          @click="handleBatchTransfer"
        >
          确认批量转让
        </el-button>
      </template>
    </el-dialog>
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  User,
  Folder,
  FolderOpened,
  Document,
  Setting,
  ArrowRight,
  ArrowLeft,
  Search,
  Plus,
  Delete,
  Edit,
  RefreshRight,
  Connection,
  Loading,
  DataAnalysis,
  InfoFilled,
  Select,
  Download,
  Upload,
  Trophy,
  Filter,
  Close,
  PriceTag,
  DocumentChecked,
  Timer,
  Switch,
  SortUp,
  SortDown
} from '@element-plus/icons-vue';
import { useAccountsStore, useSettingsStore, useUIStore } from '@/store';
import { apiService, settingsApi, accountApi, devinApi } from '@/api';
import type { Account } from '@/types';
import AccountCard from '@/components/AccountCard.vue';
import AddAccountDialog from '@/components/AddAccountDialog.vue';
import EditAccountDialog from '@/components/EditAccountDialog.vue';
import SettingsDialog from '@/components/SettingsDialog.vue';
import BatchImportDialog from '@/components/BatchImportDialog.vue';
import BatchExportDialog from '@/components/BatchExportDialog.vue';
import LogsDialog from '@/components/LogsDialog.vue';
import StatsDialog from '@/components/StatsDialog.vue';
import BillingDialog from '@/components/BillingDialog.vue';
import AccountInfoDialog from '@/components/AccountInfoDialog.vue';
import AboutDialog from '@/components/AboutDialog.vue';
import UpdateDialog from '@/components/UpdateDialog.vue';
import { useUpdaterStore } from '@/store/modules/updater';
import BatchUpdatePlanDialog from '@/components/BatchUpdatePlanDialog.vue';
import TagManageDialog from '@/components/TagManageDialog.vue';
import AutoResetDialog from '@/components/AutoResetDialog.vue';
import CardGeneratorDialog from '@/components/CardGeneratorDialog.vue';
import ThemeSelector from '@/components/ThemeSelector.vue';

const accountsStore = useAccountsStore();
const settingsStore = useSettingsStore();
const uiStore = useUIStore();

const SIDEBAR_MENU_EXPAND_DELAY_MS = 140;
const ACCOUNT_CARD_INITIAL_RENDER_COUNT = 20;
const ACCOUNT_CARD_RENDER_BATCH_SIZE = 20;
const menuCollapsed = ref(uiStore.sidebarCollapsed);
let sidebarMenuTimer: ReturnType<typeof setTimeout> | undefined;
let accountRenderFrameId: number | undefined;

const activeMenu = ref('accounts');
const searchQuery = ref('');
const currentBillingData = ref<any>(null);
const billingLoading = ref(false);
const currentWindsurfEmail = ref<string>('');
const windsurfVersion = ref<string>('');
const windsurfClientDisplayName = ref<string>('Windsurf');
const showBatchUpdatePlanDialog = ref(false);
const batchPlanAccounts = ref<Account[]>([]);
const showAbout = ref(false);
const showUpdateDialog = ref(false);
const updaterStore = useUpdaterStore();
const showTagManageDialog = ref(false);
const showBatchImportDialog = ref(false);
const batchImportDialogRef = ref<InstanceType<typeof BatchImportDialog> | null>(null);
const showBatchExportDialog = ref(false);
const batchExportAccounts = ref<Account[]>([]);
const appVersion = ref<string>('');  // 版本号从后端动态获取
const showBatchGroupDialog = ref(false);
const batchGroupTarget = ref('');
const isBatchUpdatingGroup = ref(false);
const showAutoResetDialog = ref(false);
const showCardGeneratorDialog = ref(false);
const visibleAccountCount = ref(ACCOUNT_CARD_INITIAL_RENDER_COUNT);
const renderedAccounts = computed(() => accountsStore.paginatedAccounts.slice(0, visibleAccountCount.value));
const isRenderingAccountBatch = computed(() => visibleAccountCount.value < accountsStore.paginatedAccounts.length);

// 排序相关
const currentSortField = ref<string>('custom');
const sortDirection = ref<'asc' | 'desc'>('asc');

// 处理排序变更
async function handleSortChange(field: string) {
  currentSortField.value = field;
  await accountsStore.setSortConfig(field as any, sortDirection.value);
}

// 切换排序方向
async function toggleSortDirection() {
  sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc';
  await accountsStore.setSortConfig(currentSortField.value as any, sortDirection.value);
}

// 初始化排序配置
async function initSortConfig() {
  await accountsStore.loadSortConfig();
  // 处理旧的 custom 值，自动改为 created_at
  const field = accountsStore.sortConfig.field as string;
  currentSortField.value = (field === 'custom' ? 'created_at' : field) as any;
  sortDirection.value = accountsStore.sortConfig.direction;
}

// 批量转让订阅
const showBatchTransferDialog = ref(false);
const batchTransferEmails = ref('');
const batchTransferring = ref(false);
const batchTransferProgress = ref({ current: 0, total: 0, status: '' });

// 解析输入的邮箱列表（支持"邮箱"或"邮箱 密码"格式）
const parsedTransferEmails = computed(() => {
  return batchTransferEmails.value
    .split('\n')
    .map(line => {
      const trimmed = line.trim();
      // 支持空格或制表符分隔的格式，只取第一部分（邮箱）
      const parts = trimmed.split(/[\s\t]+/);
      return parts[0] || '';
    })
    .filter(e => e && e.includes('@'));
});

// 自动重置定时器
interface AutoResetConfig {
  id: string;
  targetType: string;
  targetId: string;
  enabled: boolean;
  checkInterval: number;
  usageThreshold: number;
  remainingThreshold: number;
}
const autoResetTimerMap = ref<Map<string, ReturnType<typeof setInterval>>>(new Map());

// 初始化自动重置定时器
async function initAutoResetTimers() {
  try {
    const configs = await invoke<AutoResetConfig[]>('get_auto_reset_configs');
    
    // 清除现有定时器
    autoResetTimerMap.value.forEach(timer => clearInterval(timer));
    autoResetTimerMap.value.clear();
    
    // 为每个启用的配置设置定时器
    configs.filter(c => c.enabled).forEach(config => {
      // 立即执行一次检查
      executeAutoResetCheck(config.id);
      
      // 设置定时器
      const timer = setInterval(() => {
        executeAutoResetCheck(config.id);
      }, config.checkInterval * 60 * 1000);
      
      autoResetTimerMap.value.set(config.id, timer);
    });
    
    if (configs.filter(c => c.enabled).length > 0) {
      console.log(`[AutoReset] 已启动 ${configs.filter(c => c.enabled).length} 个自动重置定时器`);
    }
  } catch (error) {
    console.error('[AutoReset] 初始化定时器失败:', error);
  }
}

// 执行自动重置检查
async function executeAutoResetCheck(configId: string) {
  try {
    const result = await invoke<any>('check_and_auto_reset', { configId });
    
    if (result.reset_count > 0) {
      ElMessage.success(`自动重置: 重置了 ${result.reset_count} 个账号的积分`);
      await accountsStore.loadAccounts();
    }
  } catch (error) {
    console.error('[AutoReset] 检查失败:', error);
  }
}

// ==================== 额度耗尽自动切换定时器（Fork 新增） ====================
//
// 与 auto_reset 的实现思路对齐：
// - 前端持有定时器，按 Settings.autoSwitchCheckInterval 周期性调用后端
// - 后端 check_and_auto_switch 内部判断 enabled 开关，前端不重复判断
// - 仅当 settings.autoSwitchEnabled 为 true 时启动定时器
// - watch settings 变化即时重启定时器，避免用户改了配置不生效
const autoSwitchTimer = ref<ReturnType<typeof setInterval> | null>(null);
let autoSwitchUnlisten: UnlistenFn | null = null;

async function executeAutoSwitchCheck() {
  try {
    const result = await apiService.checkAndAutoSwitch();
    if (result.triggered) {
      ElMessage.success(
        `[自动切换] ${result.current_email} → ${result.switched_to_email}`
      );
      // 切换后刷新账号列表（新账号的 last_login_at 等字段会变）
      await accountsStore.loadAccounts();
      await fetchCurrentWindsurfInfo();
    } else if (result.reason && !result.reason.includes('未达切换条件')) {
      // 触发失败/跳过的告警类原因（不包括正常的"未达阈值"）
      console.warn('[AutoSwitch]', result.reason);
    }
  } catch (error) {
    console.error('[AutoSwitch] 检查失败:', error);
  }
}

function clearAutoSwitchTimer() {
  if (autoSwitchTimer.value) {
    clearInterval(autoSwitchTimer.value);
    autoSwitchTimer.value = null;
  }
}

function initAutoSwitchTimer() {
  clearAutoSwitchTimer();

  const s = settingsStore.settings as any;
  if (!s?.autoSwitchEnabled) {
    return;
  }

  // ⚠️ 下限锁 3 分钟：实测 1 分钟会触发 Windsurf rate limit (GetPlanStatus 拒绝服务)
  // 即使用户在配置文件里手改成 1，运行时也强制提到 3
  const intervalMin = Math.max(3, Number(s.autoSwitchCheckInterval) || 5);
  console.log(`[AutoSwitch] 启动定时器：每 ${intervalMin} 分钟检查一次`);

  // 启动后延迟 30 秒做第一次检查（避开应用启动高峰）
  window.setTimeout(() => {
    executeAutoSwitchCheck();
  }, 30 * 1000);

  autoSwitchTimer.value = setInterval(() => {
    executeAutoSwitchCheck();
  }, intervalMin * 60 * 1000);
}

// 后端 emit 的切换结果事件（手动测试 + 定时器都会触发，避免重复 toast）
async function setupAutoSwitchListener() {
  if (autoSwitchUnlisten) {
    autoSwitchUnlisten();
    autoSwitchUnlisten = null;
  }
  autoSwitchUnlisten = await listen<{
    triggered: boolean;
    current_email?: string;
    switched_to_email?: string;
    reason?: string;
  }>('auto-switch-result', (event) => {
    const payload = event.payload;
    // triggered=false 但 reason 是兜底警示（如"无可用账号"），用 warning toast
    if (!payload.triggered && payload.reason && payload.current_email) {
      ElMessage.warning(`[自动切换] ${payload.reason}`);
    }
  });
}

// 筛选面板状态
const showFilterPanel = ref(false);
// 状态选项
const statusOptions = [
  { value: 'normal', label: '🟢 正常', desc: '账户正常' },
  { value: 'inactive', label: '🔘 未激活', desc: '订阅未激活' },
  { value: 'disabled', label: '🟠 已禁用', desc: '被 Windsurf 禁用' },
  { value: 'offline', label: '⚪ 离线', desc: 'Token 失效' },
  { value: 'error', label: '🔴 错误', desc: '操作异常' },
];

const filterForm = ref({
  remainingQuotaMin: undefined as number | undefined,
  remainingQuotaMax: undefined as number | undefined,
  totalQuotaMin: undefined as number | undefined,
  totalQuotaMax: undefined as number | undefined,
  expiryDaysMin: undefined as number | undefined,
  expiryDaysMax: undefined as number | undefined,
  // 日/周配额剩余百分比（0-100，仅 billing_strategy === 2 (QUOTA) 账号参与）
  dailyQuotaPercentMin: undefined as number | undefined,
  dailyQuotaPercentMax: undefined as number | undefined,
  weeklyQuotaPercentMin: undefined as number | undefined,
  weeklyQuotaPercentMax: undefined as number | undefined,
  selectedTags: [] as string[],
  selectedPlans: [] as string[],
  selectedDomains: [] as string[],
  selectedStatuses: [] as string[],
});

// 是否有激活的筛选条件
const hasActiveFilter = computed(() => {
  const f = accountsStore.currentFilter;
  return !!(
    f.remainingQuotaMin !== undefined ||
    f.remainingQuotaMax !== undefined ||
    f.totalQuotaMin !== undefined ||
    f.totalQuotaMax !== undefined ||
    f.expiryDaysMin !== undefined ||
    f.expiryDaysMax !== undefined ||
    f.dailyQuotaPercentMin !== undefined ||
    f.dailyQuotaPercentMax !== undefined ||
    f.weeklyQuotaPercentMin !== undefined ||
    f.weeklyQuotaPercentMax !== undefined ||
    (f.tags && f.tags.length > 0) ||
    (f.planNames && f.planNames.length > 0) ||
    (f.domains && f.domains.length > 0) ||
    (f.statuses && f.statuses.length > 0)
  );
});

const sidebarWidth = computed(() => uiStore.sidebarCollapsed ? '64px' : '240px');

function clearSidebarMenuTimer() {
  if (sidebarMenuTimer !== undefined) {
    clearTimeout(sidebarMenuTimer);
    sidebarMenuTimer = undefined;
  }
}

function scheduleMenuExpand() {
  clearSidebarMenuTimer();
  sidebarMenuTimer = setTimeout(() => {
    if (!uiStore.sidebarCollapsed) {
      menuCollapsed.value = false;
    }
    sidebarMenuTimer = undefined;
  }, SIDEBAR_MENU_EXPAND_DELAY_MS);
}

function toggleSidebar() {
  if (uiStore.sidebarCollapsed) {
    uiStore.toggleSidebar();
    scheduleMenuExpand();
    return;
  }

  menuCollapsed.value = true;
  requestAnimationFrame(() => {
    uiStore.toggleSidebar();
  });
}

watch(() => uiStore.sidebarCollapsed, (collapsed) => {
  if (collapsed) {
    clearSidebarMenuTimer();
    menuCollapsed.value = true;
    return;
  }

  scheduleMenuExpand();
});

function clearAccountRenderFrame() {
  if (accountRenderFrameId !== undefined) {
    cancelAnimationFrame(accountRenderFrameId);
    accountRenderFrameId = undefined;
  }
}

function scheduleAccountBatchRender() {
  clearAccountRenderFrame();

  const renderNextBatch = () => {
    const total = accountsStore.paginatedAccounts.length;
    if (visibleAccountCount.value >= total) {
      accountRenderFrameId = undefined;
      return;
    }

    visibleAccountCount.value = Math.min(
      visibleAccountCount.value + ACCOUNT_CARD_RENDER_BATCH_SIZE,
      total
    );

    if (visibleAccountCount.value < total) {
      accountRenderFrameId = requestAnimationFrame(renderNextBatch);
      return;
    }

    accountRenderFrameId = undefined;
  };

  accountRenderFrameId = requestAnimationFrame(renderNextBatch);
}

watch(
  () => accountsStore.paginatedAccounts.map(account => account.id).join('|'),
  () => {
    const total = accountsStore.paginatedAccounts.length;
    visibleAccountCount.value = Math.min(ACCOUNT_CARD_INITIAL_RENDER_COUNT, total);

    if (visibleAccountCount.value < total) {
      scheduleAccountBatchRender();
      return;
    }

    clearAccountRenderFrame();
  },
  { immediate: true }
);

// 全选状态：在分组视图中用该分组账号数判断，无分组时用总数
const isAllSelected = computed(() => {
  const size = accountsStore.selectedAccounts.size;
  if (size === 0) return false;
  const group = accountsStore.currentFilter.group;
  const target = group
    ? (accountsStore.aggregates.group_counts[group] ?? 0)
    : accountsStore.aggregates.total_count;
  return size >= target && target > 0;
});

function setActiveMenu(menu: string) {
  activeMenu.value = menu;
  accountsStore.clearFilter();
}

function getGroupAccountCount(group: string): number {
  return accountsStore.aggregates.group_counts[group] ?? 0;
}

function filterByGroup(group: string) {
  accountsStore.setFilter({ group });
}

function handleSearch() {
  accountsStore.setFilter({ ...accountsStore.currentFilter, search: searchQuery.value });
}

// 应用筛选
function applyFilters() {
  accountsStore.setFilter({
    ...accountsStore.currentFilter,
    remainingQuotaMin: filterForm.value.remainingQuotaMin,
    remainingQuotaMax: filterForm.value.remainingQuotaMax,
    totalQuotaMin: filterForm.value.totalQuotaMin,
    totalQuotaMax: filterForm.value.totalQuotaMax,
    expiryDaysMin: filterForm.value.expiryDaysMin,
    expiryDaysMax: filterForm.value.expiryDaysMax,
    dailyQuotaPercentMin: filterForm.value.dailyQuotaPercentMin,
    dailyQuotaPercentMax: filterForm.value.dailyQuotaPercentMax,
    weeklyQuotaPercentMin: filterForm.value.weeklyQuotaPercentMin,
    weeklyQuotaPercentMax: filterForm.value.weeklyQuotaPercentMax,
    tags: filterForm.value.selectedTags.length > 0 ? filterForm.value.selectedTags : undefined,
    planNames: filterForm.value.selectedPlans.length > 0 ? filterForm.value.selectedPlans : undefined,
    domains: filterForm.value.selectedDomains.length > 0 ? filterForm.value.selectedDomains : undefined,
    statuses: filterForm.value.selectedStatuses.length > 0 ? filterForm.value.selectedStatuses as any : undefined,
  });
}

// 清除所有筛选
function clearAllFilters() {
  filterForm.value = {
    remainingQuotaMin: undefined,
    remainingQuotaMax: undefined,
    totalQuotaMin: undefined,
    totalQuotaMax: undefined,
    expiryDaysMin: undefined,
    expiryDaysMax: undefined,
    dailyQuotaPercentMin: undefined,
    dailyQuotaPercentMax: undefined,
    weeklyQuotaPercentMin: undefined,
    weeklyQuotaPercentMax: undefined,
    selectedTags: [],
    selectedPlans: [],
    selectedDomains: [],
    selectedStatuses: [],
  };
  accountsStore.clearFilter();
  searchQuery.value = '';
}

// 分页处理
function handlePageSizeChange(size: number) {
  accountsStore.setPageSize(size);
}

function handleCurrentPageChange(page: number) {
  accountsStore.setCurrentPage(page);
}

function handleAccountSelect(accountId: string, selected: boolean) {
  if (selected) {
    accountsStore.selectedAccounts.add(accountId);
  } else {
    accountsStore.selectedAccounts.delete(accountId);
  }
}

function handleAccountUpdate(account: Account) {
  accountsStore.updateAccount(account);
}

async function refreshAccounts() {
  const loading = ElMessage({
    message: '正在刷新账号列表...',
    duration: 0,
    icon: Loading
  });
  
  try {
    // 批量刷新所有账号（v1.7.8 方案 B：从后端获取全部 ID，不遍历当前页数组）
    const allIds = await accountApi.getAllAccountIds();
    if (allIds.length > 0) {
      loading.close();
      
      const totalCount = allIds.length;
      
      const progressLoading = ElMessage({
        message: `正在批量刷新 ${totalCount} 个账号...`,
        duration: 0,
        icon: Loading
      });
      
      // 使用优化的批量刷新 API（后端只保存一次）
      const result = await apiService.batchRefreshTokens(allIds);
      
      progressLoading.close();
      
      const successCount = result.success_count || 0;
      const failedCount = totalCount - successCount;
      
      // v1.7.8：后端已将刷新结果写入 SQLite，前端刷新当前页即可
      await Promise.all([accountsStore.fetchPage(), accountsStore.refreshAggregates()]);
      
      // 显示详细的刷新结果
      if (failedCount === 0) {
        ElMessage.success({
          message: `✅ 全部刷新完成！\n成功: ${successCount}/${totalCount}`,
          duration: 3000,
          showClose: true
        });
      } else {
        const failedItems = result.results?.filter((r: any) => !r.success) || [];
        const failedDetails = failedItems.slice(0, 3).map((item: any) => {
          return `  • ${item.id}: ${item.error || '未知错误'}`;
        }).join('\n');
        const moreFailures = failedItems.length > 3 ? `\n  ... 还有 ${failedItems.length - 3} 个失败` : '';
        
        ElMessage.warning({
          message: `⚠️ 刷新完成（部分失败）\n成功: ${successCount}/${totalCount}\n失败: ${failedCount}/${totalCount}\n\n失败账号:\n${failedDetails}${moreFailures}`,
          duration: 5000,
          showClose: true,
          dangerouslyUseHTMLString: false
        });
      }
    } else {
      loading.close();
      ElMessage.success('账号列表已刷新');
    }
  } catch (error) {
    loading.close();
    ElMessage.error(`刷新失败: ${error}`);
  }
}

async function handleBatchDelete() {
  try {
    await ElMessageBox.confirm(
      `确定要删除选中的 ${accountsStore.selectedAccounts.size} 个账号吗？`,
      '批量删除确认',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );
    
    const result = await accountsStore.deleteSelectedAccounts();
    ElMessage.success(`成功删除 ${result?.success_count || 0} 个账号`);
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`批量删除失败: ${error}`);
    }
  }
}

// 批量转让订阅（并发执行）
async function handleBatchTransfer() {
  const selectedIds = Array.from(accountsStore.selectedAccounts);
  const targetEmails = parsedTransferEmails.value;
  
  if (selectedIds.length !== targetEmails.length) {
    ElMessage.warning('源账户数量与目标邮箱数量不匹配');
    return;
  }
  
  try {
    await ElMessageBox.confirm(
      `确定要将 ${selectedIds.length} 个账户的订阅转让给对应的目标邮箱吗？\n\n转让后源账户将被移出团队，此操作不可撤销！`,
      '确认批量转让',
      {
        confirmButtonText: '确认转让',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );
  } catch {
    return;
  }
  
  batchTransferring.value = true;
  batchTransferProgress.value = { current: 0, total: selectedIds.length, status: '并发执行中...' };
  
  // 构建转让任务列表
  const transferTasks = selectedIds.map((sourceId, index) => {
    const targetEmail = targetEmails[index];
    const sourceAccount = accountsStore.accounts.find(a => a.id === sourceId);
    const sourceEmail = sourceAccount?.email || sourceId;
    
    return (async () => {
      try {
        const result = await invoke<any>('transfer_subscription', {
          id: sourceId,
          targetEmail: targetEmail,
          targetName: targetEmail.split('@')[0]
        });
        
        // 更新进度
        batchTransferProgress.value = {
          ...batchTransferProgress.value,
          current: batchTransferProgress.value.current + 1
        };
        
        if (result.success) {
          return { sourceEmail, targetEmail, success: true };
        } else {
          return { sourceEmail, targetEmail, success: false, error: result.error || '转让失败' };
        }
      } catch (error: any) {
        batchTransferProgress.value = {
          ...batchTransferProgress.value,
          current: batchTransferProgress.value.current + 1
        };
        return { sourceEmail, targetEmail, success: false, error: error.toString() };
      }
    })();
  });
  
  // 并发执行所有转让任务
  const results = await Promise.all(transferTasks);
  
  batchTransferProgress.value = {
    current: selectedIds.length,
    total: selectedIds.length,
    status: '完成'
  };
  
  batchTransferring.value = false;
  
  const successCount = results.filter(r => r.success).length;
  const failedCount = results.filter(r => !r.success).length;
  
  if (failedCount === 0) {
    ElMessage.success(`批量转让完成！成功: ${successCount}/${selectedIds.length}`);
  } else {
    const failedDetails = results
      .filter(r => !r.success)
      .slice(0, 3)
      .map(r => `${r.sourceEmail}: ${r.error}`)
      .join('\n');
    ElMessage.warning({
      message: `批量转让完成\n成功: ${successCount}, 失败: ${failedCount}\n\n失败详情:\n${failedDetails}`,
      duration: 5000,
      showClose: true
    });
  }
  
  // 关闭对话框并清理
  showBatchTransferDialog.value = false;
  batchTransferEmails.value = '';
  accountsStore.clearSelection();
  
  // 刷新账号列表
  await accountsStore.loadAccounts();
}

async function refreshBillingData() {
  if (uiStore.currentViewingAccountId) {
    billingLoading.value = true;
    currentBillingData.value = null;
    
    try {
      const result = await apiService.getBilling(uiStore.currentViewingAccountId);
      currentBillingData.value = result;
    } catch (error) {
      ElMessage.error(`获取账单信息失败: ${error}`);
    } finally {
      billingLoading.value = false;
    }
  }
}

// 监听账单对话框打开
watch(() => uiStore.showBillingDialog, (show) => {
  if (show && uiStore.currentViewingAccountId) {
    refreshBillingData();
  } else if (!show) {
    currentBillingData.value = null;
  }
});

// 全选/取消全选（v1.7.8：在分组视图中只选当前分组，无分组时选全部）
async function toggleSelectAll() {
  const group = accountsStore.currentFilter.group;
  const targetCount = group
    ? (accountsStore.aggregates.group_counts[group] ?? 0)
    : accountsStore.aggregates.total_count;

  if (accountsStore.selectedAccounts.size > 0 && accountsStore.selectedAccounts.size >= targetCount) {
    accountsStore.clearSelection();
  } else {
    await accountsStore.selectAll();
    const label = group ? `"${group}" 分组` : '全部';
    ElMessage.success(`已全选${label} ${accountsStore.selectedAccounts.size} 个账号`);
  }
}

// 选择本页账号
function selectCurrentPageAccounts() {
  const pageAccounts = accountsStore.paginatedAccounts;
  if (pageAccounts.length === 0) {
    ElMessage.info('当前页没有账号');
    return;
  }
  
  // 检查本页是否已全部选中
  const allSelected = pageAccounts.every(acc => accountsStore.selectedAccounts.has(acc.id));
  
  if (allSelected) {
    // 如果本页已全选，则取消本页选择
    pageAccounts.forEach(account => {
      accountsStore.selectedAccounts.delete(account.id);
    });
    ElMessage.info(`已取消选择本页 ${pageAccounts.length} 个账号`);
  } else {
    // 选择本页所有账号
    pageAccounts.forEach(account => {
      accountsStore.selectedAccounts.add(account.id);
    });
    ElMessage.success(`已选择本页 ${pageAccounts.length} 个账号`);
  }
}

// 打开批量导入对话框
function handleBatchImport() {
  showBatchImportDialog.value = true;
}

// 批量导入确认（从对话框接收数据）
async function handleBatchImportConfirm(
  accountsToImport: Array<{ email: string; password: string; remark: string; refreshToken?: string; sessionToken?: string; auth1Token?: string }>,
  autoLogin: boolean,
  group: string = '默认分组',
  tags: string[] = [],
  mode: 'password' | 'refresh_token' | 'devin_session_token' | 'devin_auth1_token' = 'password',
  // v1.7.6：新增 `'devin_native'` 支持纯 Devin 原生账号的批量导入
  authProvider: 'firebase' | 'devin' | 'devin_native' | 'smart' = 'firebase'
) {
  // 获取并发设置
  const unlimitedConcurrent = settingsStore.settings?.unlimitedConcurrentRefresh || false;
  const concurrencyLimit = settingsStore.settings?.concurrent_limit || 5;

  // === Devin Session Token 模式：逐条调 add_account_by_devin_session_token，不走 sniff/importTask ===
  if (mode === 'devin_session_token') {
    await handleDevinSessionTokenBatchImport(accountsToImport, group, tags, unlimitedConcurrent, concurrencyLimit);
    return;
  }

  // === Devin Auth1 Token 模式：逐条调 add_account_by_devin_auth1_token（autoSelectPrimaryOrg=true） ===
  if (mode === 'devin_auth1_token') {
    await handleDevinAuth1TokenBatchImport(accountsToImport, group, tags, unlimitedConcurrent, concurrencyLimit);
    return;
  }

  const providerLabel =
    authProvider === 'devin' ? 'Devin'
    : authProvider === 'devin_native' ? 'Devin 原生'
    : authProvider === 'smart' ? '智能识别'
    : 'Firebase';
  const modeLabel = mode === 'refresh_token' ? 'Refresh Token' : '邮箱密码';
  const fullLabel = `${providerLabel} · ${modeLabel}`;
  // 多组织自动首选计数：批量结束后汇总提示
  let devinAutoOrgPickedCount = 0;
  // 智能模式下的嗅探结果：email -> 实际走的 provider
  // 只给崇探成功且归类为 firebase/devin/devin_native 的账号记录；其余直接进入失败列表
  const resolvedProviders = new Map<string, 'firebase' | 'devin' | 'devin_native'>();
  // 嗅探环节被跳过的账号（SSO / 未设密码 / 未注册 / 企业禁许 / 网络异常等）
  const skippedBySniff: Array<{ email: string; reason: string }> = [];

  // 显示进度提示
  let progressMsg = ElMessage({
    message: unlimitedConcurrent
      ? `正在全量并发导入 ${accountsToImport.length} 个账号（${fullLabel}）...`
      : `正在导入 ${accountsToImport.length} 个账号（${fullLabel}，并发${concurrencyLimit}）...`,
    duration: 0,
    icon: Loading
  });

  // === 智能识别模式：导入前先并发嗅探所有账号的登录流派 ===
  if (authProvider === 'smart' && mode === 'password') {
    progressMsg.close();
    progressMsg = ElMessage({
      message: `正在识别 ${accountsToImport.length} 个账号的登录类型……`,
      duration: 0,
      icon: Loading
    });

    const sniffResults = await Promise.all(
      accountsToImport.map(async (item) => {
        try {
          const sniff = await devinApi.sniffLoginMethod(item.email);
          return { email: item.email, sniff, error: null as string | null };
        } catch (e) {
          return { email: item.email, sniff: null, error: String(e) };
        }
      })
    );

    for (const r of sniffResults) {
      if (r.error || !r.sniff) {
        skippedBySniff.push({ email: r.email, reason: `嗅探失败: ${r.error || '未知错误'}` });
        continue;
      }
      switch (r.sniff.recommended) {
        case 'firebase':
          resolvedProviders.set(r.email, 'firebase');
          break;
        case 'devin':
          resolvedProviders.set(r.email, 'devin');
          break;
        case 'devin_native':
          // v1.7.6 新增：纯 Devin 原生账号，走 add_account_by_devin_native_login
          resolvedProviders.set(r.email, 'devin_native');
          break;
        default:
          // sso / no_password / not_found / blocked / devin_native_no_password
          //   —— 均无法自动导入，挂失败
          skippedBySniff.push({
            email: r.email,
            reason: `[${r.sniff.recommended}] ${r.sniff.reason}`,
          });
      }
    }

    progressMsg.close();
    progressMsg = ElMessage({
      message: unlimitedConcurrent
        ? `嗅探完成，正在全量并发导入 ${resolvedProviders.size} 个账号……`
        : `嗅探完成，正在导入 ${resolvedProviders.size} 个账号（并发${concurrencyLimit}）……`,
      duration: 0,
      icon: Loading
    });
  }

  // 崇探被跳过的账号不进入 importTask，但要在结果集中序列语义上表现为失败
  const itemsToImport =
    authProvider === 'smart' && mode === 'password'
      ? accountsToImport.filter(item => resolvedProviders.has(item.email))
      : accountsToImport;

  const results: Array<{
    email: string;
    success: boolean;
    accountId?: string;
    error?: string;
    effectiveProvider?: 'firebase' | 'devin' | 'devin_native';
  }> = skippedBySniff.map(s => ({ email: s.email, success: false, error: s.reason }));

  // 单个导入任务
  const importTask = async (item: { email: string; password: string; remark: string; refreshToken?: string }) => {
    // 计算本条实际走的 provider（智能模式从嗅探结果取，其余模式直接用 authProvider）
    const effectiveProvider: 'firebase' | 'devin' | 'devin_native' =
      authProvider === 'smart' ? resolvedProviders.get(item.email)! : authProvider;

    try {
      if (mode === 'refresh_token' && item.refreshToken) {
        // Refresh Token 模式：调用后端命令（仅 Firebase；Devin/smart 在对话框侧已禁用该 radio）
        const result = await invoke<any>('add_account_by_refresh_token', {
          refreshToken: item.refreshToken,
          nickname: item.remark || undefined,
          tags: tags.length > 0 ? [...tags] : [],
          group: group
        });
        
        if (result.success) {
          return { email: result.email, success: true, accountId: result.account?.id, effectiveProvider };
        } else {
          return { email: item.email, success: false, error: result.error || '添加失败', effectiveProvider };
        }
      } else if (effectiveProvider === 'devin' || effectiveProvider === 'devin_native') {
        // Devin 账密导入：根据 provider 选端口，其余逻辑（多组织自动首选 orgs[0]）完全共用
        // - devin       → add_account_by_devin_login         （走 windsurf.com/_devin-auth/password/login）
        // - devin_native→ add_account_by_devin_native_login  （走 app.devin.ai/api/auth1/password/login，v1.7.6）
        // 后端两个命令的响应结构完全对齐（DevinLoginResult），后续 multi-org + fall-back
        // 调用 add_account_by_devin_with_org 的链路并无差异。
        const loginCommand =
          effectiveProvider === 'devin_native'
            ? 'add_account_by_devin_native_login'
            : 'add_account_by_devin_login';
        const providerTag = effectiveProvider === 'devin_native' ? '[Devin 原生]' : '[Devin]';

        const loginResult = await invoke<any>(loginCommand, {
          email: item.email,
          password: item.password,
          nickname: item.remark || undefined,
          tags: tags.length > 0 ? [...tags] : [],
          group: group,
          orgId: null,
        });

        if (loginResult?.success && !loginResult?.requires_org_selection) {
          // 单组织直通：后端已完成落库 + enrich
          return {
            email: loginResult.email || item.email,
            success: true,
            accountId: loginResult.account?.id,
            effectiveProvider,
          };
        }

        if (loginResult?.requires_org_selection) {
          const orgs: Array<{ org_id?: string; id?: string; name?: string }> = loginResult.orgs || [];
          const firstOrgId = orgs[0]?.org_id || orgs[0]?.id;
          if (!firstOrgId) {
            return {
              email: item.email,
              success: false,
              error: `${providerTag} 多组织但 orgs[] 为空，无法自动选择`,
              effectiveProvider,
            };
          }
          const auth1Token: string = loginResult.auth1_token;
          if (!auth1Token) {
            return {
              email: item.email,
              success: false,
              error: `${providerTag} 多组织响应缺失 auth1_token`,
              effectiveProvider,
            };
          }
          // 二次落库（使用首个组织）——传 password 让账号卡可回显用户原始密码
          // add_account_by_devin_with_org 内部使用 auth1_token 调 WindsurfPostAuth，不区分端口来源
          const withOrgResult = await invoke<any>('add_account_by_devin_with_org', {
            email: item.email,
            auth1Token,
            orgId: firstOrgId,
            nickname: item.remark || undefined,
            tags: tags.length > 0 ? [...tags] : [],
            group: group,
            password: item.password,
          });

          if (withOrgResult?.success) {
            devinAutoOrgPickedCount += 1;
            return {
              email: withOrgResult.email || item.email,
              success: true,
              accountId: withOrgResult.account?.id,
              effectiveProvider,
            };
          }
          return {
            email: item.email,
            success: false,
            error: withOrgResult?.error || `${providerTag} 多组织二次落库失败`,
            effectiveProvider,
          };
        }

        // 既非 success 也非 requires_org_selection
        return {
          email: item.email,
          success: false,
          error: loginResult?.error || loginResult?.message || `${providerTag} 登录失败`,
          effectiveProvider,
        };
      } else {
        // 邮箱密码模式（Firebase）
        const newAccount = await accountsStore.addAccount({
          email: item.email,
          password: item.password,
          nickname: item.remark || item.email.split('@')[0],
          tags: tags.length > 0 ? [...tags] : [],
          group: group
        });
        return { email: item.email, success: true, accountId: newAccount.id, effectiveProvider };
      }
    } catch (error) {
      console.error(`导入账号 ${item.email} 失败:`, error);
      return { email: item.email, success: false, error: String(error), effectiveProvider };
    }
  };
  
  // 节流刷新：每成功导入一个账号就刷新 UI，但间隔不小于 500ms
  let lastRefreshTime = 0;
  const throttledRefresh = () => {
    const now = Date.now();
    if (now - lastRefreshTime > 500) {
      lastRefreshTime = now;
      accountsStore.fetchPage().catch(() => {});
      accountsStore.refreshAggregates().catch(() => {});
    }
  };

  try {
    if (unlimitedConcurrent) {
      // 全量并发导入（带逐条进度 + 实时 UI 刷新）
      let completedCount = 0;
      const total = itemsToImport.length;
      const allResults = await Promise.all(itemsToImport.map(async (item) => {
        const result = await importTask(item);
        completedCount++;
        progressMsg.close();
        progressMsg = ElMessage({
          message: `导入进度: ${completedCount}/${total}（${result.success ? '✓' : '✗'} ${result.email}）`,
          duration: 0,
          icon: Loading
        });
        if (result.success) throttledRefresh();
        return result;
      }));
      results.push(...allResults);
    } else {
      // 分批并发处理
      for (let i = 0; i < itemsToImport.length; i += concurrencyLimit) {
        const batch = itemsToImport.slice(i, i + concurrencyLimit);
        const batchResults = await Promise.all(batch.map(item => importTask(item)));
        results.push(...batchResults);
        // 每批完成后刷新 UI
        throttledRefresh();
        
        // 更新进度
        progressMsg.close();
        progressMsg = ElMessage({
          message: `导入进度: ${results.length - skippedBySniff.length}/${itemsToImport.length}`,
          duration: 0,
          icon: Loading
        });
      }
    }
    
    // 统计添加结果
    const addedAccounts = results.filter(r => r.success);
    const failedAccounts = results.filter(r => !r.success);
    
    // 并发登录成功添加的账号
    // - refresh_token 模式：已拿账号信息，跳过
    // - Devin 模式：add_account_by_devin_login 已完成 post_auth + enrich_account_with_plan_status，跳过
    // - Firebase + 邮箱密码：按 autoLogin 选项决定是否逐账号调 loginAccount
    //
    // 智能模式下同一批中 Firebase/Devin 混合，按 result.effectiveProvider === 'firebase' 过滤
    const needsAutoLogin = (r: typeof results[number]) =>
      r.success && r.effectiveProvider === 'firebase';
    let loginSuccessCount = 0;
    if (autoLogin && addedAccounts.some(needsAutoLogin) && mode === 'password') {
      progressMsg.close();
      progressMsg = ElMessage({
        message: unlimitedConcurrent
          ? `正在全量并发登录 ${addedAccounts.length} 个账号...`
          : `正在登录 ${addedAccounts.length} 个账号（并发${concurrencyLimit}）...`,
        duration: 0,
        icon: Loading
      });
      
      // Firebase 子集（智能模式下仅包含 effectiveProvider === 'firebase' 的行）
      const firebaseAddedAccounts = addedAccounts.filter(needsAutoLogin);

      // 单个登录任务
      const loginTask = async (item: { email: string; accountId?: string }) => {
        try {
          const loginResult = await apiService.loginAccount(item.accountId!);
          if (loginResult.success) {
            // 从后端获取完整的账号信息（包含token）
            const latestAccount = await accountApi.getAccount(item.accountId!);
            await accountsStore.updateAccount(latestAccount);
            return { success: true };
          }
          return { success: false };
        } catch (loginError) {
          console.error(`账号 ${item.email} 登录失败:`, loginError);
          return { success: false };
        }
      };
      
      const loginResults: Array<{ success: boolean }> = [];
      
      if (unlimitedConcurrent) {
        // 全量并发登录
        const allLoginResults = await Promise.all(firebaseAddedAccounts.map(item => loginTask(item)));
        loginResults.push(...allLoginResults);
      } else {
        // 分批并发登录
        for (let i = 0; i < firebaseAddedAccounts.length; i += concurrencyLimit) {
          const batch = firebaseAddedAccounts.slice(i, i + concurrencyLimit);
          const batchResults = await Promise.all(batch.map(item => loginTask(item)));
          loginResults.push(...batchResults);
          
          // 更新进度
          progressMsg.close();
          progressMsg = ElMessage({
            message: `登录进度: ${loginResults.length}/${firebaseAddedAccounts.length}`,
            duration: 0,
            icon: Loading
          });
        }
      }
      
      loginSuccessCount = loginResults.filter(r => r.success).length;
    }
    
    progressMsg.close();
    
    // 关闭对话框
    showBatchImportDialog.value = false;
    batchImportDialogRef.value?.resetImporting();
    
    // 显示最终结果
    if (addedAccounts.length > 0) {
      let message = `成功导入 ${addedAccounts.length} 个账号（${providerLabel}）`;
      if (authProvider === 'smart') {
        const firebaseCount = addedAccounts.filter(r => r.effectiveProvider === 'firebase').length;
        const devinCount = addedAccounts.filter(r => r.effectiveProvider === 'devin').length;
        const devinNativeCount = addedAccounts.filter(r => r.effectiveProvider === 'devin_native').length;
        // v1.7.6：汇总说明补充 Devin 原生计数；既有版本不显示 0 的分类避免信息噪声
        const segments = [
          `Firebase ${firebaseCount}`,
          `Devin ${devinCount}`,
          ...(devinNativeCount > 0 ? [`Devin 原生 ${devinNativeCount}`] : []),
        ];
        message += `（${segments.join(' · ')}）`;
      }
      if (autoLogin && loginSuccessCount > 0) {
        message += `，${loginSuccessCount} 个已登录`;
      }
      if (devinAutoOrgPickedCount > 0) {
        message += `，${devinAutoOrgPickedCount} 个多组织账号已自动选择首个组织`;
      }
      if (skippedBySniff.length > 0) {
        message += `，${skippedBySniff.length} 个没识别到可用流派`;
      }
      if (failedAccounts.length - skippedBySniff.length > 0) {
        message += `，其余失败 ${failedAccounts.length - skippedBySniff.length} 个`;
      }
      ElMessage.success({
        message,
        duration: 5000,
        showClose: true
      });
      await accountsStore.loadAccounts();
    } else {
      let errorMsg = '没有成功导入任何账号';
      if (failedAccounts.length > 0) {
        const details = failedAccounts.slice(0, 3).map(f => `${f.email}（${f.error || '未知'}）`).join('\n');
        errorMsg += `\n${details}${failedAccounts.length > 3 ? '\n...' : ''}`;
      }
      ElMessage.error({
        message: errorMsg,
        duration: 5000,
        showClose: true
      });
    }
  } catch (error) {
    progressMsg.close();
    showBatchImportDialog.value = false;
    batchImportDialogRef.value?.resetImporting();
    ElMessage.error(`批量导入失败: ${error}`);
  }
}

/**
 * Devin Auth1 Token 批量导入辅助函数
 *
 * 逐条调 devinApi.addAccountByAuth1Token 并开启 autoSelectPrimaryOrg：
 * 后端用 auth1_token 换取 session_token → 反查 GetCurrentUser 拿 email / 配额 → 落库。
 * 多组织场景自动选择 primary org（批量场景不弹组织选择对话框，保证流程不被中断）。
 *
 * 与 handleDevinSessionTokenBatchImport 对称，区别在于落库后账号的 devin_auth1_token 字段已填充，
 * 后续 session 到期可直接用 refresh_devin_session 刷新，无需重新获取 token。
 */
async function handleDevinAuth1TokenBatchImport(
  items: Array<{ email: string; password: string; remark: string; refreshToken?: string; sessionToken?: string; auth1Token?: string }>,
  group: string,
  tags: string[],
  unlimitedConcurrent: boolean,
  concurrencyLimit: number,
) {
  let progressMsg = ElMessage({
    message: unlimitedConcurrent
      ? `正在全量并发导入 ${items.length} 个 Devin Auth1 Token...`
      : `正在导入 ${items.length} 个 Devin Auth1 Token（并发${concurrencyLimit}）...`,
    duration: 0,
    icon: Loading,
  });

  const results: Array<{ email: string; success: boolean; error?: string }> = [];

  const importTask = async (item: { remark: string; auth1Token?: string }) => {
    if (!item.auth1Token) {
      return { email: '(missing token)', success: false, error: '缺少 auth1Token' };
    }
    try {
      const result = await devinApi.addAccountByAuth1Token({
        auth1Token: item.auth1Token,
        nickname: item.remark || undefined,
        tags: tags.length > 0 ? [...tags] : [],
        group: group,
        autoSelectPrimaryOrg: true,
      });
      if (result.success) {
        return { email: result.email || '(unknown)', success: true };
      }
      return {
        email: result.email || '(unknown)',
        success: false,
        error: result.message || '导入失败',
      };
    } catch (e) {
      return {
        email: item.auth1Token.slice(0, 30) + '...',
        success: false,
        error: String(e),
      };
    }
  };

  try {
    if (unlimitedConcurrent) {
      const all = await Promise.all(items.map(importTask));
      results.push(...all);
    } else {
      for (let i = 0; i < items.length; i += concurrencyLimit) {
        const batch = items.slice(i, i + concurrencyLimit);
        const batchResults = await Promise.all(batch.map(importTask));
        results.push(...batchResults);
        progressMsg.close();
        progressMsg = ElMessage({
          message: `导入进度: ${results.length}/${items.length}`,
          duration: 0,
          icon: Loading,
        });
      }
    }

    progressMsg.close();
    showBatchImportDialog.value = false;
    batchImportDialogRef.value?.resetImporting();

    const succeeded = results.filter(r => r.success).length;
    const failed = results.filter(r => !r.success);
    if (succeeded > 0) {
      let msg = `成功通过 Auth1 Token 导入 ${succeeded} 个 Devin 账号`;
      if (failed.length > 0) msg += `，失败 ${failed.length} 个`;
      ElMessage.success({ message: msg, duration: 5000, showClose: true });
      await accountsStore.loadAccounts();
    } else {
      const details = failed.slice(0, 3).map(f => `${f.email}（${f.error || '未知'}）`).join('\n');
      ElMessage.error({
        message: `没有成功导入任何账号\n${details}${failed.length > 3 ? '\n...' : ''}`,
        duration: 5000,
        showClose: true,
      });
    }
  } catch (e) {
    progressMsg.close();
    showBatchImportDialog.value = false;
    batchImportDialogRef.value?.resetImporting();
    ElMessage.error(`批量导入失败: ${e}`);
  }
}

/**
 * Devin Session Token 批量导入辅助函数
 *
 * 逐条调 devinApi.addAccountBySessionToken，后端反查 GetCurrentUser 拿 email / 配额并落库。
 * 不走 sniff/importTask 链路（session_token 本身就是 Devin 凭证，无需嗅探）。
 */
async function handleDevinSessionTokenBatchImport(
  items: Array<{ email: string; password: string; remark: string; refreshToken?: string; sessionToken?: string }>,
  group: string,
  tags: string[],
  unlimitedConcurrent: boolean,
  concurrencyLimit: number,
) {
  let progressMsg = ElMessage({
    message: unlimitedConcurrent
      ? `正在全量并发导入 ${items.length} 个 Devin Session Token...`
      : `正在导入 ${items.length} 个 Devin Session Token（并发${concurrencyLimit}）...`,
    duration: 0,
    icon: Loading,
  });

  const results: Array<{ email: string; success: boolean; error?: string }> = [];

  const importTask = async (item: { remark: string; sessionToken?: string }) => {
    if (!item.sessionToken) {
      return { email: '(missing token)', success: false, error: '缺少 sessionToken' };
    }
    try {
      const result = await devinApi.addAccountBySessionToken({
        sessionToken: item.sessionToken,
        nickname: item.remark || undefined,
        tags: tags.length > 0 ? [...tags] : [],
        group: group,
      });
      if (result.success) {
        return { email: result.email || '(unknown)', success: true };
      }
      return {
        email: result.email || '(unknown)',
        success: false,
        error: result.message || '导入失败',
      };
    } catch (e) {
      return {
        email: item.sessionToken.slice(0, 30) + '...',
        success: false,
        error: String(e),
      };
    }
  };

  try {
    if (unlimitedConcurrent) {
      const all = await Promise.all(items.map(importTask));
      results.push(...all);
    } else {
      for (let i = 0; i < items.length; i += concurrencyLimit) {
        const batch = items.slice(i, i + concurrencyLimit);
        const batchResults = await Promise.all(batch.map(importTask));
        results.push(...batchResults);
        progressMsg.close();
        progressMsg = ElMessage({
          message: `导入进度: ${results.length}/${items.length}`,
          duration: 0,
          icon: Loading,
        });
      }
    }

    progressMsg.close();
    showBatchImportDialog.value = false;
    batchImportDialogRef.value?.resetImporting();

    const succeeded = results.filter(r => r.success).length;
    const failed = results.filter(r => !r.success);
    if (succeeded > 0) {
      let msg = `成功通过 Session Token 导入 ${succeeded} 个 Devin 账号`;
      if (failed.length > 0) msg += `，失败 ${failed.length} 个`;
      ElMessage.success({ message: msg, duration: 5000, showClose: true });
      await accountsStore.loadAccounts();
    } else {
      const details = failed.slice(0, 3).map(f => `${f.email}（${f.error || '未知'}）`).join('\n');
      ElMessage.error({
        message: `没有成功导入任何账号\n${details}${failed.length > 3 ? '\n...' : ''}`,
        duration: 5000,
        showClose: true,
      });
    }
  } catch (e) {
    progressMsg.close();
    showBatchImportDialog.value = false;
    batchImportDialogRef.value?.resetImporting();
    ElMessage.error(`批量导入失败: ${e}`);
  }
}

// 批量刷新状态（使用优化的批量 API，只保存一次）
async function handleBatchRefresh() {
  const selectedIds = Array.from(accountsStore.selectedAccounts);
  if (selectedIds.length === 0) {
    ElMessage.warning('请先选择账号');
    return;
  }
  
  const totalCount = selectedIds.length;
  
  const progressLoading = ElMessage({
    message: `正在批量刷新 ${totalCount} 个账号状态...`,
    duration: 0,
    icon: Loading
  });
  
  try {
    // 使用优化的批量刷新 API（后端只保存一次）
    const result = await apiService.batchRefreshTokens(selectedIds);
    
    progressLoading.close();
    
    const successCount = result.success_count || 0;
    const failedCount = totalCount - successCount;
    
    // v1.7.8：后端已将刷新结果写入 SQLite，前端刷新当前页即可
    await Promise.all([accountsStore.fetchPage(), accountsStore.refreshAggregates()]);
    
    // 显示结果
    if (failedCount === 0) {
      ElMessage.success(`刷新完成: 成功 ${successCount} 个`);
    } else {
      // 收集失败信息
      const failedItems = result.results?.filter((r: any) => !r.success) || [];
      const failedEmails = failedItems.slice(0, 3).map((item: any) => {
        return `${item.id}: ${item.error || '未知错误'}`;
      });
      
      const moreCount = failedItems.length - 3;
      let message = `刷新完成（部分失败）\n成功: ${successCount}/${totalCount}\n失败: ${failedCount}/${totalCount}`;
      if (failedEmails.length > 0) {
        message += `\n\n失败账号:\n• ${failedEmails.join('\n• ')}`;
        if (moreCount > 0) {
          message += `\n... 还有 ${moreCount} 个失败`;
        }
      }
      
      ElMessageBox.alert(message, '刷新结果', {
        type: 'warning',
        confirmButtonText: '确定'
      });
    }
    
    accountsStore.clearSelection();
  } catch (error) {
    progressLoading.close();
    ElMessage.error(`批量刷新失败: ${error}`);
  }
}

async function handleExportAccounts(selectedOnly: boolean = false) {
  try {
    let accounts: Account[];
    if (selectedOnly) {
      if (accountsStore.selectedAccounts.size === 0) {
        ElMessage.warning('请先选择要导出的账号');
        return;
      }
      // 全选时直接用 getAllAccounts，避免大量 ID 的分批查询开销
      if (accountsStore.selectedAccounts.size >= accountsStore.aggregates.total_count) {
        accounts = await accountApi.getAllAccounts();
      } else {
        accounts = await accountApi.getAccountsByIds(Array.from(accountsStore.selectedAccounts));
      }
      if (accounts.length === 0) {
        ElMessage.warning('没有选中的账号');
        return;
      }
    } else {
      // v1.7.8：导出全部账号需从后端获取全量（accounts 只存当前页）
      accounts = await accountApi.getAllAccounts();
      if (accounts.length === 0) {
        ElMessage.warning('没有可导出的账号');
        return;
      }
    }
    batchExportAccounts.value = accounts;
    showBatchExportDialog.value = true;
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`导出失败: ${error}`);
    }
  }
}

async function showAddGroupDialog() {
  try {
    const { value } = await ElMessageBox.prompt('请输入分组名称', '添加分组', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputPattern: /^.{1,20}$/,
      inputErrorMessage: '分组名称长度应为1-20个字符'
    });
    
    await settingsStore.addGroup(value);
    ElMessage.success('分组添加成功');
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`添加分组失败: ${error}`);
    }
  }
}

async function showRenameGroupDialog(oldName: string) {
  try {
    const { value } = await ElMessageBox.prompt('请输入新的分组名称', `重命名分组 "${oldName}"`, {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputPattern: /^.{1,20}$/,
      inputErrorMessage: '分组名称长度应为1-20个字符',
      inputValue: oldName
    });
    
    if (value === oldName) {
      return;
    }
    
    await settingsStore.renameGroup(oldName, value);
    ElMessage.success('分组重命名成功');
    
    // 刷新账号列表
    await accountsStore.loadAccounts();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`重命名分组失败: ${error}`);
    }
  }
}

async function showDeleteGroupConfirm(name: string) {
  try {
    await ElMessageBox.confirm(
      `确定要删除分组 "${name}" 吗？该分组下的账号将被移至"未分组"`,
      '删除分组',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );
    
    await settingsStore.deleteGroup(name);
    ElMessage.success('分组删除成功');
    
    // 刷新账号列表
    await accountsStore.loadAccounts();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`删除分组失败: ${error}`);
    }
  }
}

// 获取当前活跃 Windsurf / Windsurf - Next 客户端的账号信息
// 后端按「进程优先 + state.vscdb mtime fallback」自动选择活跃客户端
async function fetchCurrentWindsurfInfo() {
  try {
    const info = await settingsApi.getCurrentWindsurfInfo();
    if (info.is_active && info.email) {
      currentWindsurfEmail.value = info.email;
    }
    if (info.version) {
      windsurfVersion.value = info.version;
    }
    // 活跃客户端展示名（驱动关于对话框标题动态切换）
    if (info.client_display_name) {
      windsurfClientDisplayName.value = info.client_display_name;
    }
  } catch (error) {
    console.error('获取当前Windsurf信息失败:', error);
  }
}

// 打开批量更换订阅对话框（v1.7.8：从后端加载选中账号完整数据）
async function openBatchUpdatePlanDialog() {
  batchPlanAccounts.value = await accountApi.getAccountsByIds(Array.from(accountsStore.selectedAccounts));
  showBatchUpdatePlanDialog.value = true;
}

// 关闭批量分组对话框
function closeBatchGroupDialog() {
  showBatchGroupDialog.value = false;
  batchGroupTarget.value = '';
}

// 批量更改分组（v1.7.8：后端批量 API，支持跨页选中）
async function handleBatchUpdateGroup() {
  const selectedIds = Array.from(accountsStore.selectedAccounts);
  if (selectedIds.length === 0) {
    ElMessage.warning('请先选择账号');
    return;
  }
  
  if (!batchGroupTarget.value) {
    ElMessage.warning('请选择目标分组');
    return;
  }
  
  isBatchUpdatingGroup.value = true;
  
  try {
    const affected = await accountApi.batchUpdateGroupByIds(selectedIds, batchGroupTarget.value);
    ElMessage.success(`成功将 ${affected} 个账号移动到"${batchGroupTarget.value}"分组`);
    
    // 关闭对话框并刷新
    closeBatchGroupDialog();
    accountsStore.clearSelection();
    await Promise.all([accountsStore.fetchPage(), accountsStore.refreshAggregates()]);
  } catch (error) {
    ElMessage.error(`批量更改分组失败: ${error}`);
  } finally {
    isBatchUpdatingGroup.value = false;
  }
}

// 显示关于对话框
function showAboutDialog() {
  showAbout.value = true;
}

// 初始化时获取当前账号信息和应用版本
onMounted(async () => {
  fetchCurrentWindsurfInfo();
  
  // 获取应用版本号
  try {
    const versionInfo = await invoke<any>('get_app_version');
    appVersion.value = versionInfo.version;
  } catch (error) {
    console.error('Failed to get app version:', error);
  }
  
  // 初始化排序配置
  initSortConfig();
  
  // 初始化自动重置定时器
  initAutoResetTimers();

  // 初始化额度耗尽自动切换（Fork 新增）
  initAutoSwitchTimer();
  setupAutoSwitchListener();

  // 启动静默检测更新（延迟 3 秒，避开启动高峰；store 内部 24h 防抖 + 跳过版本）
  window.setTimeout(async () => {
    try {
      const hasUpdate = await updaterStore.checkUpdate(true);
      if (hasUpdate) {
        showUpdateDialog.value = true;
      }
    } catch (e) {
      console.warn('[Updater] silent check failed:', e);
    }
  }, 3000);
});

// 监听设置变化，自动切换的开关/间隔变化时即时重启定时器（Fork 新增）
watch(
  () => {
    const s = settingsStore.settings as any;
    return {
      enabled: s?.autoSwitchEnabled ?? false,
      interval: s?.autoSwitchCheckInterval ?? 5,
    };
  },
  () => {
    initAutoSwitchTimer();
  },
  { deep: false }
);

// 组件卸载时清除自动重置定时器
onUnmounted(() => {
  clearSidebarMenuTimer();
  clearAccountRenderFrame();
  autoResetTimerMap.value.forEach(timer => clearInterval(timer));
  autoResetTimerMap.value.clear();
  // 清理自动切换定时器与监听器（Fork 新增）
  clearAutoSwitchTimer();
  if (autoSwitchUnlisten) {
    autoSwitchUnlisten();
    autoSwitchUnlisten = null;
  }
});
</script>

<style scoped>
.main-container {
  height: 100vh;
  width: 100vw;
}

.sidebar {
  background: #fff;
  border-right: 1px solid #e4e7ed;
  display: flex;
  flex-direction: column;
  transition: width 0.22s cubic-bezier(0.2, 0, 0.2, 1);
  overflow: hidden;
  contain: layout paint style;
  isolation: isolate;
  will-change: width;
  transform: translateZ(0);
}

/* 全局隐藏侧边栏的所有滚动条 */
.el-aside {
  overflow: hidden !important;
}

.sidebar::-webkit-scrollbar,
.sidebar :deep(::-webkit-scrollbar),
.el-aside::-webkit-scrollbar,
.el-menu::-webkit-scrollbar {
  display: none !important;
  width: 0 !important;
  background: transparent !important;
}

.sidebar,
.sidebar :deep(*),
.el-aside,
.el-menu {
  -ms-overflow-style: none !important;  /* IE and Edge */
  scrollbar-width: none !important;  /* Firefox */
  overflow-x: hidden !important;
}

.app-title {
  padding: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  font-size: 18px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  min-height: 64px;
  position: relative;
  transition: padding 0.22s cubic-bezier(0.2, 0, 0.2, 1);
  
  .el-icon {
    flex-shrink: 0;
  }
  
  .app-title-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    
    .version-text {
      font-size: 12px;
      font-weight: normal;
      color: var(--el-text-color-secondary);
      opacity: 0.8;
    }
  }
}

/* 收缩状态下标题样式 */
.sidebar.el-aside--collapse .app-title {
  padding: 16px 8px;
}

.sidebar.is-collapsed .app-title {
  padding: 16px 8px;
  justify-content: center;
}

.app-title-text {
  position: absolute;
  top: 50%;
  left: 48px;
  right: 12px;
  opacity: 1;
  transform: translate3d(0, -50%, 0);
  transition: opacity 0.14s ease, transform 0.18s cubic-bezier(0.2, 0, 0.2, 1);
  will-change: opacity, transform;
  pointer-events: none;
}

.sidebar.is-collapsed .app-title-text {
  opacity: 0;
  transform: translate3d(-8px, -50%, 0);
}

.sidebar-menu {
  flex: 1;
  border-right: none;
  overflow: hidden !important;
  contain: layout paint;
}

/* 隐藏Element Plus菜单的滚动条 */
.sidebar-menu::-webkit-scrollbar {
  display: none;
}

.sidebar-menu {
  -ms-overflow-style: none;  /* IE and Edge */
  scrollbar-width: none;  /* Firefox */
}

/* "自动切换"菜单项右侧的 ON 状态徽标 */
.auto-switch-badge {
  margin-left: 8px;
  transform: translateY(-2px);
}
.auto-switch-badge :deep(.el-badge__content) {
  font-size: 10px;
  padding: 0 5px;
  height: 16px;
  line-height: 16px;
}

.sidebar-footer {
  padding: 12px;
  text-align: center;
  border-top: 1px solid #e4e7ed;
  background: #fff;
  position: relative;
  z-index: 1;
}

.header {
  background: linear-gradient(to bottom, #ffffff 0%, #fafbfc 100%);
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: nowrap;
  gap: 8px;
  padding: 0 20px;
  height: 64px;
  min-height: 64px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.03);
  position: relative;
  z-index: 10;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 0 1 auto;
  min-width: 0;
}

.search-input {
  width: 220px;
}

.sort-select {
  width: 105px;
}

.sort-select :deep(.el-input__wrapper) {
  border-radius: 8px;
  height: 32px;
}

.search-input :deep(.el-input__wrapper) {
  border-radius: 20px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  border: 1px solid rgba(0, 0, 0, 0.05);
  transition: all 0.3s ease;
  padding: 0 16px;
  height: 40px;
  background: #ffffff;
}

.search-input :deep(.el-input__wrapper:hover) {
  box-shadow: 0 3px 12px rgba(0, 0, 0, 0.1);
  border-color: var(--el-color-primary-light-5);
}

.search-input :deep(.el-input__wrapper.is-focus) {
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.15);
  border-color: var(--el-color-primary);
}

.search-input :deep(.el-input__inner) {
  font-size: 14px;
  color: #1e293b;
}

.search-input :deep(.el-input__prefix) {
  font-size: 16px;
  color: #94a3b8;
}

.header-right {
  display: flex;
  gap: 2px;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: nowrap;
  flex: 1 1 360px;
  min-width: 0;
}

.header.has-selected-actions {
  gap: 6px;
  padding: 0 14px;
}

.header.has-selected-actions .search-input {
  width: 180px;
}

.header.has-selected-actions .sort-select {
  width: 96px;
}

.header.has-selected-actions .header-right {
  gap: clamp(6px, 0.8vw, 12px);
}

.header.has-selected-actions .header-right :deep(.el-button.is-circle),
.header.has-selected-actions .header-right :deep(.theme-trigger) {
  width: 34px;
  height: 34px;
  min-width: 34px;
}

.header.has-selected-actions .header-right :deep(.el-button .el-icon),
.header.has-selected-actions .header-right :deep(.theme-trigger .el-icon) {
  font-size: 16px;
}

.header-right :deep(.el-button.is-circle),
.header-right :deep(.theme-trigger) {
  margin: 0;
}

/* 圆形按钮徒加徽章样式 */
.header-right :deep(.el-badge) {
  vertical-align: middle;
}

.header-right :deep(.el-badge__content) {
  padding: 0 4px;
  height: 16px;
  line-height: 16px;
  font-size: 10px;
  border-radius: 8px;
  background-color: #f56c6c;
  border: none;
}


/* 全选按钮分隔线 */
.header-right .select-all-button {
  margin-left: 4px;
  position: relative;
}

.header-right .select-all-button::before {
  content: '';
  position: absolute;
  left: -6px;
  top: 50%;
  transform: translateY(-50%);
  height: 24px;
  width: 1px;
  background-color: #dcdfe6;
  pointer-events: none;
  z-index: 1;
}

/* 批量操作按钮 - 已通过父元素统一设置间距 */

/* 主要操作按钮样式 */
.header-right :deep(.el-button--primary:not(.is-circle)) {
  background: linear-gradient(135deg, #409eff 0%, #3b8cef 100%);
  border: none;
  border-radius: 20px;
  padding: 10px 20px;
  font-weight: 600;
  box-shadow: 0 3px 10px rgba(64, 158, 255, 0.25);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 圆形主要按钮 - 蓝色 */
.header-right :deep(.el-button--primary.is-circle) {
  background: linear-gradient(135deg, #409eff 0%, #3b8cef 100%);
  border: none;
  box-shadow: 0 2px 8px rgba(64, 158, 255, 0.3);
  transition: all 0.3s ease;
  color: #ffffff;
}

.header-right :deep(.el-button--primary.is-circle:hover) {
  background: linear-gradient(135deg, #3b8cef 0%, #2d7ee5 100%);
  transform: scale(1.1) rotate(10deg);
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.4);
}

.header-right :deep(.el-button--primary:not(.is-circle):hover) {
  background: linear-gradient(135deg, #3b8cef 0%, #2d7ee5 100%);
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(64, 158, 255, 0.35);
}

.header-right :deep(.el-button--primary:active) {
  transform: translateY(0);
  box-shadow: 0 2px 6px rgba(64, 158, 255, 0.2);
}

/* 危险按钮 - 红色（批量删除） */
.header-right :deep(.el-button--danger.is-circle) {
  background: linear-gradient(135deg, #f56c6c 0%, #f04848 100%);
  border: none;
  box-shadow: 0 2px 8px rgba(245, 108, 108, 0.3);
  transition: all 0.3s ease;
  color: #ffffff;
}

.header-right :deep(.el-button--danger.is-circle:hover) {
  background: linear-gradient(135deg, #f04848 0%, #e63535 100%);
  transform: scale(1.1) rotate(10deg);
  box-shadow: 0 4px 12px rgba(245, 108, 108, 0.4);
}

.header-right :deep(.el-button--danger:active) {
  transform: translateY(0);
  box-shadow: 0 2px 6px rgba(245, 108, 108, 0.2);
}

/* 成功按钮 - 绿色（批量转让订阅） */
.header-right :deep(.el-button--success.is-circle) {
  background: linear-gradient(135deg, #67c23a 0%, #5daf34 100%);
  border: none;
  box-shadow: 0 2px 8px rgba(103, 194, 58, 0.3);
  transition: all 0.3s ease;
  color: #ffffff;
}

.header-right :deep(.el-button--success.is-circle:hover) {
  background: linear-gradient(135deg, #5daf34 0%, #529b2e 100%);
  transform: scale(1.1) rotate(10deg);
  box-shadow: 0 4px 12px rgba(103, 194, 58, 0.4);
}

/* 警告按钮 - 橙色（批量刷新状态） */
.header-right :deep(.el-button--warning.is-circle) {
  background: linear-gradient(135deg, #e6a23c 0%, #d48a1f 100%);
  border: none;
  box-shadow: 0 2px 8px rgba(230, 162, 60, 0.3);
  transition: all 0.3s ease;
  color: #ffffff;
}

.header-right :deep(.el-button--warning.is-circle:hover) {
  background: linear-gradient(135deg, #d48a1f 0%, #c27c0e 100%);
  transform: scale(1.1) rotate(10deg);
  box-shadow: 0 4px 12px rgba(230, 162, 60, 0.4);
}

.header-right :deep(.el-button--success:active) {
  transform: translateY(0);
  box-shadow: 0 2px 6px rgba(103, 194, 58, 0.2);
}

/* 普通按钮样式 */
.header-right :deep(.el-button--default) {
  background: linear-gradient(135deg, #f5f7fa 0%, #e9ecef 100%);
  border: 1px solid #dcdfe6;
  border-radius: 20px;
  padding: 10px 20px;
  font-weight: 600;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  color: #606266;
}

.header-right :deep(.el-button--default:hover) {
  background: linear-gradient(135deg, #ecf5ff 0%, #d9ecff 100%);
  border-color: #409eff;
  transform: translateY(-1px);
  box-shadow: 0 4px 10px rgba(64, 158, 255, 0.15);
  color: #409eff;
}

.header-right :deep(.el-button--default:active) {
  transform: translateY(0);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

/* 圆形按钮基础样式 */
.header-right :deep(.el-button.is-circle) {
  transition: all 0.3s ease;
  width: 40px;
  height: 40px;
  flex: 0 0 auto;
}

/* 默认圆形按钮 - 统一的灰色风格 */
.header-right :deep(.el-button--default.is-circle) {
  color: #606266 !important;  
  background: #ffffff;
  border: 1px solid #dcdfe6;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.header-right :deep(.el-button--default.is-circle:hover) {
  color: #409eff !important;
  background: #ecf5ff;
  border-color: #c6e2ff;
  transform: scale(1.1);
  box-shadow: 0 2px 8px rgba(64, 158, 255, 0.2);
}

/* 特别处理圆形按钮的图标颜色 */
.header-right :deep(.el-button.is-circle .el-icon) {
  color: inherit !important;
  font-weight: 500;
}

.header-right :deep(.el-button.is-circle:hover) {
  transform: scale(1.1);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.header-right :deep(.el-button.is-circle:active) {
  transform: rotate(90deg) scale(0.95);
}

/* 按钮图标样式 */
.header-right :deep(.el-button .el-icon) {
  font-size: 18px;
  transition: all 0.3s ease;
}

/* 默认按钮的图标颜色 */
.header-right :deep(.el-button--default .el-icon) {
  color: #606266 !important;
}

.header-right :deep(.el-button--default:hover .el-icon) {
  color: #409eff !important;
}

/* 全选按钮 - 选中时特殊处理 */
.header-right :deep(.el-button--primary.is-circle[class*="el-button--primary"]:first-child) {
  background: #409eff;
  border: none;
  color: #ffffff;
  box-shadow: 0 2px 6px rgba(64, 158, 255, 0.3);
}

/* 动态出现的按钮动画 */
.header-right :deep(.el-button--danger.is-circle),
.header-right :deep(.el-button--success.is-circle),
.header-right :deep(.el-button--warning.is-circle) {
  animation: slideInFromLeft 0.3s ease;
}

@keyframes slideInFromLeft {
  from {
    opacity: 0;
    transform: translateX(-20px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}


/* 按钮内文字样式 */
.header-right :deep(.el-button span) {
  font-size: 14px;
  letter-spacing: 0.3px;
}

/* 按钮加载状态 */
.header-right :deep(.el-button.is-loading) {
  opacity: 0.8;
}

/* 导出对话框样式 */
.export-dialog {
  .el-message-box__message {
    padding: 0 !important;
  }
  
  input[type="radio"] {
    accent-color: #409eff;
    width: 16px;
    height: 16px;
    vertical-align: middle;
  }
  
  label:hover {
    background-color: #f5f7fa;
    border-radius: 6px;
    padding: 8px 12px;
    margin-left: -12px;
    margin-right: -12px;
  }
}

/* 按钮禁用状态 */
.header-right :deep(.el-button.is-disabled) {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none !important;
}

/* 筛选面板样式 */
.filter-panel {
  background: #ffffff;
  border-radius: 10px;
  padding: 12px 16px;
  margin-bottom: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  border: 1px solid rgba(0, 0, 0, 0.04);
}

.filter-panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.filter-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}

.filter-header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.filter-panel-body {
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.filter-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.filter-row-select {
  gap: 20px;
}

.filter-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.filter-item-range {
  flex: none;
  min-width: auto;
}

.filter-item-select {
  flex-shrink: 0;
}

.filter-item-select .el-select {
  width: 120px;
}

.filter-label {
  font-size: 13px;
  color: #606266;
  font-weight: 500;
  white-space: nowrap;
}

.filter-range {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
}

.filter-range .el-input-number {
  width: 72px;
}

.range-separator {
  color: #c0c4cc;
  font-size: 12px;
}

/* 筛选面板动画 */
.filter-slide-enter-active,
.filter-slide-leave-active {
  transition: all 0.3s ease;
}

.filter-slide-enter-from,
.filter-slide-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

/* 分页容器 */
.pagination-container {
  display: flex;
  justify-content: center;
  padding: 20px 0;
  margin-top: 16px;
  background: transparent;
}

.pagination-container .el-pagination {
  background: #ffffff;
  padding: 12px 20px;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}

/* 筛选按钮样式 */
.filter-toggle-btn {
  margin-left: 12px;
}

.main-content {
  background: #f5f7fa;
  padding: 8px 6px;
}

.loading-container,
.empty-container {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.accounts-container {
  width: 100%;
  position: relative;
}

.accounts-container.is-refreshing .accounts-grid {
  opacity: 0.72;
  pointer-events: none;
}

.accounts-loading-overlay {
  position: absolute;
  top: 10px;
  right: 12px;
  z-index: 5;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 999px;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  box-shadow: 0 6px 16px rgba(15, 23, 42, 0.1);
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.accounts-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(clamp(272px, 28vw, 360px), 360px));
  gap: 16px;
  padding: 0 0 20px;
  justify-content: start;
  align-items: start;
}

.account-rendering-hint {
  display: flex;
  justify-content: center;
  padding: 10px 0 4px;
  color: #909399;
  font-size: 12px;
}

/* 响应式布局 */
@media (max-width: 1400px) {
  .accounts-grid {
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 300px), 360px));
  }
}

@media (max-width: 1024px) {
  .header {
    gap: 6px;
    padding: 0 12px;
  }

  .header.has-selected-actions {
    gap: 4px;
    padding: 0 8px;
  }

  .header.has-selected-actions .header-left {
    gap: 4px;
  }

  .header.has-selected-actions .search-input {
    width: 140px;
  }

  .header.has-selected-actions .sort-select {
    width: 82px;
  }

  .header.has-selected-actions .header-right :deep(.el-button.is-circle),
  .header.has-selected-actions .header-right :deep(.theme-trigger) {
    width: 30px;
    height: 30px;
    min-width: 30px;
  }

  .main-content {
    padding: 10px 6px;
  }
  
  .accounts-grid {
    gap: 8px;
  }
}

@media (max-width: 768px) {
  .accounts-grid {
    grid-template-columns: 1fr;
  }
  
  .header {
    padding: 0 6px;
    gap: 4px;
  }

  .header.has-selected-actions .search-input {
    width: 120px;
  }

  .header.has-selected-actions .sort-select {
    width: 74px;
  }

  .header.has-selected-actions .header-right :deep(.el-button.is-circle),
  .header.has-selected-actions .header-right :deep(.theme-trigger) {
    width: 28px;
    height: 28px;
    min-width: 28px;
  }

  .header.has-selected-actions .header-right :deep(.el-button .el-icon),
  .header.has-selected-actions .header-right :deep(.theme-trigger .el-icon) {
    font-size: 14px;
  }
  
  .main-content {
    padding: 8px 4px;
  }
}

/* 暗色主题支持 */
:root.dark .sidebar {
  background: #1e1e1e;
  border-color: rgba(255, 255, 255, 0.08);
}

:root.dark .sidebar-menu {
  background: #1e1e1e;
}

:root.dark .sidebar-menu .el-menu-item {
  background: transparent;
  color: #cfd3dc;
}

:root.dark .sidebar-menu .el-menu-item:hover {
  background-color: rgba(64, 158, 255, 0.05);
}

/* 分组项样式 */
.group-item {
  position: relative;
}

.group-item-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.group-name {
  flex: 1;
  cursor: pointer;
}

.group-count {
  font-size: 12px;
  color: #909399;
  font-weight: normal;
}

.group-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.group-action-icon {
  cursor: pointer;
  padding: 4px;
  transition: all 0.3s;
  color: #606266;
}

.group-action-icon:hover {
  color: #409eff;
  background-color: rgba(64, 158, 255, 0.1);
  border-radius: 4px;
}

.group-action-icon.delete:hover {
  color: #f56c6c;
  background-color: rgba(245, 108, 108, 0.1);
}

/* 暗色主题适配 */
:root.dark .sidebar-menu .el-menu-item.is-active {
  background-color: rgba(64, 158, 255, 0.1);
  color: #409eff;
}

:root.dark .sidebar-footer {
  background: #1e1e1e;
  border-top-color: rgba(255, 255, 255, 0.08);
}

:root.dark .sidebar-footer .el-button.is-circle {
  background-color: #262729;
  border-color: #4c4d4f;
  color: #cfd3dc;
}

:root.dark .sidebar-footer .el-button.is-circle:hover {
  background-color: #303133;
  border-color: #5a5b5d;
  color: #409eff;
}

:root.dark .sidebar-footer .el-button.is-circle:active {
  background-color: #1a1a1c;
  border-color: #409eff;
}

:root.dark .header {
  background: linear-gradient(to bottom, #1e1e1e 0%, #1a1a1a 100%);
  border-bottom-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

:root.dark .search-input :deep(.el-input__wrapper) {
  background: #2a2a2a;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

:root.dark .search-input :deep(.el-input__wrapper:hover) {
  border-color: var(--el-color-primary-light-3);
  box-shadow: 0 3px 12px rgba(0, 0, 0, 0.3);
}

:root.dark .search-input :deep(.el-input__wrapper.is-focus) {
  border-color: var(--el-color-primary);
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.25);
}

:root.dark .search-input :deep(.el-input__inner) {
  color: #e4e4e7;
}

:root.dark .search-input :deep(.el-input__prefix) {
  color: #94a3b8;
}

:root.dark .header-right :deep(.el-button--primary) {
  background: linear-gradient(135deg, #409eff 0%, #3b8cef 100%);
  box-shadow: 0 3px 10px rgba(64, 158, 255, 0.3);
}

:root.dark .header-right :deep(.el-button--primary:hover) {
  box-shadow: 0 6px 16px rgba(64, 158, 255, 0.4);
}

:root.dark .header-right :deep(.el-button--danger) {
  background: linear-gradient(135deg, #f56c6c 0%, #f04848 100%);
  box-shadow: 0 3px 10px rgba(245, 108, 108, 0.3);
}

:root.dark .header-right :deep(.el-button--danger:hover) {
  box-shadow: 0 6px 16px rgba(245, 108, 108, 0.4);
}

:root.dark .header-right :deep(.el-button--success) {
  background: linear-gradient(135deg, #67c23a 0%, #5daf34 100%);
  box-shadow: 0 3px 10px rgba(103, 194, 58, 0.3);
}

:root.dark .header-right :deep(.el-button--success:hover) {
  box-shadow: 0 6px 16px rgba(103, 194, 58, 0.4);
}

:root.dark .header-right :deep(.el-button--default) {
  background: linear-gradient(135deg, #2a2a2a 0%, #252525 100%);
  border-color: rgba(255, 255, 255, 0.1);
  color: #e4e4e7;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
}

:root.dark .header-right :deep(.el-button--default:hover) {
  background: linear-gradient(135deg, #303030 0%, #2a2a2a 100%);
  border-color: rgba(255, 255, 255, 0.15);
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.4);
}

:root.dark .header-right :deep(.el-button.is-circle) {
  background: linear-gradient(135deg, #2a2a2a 0%, #252525 100%);
  border-color: rgba(255, 255, 255, 0.1);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
}

:root.dark .header-right :deep(.el-button.is-circle:hover) {
  background: linear-gradient(135deg, #1e3a5f 0%, #1a3454 100%);
  border-color: var(--el-color-primary-light-3);
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.3);
}

:root.dark .main-content {
  background: #121212;
}

/* 深色模式筛选面板 */
:root.dark .filter-panel {
  background: #1e1e1e;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

:root.dark .filter-title {
  color: #e5eaf3;
}

:root.dark .filter-label {
  color: #a0aec0;
}

:root.dark .range-separator {
  color: #4a5568;
}

/* 深色模式分页 */
:root.dark .pagination-container .el-pagination {
  background: #1e1e1e;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

/* 批量分组对话框样式 */
.batch-group-content {
  padding: 10px 0;
}

.batch-group-hint {
  margin-bottom: 16px;
  color: #606266;
  font-size: 14px;
}

.batch-group-hint strong {
  color: #409eff;
  font-weight: 600;
}

:root.dark .batch-group-hint {
  color: #a0aec0;
}

/* 批量转让对话框样式 */
.email-count-hint {
  font-size: 14px;
  color: #606266;
}

.batch-transfer-progress {
  margin-top: 20px;
  padding: 16px;
  background: #f8fafc;
  border-radius: 8px;
}

.batch-transfer-progress .progress-status {
  margin-top: 12px;
  text-align: center;
  color: #64748b;
  font-size: 14px;
}

:root.dark .batch-transfer-progress {
  background: #2d3748;
}

:root.dark .batch-transfer-progress .progress-status {
  color: #a0aec0;
}

/* ==================== 分组管理子菜单：超出限高后局部滚动 ====================
 * 背景：全局 `.sidebar :deep(::-webkit-scrollbar) { display: none !important; }` 强制隐藏
 *   了侧边栏所有滚动条。当分组数量较多（如 > 8 个）时，el-sub-menu 展开
 *   区会把下方其它菜单项顶出视口，用户无法访问亦无法滚动。
 *
 * 方案：只在 .groups-submenu 的展开容器（:deep(.el-menu)）上重新启用滚动：
 *   1. 限制 max-height（约 8 项高度），超出则出现滚动条；
 *   2. 用更高特异性 + !important 覆盖全局 scrollbar 隐藏规则，定制一条 6px 细滚动条；
 *   3. "添加分组"按钮用 position: sticky 钉在底部，滚动时始终可见。
 * 折叠态（collapse）下 sub-menu 是 popper 形式渲染在 <body> 下，不会命中本规则，无副作用。
 */

.sidebar-menu .groups-submenu :deep(.el-menu) {
  max-height: 360px;
  overflow-y: auto !important;
  overflow-x: hidden;
  scrollbar-width: thin !important;
  -ms-overflow-style: auto !important;
}

.sidebar-menu .groups-submenu :deep(.el-menu)::-webkit-scrollbar {
  display: block !important;
  width: 6px !important;
  background: transparent !important;
}

.sidebar-menu .groups-submenu :deep(.el-menu)::-webkit-scrollbar-thumb {
  background-color: var(--el-border-color) !important;
  border-radius: 3px !important;
}

.sidebar-menu .groups-submenu :deep(.el-menu)::-webkit-scrollbar-thumb:hover {
  background-color: var(--el-border-color-darker) !important;
}

/* “添加分组”固定在展开区底部：滚动分组列表时该按钮始终可见 */
.sidebar-menu .groups-submenu :deep(.group-add-action) {
  position: sticky;
  bottom: 0;
  z-index: 1;
  background-color: var(--el-menu-bg-color);
  border-top: 1px solid var(--el-border-color-lighter);
}

</style>

<!--
  折叠态 sub-menu 的 popper 被 Element Plus 通过 Teleport 渲染到 <body> 下，
  已脱离本组件的 DOM 子树；scoped style 的 `:deep()` 无法穿透 Teleport，
  因此折叠态的滚动样式必须放在下面这个“非 scoped”的 style 块里，
  通过 popper-class="groups-submenu-popper" 精准锁定，避免污染全局。
-->
<style>
/* ==================== 分组管理子菜单：折叠态 popper 浮层 ==================== */

/* popper 的 ul.el-menu 限高并启用滚动；容器相对定位是为了让 sticky 的“添加分组”生效 */
.groups-submenu-popper .el-menu {
  max-height: 70vh;
  overflow-y: auto !important;
  overflow-x: hidden;
  position: relative;
  scrollbar-width: thin !important;
  -ms-overflow-style: auto !important;
}

.groups-submenu-popper .el-menu::-webkit-scrollbar {
  display: block !important;
  width: 6px !important;
  background: transparent !important;
}

.groups-submenu-popper .el-menu::-webkit-scrollbar-thumb {
  background-color: var(--el-border-color) !important;
  border-radius: 3px !important;
}

.groups-submenu-popper .el-menu::-webkit-scrollbar-thumb:hover {
  background-color: var(--el-border-color-darker) !important;
}

/* “添加分组”按钮贴底：滚动分组列表时始终可见 */
.groups-submenu-popper .el-menu .group-add-action {
  position: sticky;
  bottom: 0;
  z-index: 1;
  background-color: var(--el-bg-color-overlay, var(--el-menu-bg-color));
  border-top: 1px solid var(--el-border-color-lighter);
}
</style>
