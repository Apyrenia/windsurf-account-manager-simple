# Windsurf Account Manager - Simple (Apyrenia Fork)

> 🍴 **This repository is a fork of [chaogei/windsurf-account-manager-simple](https://github.com/chaogei/windsurf-account-manager-simple)**, based on v1.7.11.
> Fully open source under AGPL-3.0. **Opposes any form of closed-source commercial distribution.**

A Windsurf multi-account management desktop application built with Tauri + Vue 3 + TypeScript, designed to manage multiple Windsurf accounts with features including one-click account switching and automatic quota-based switching.

> ⚠️ **Free Software Notice**: This software is completely free. If you paid for it, you have been scammed!

---

## 🆕 What This Fork Changes

### ✨ New Features

- **Automatic quota-based switching**: Polls the current account's quota periodically; when the usage threshold or remaining threshold is reached, automatically switches to the next available account. Fully compatible with both Windsurf's **new QUOTA billing model** (daily/weekly percentage) and **legacy CREDITS model**.
- **Sidebar shortcut**: "Auto Switch" menu entry with green ON badge when enabled.

### 🐛 Key Bug Fixes

- **Daily quota used but UI still shows 100%**: The new Windsurf API omits the `int_14` field for some accounts (e.g., Devin Trial after exhaustion). The original "update only if field exists" logic preserved stale full values forever. **Fix**: Treat missing fields as 0% (exhausted), both in backend `apply_plan_status_to_account` and frontend `handleRefreshToken`.
- **"Refresh Account Info" button does nothing, only "Re-login" works**: Frontend merge logic was out of sync with backend. **Fix**: Aligned frontend logic with backend.
- **`auto_switch` / `auto_reset` judgement completely broken on new-style accounts**: Original code only parsed legacy CREDITS fields. **Fix**: Extracted `WindsurfService::query_quota_summary` public method producing a standardized `QuotaSummary` compatible with both billing models.
- **1-minute polling interval triggers rate limit**: `GetPlanStatus` endpoint is rate-limited. **Fix**: Lower bound clamped to 3 minutes (UI + runtime).

### 🧹 UI Simplification

- **Account card buttons**: 18 → 6 (Refresh Token / Account Info / Delete / Edit / Re-login / Seamless Switch)
- **"Auto Reset" sidebar entry hidden** (kept in source for easy restore)

### 🔗 Upstream Relationship

Git remote setup: `origin` → this fork, `upstream` → chaogei. To sync upstream: `git fetch upstream && git merge upstream/main`.

---

## 📦 Project Information

- **Current Version**: 1.8.1
- **License**: AGPL-3.0
- **Development Language**: Rust + TypeScript
- **Supported Platforms**: Windows 10/11
- **Architecture**: Tauri 2.x Desktop Application

**🌐 中文版本**: [README.md](README.md)

## � Feedback & Discussion

This fork's maintainer does **not** run any WeChat / QQ group. For all issues please use GitHub channels:

- 🐛 **Bug reports / suggestions** → [GitHub Issues](https://github.com/Apyrenia/windsurf-account-manager-simple/issues)
- 💡 **Discussion / tips** → [GitHub Discussions](https://github.com/Apyrenia/windsurf-account-manager-simple/discussions)
- 📝 **Contributions** → Fork, submit PR; AGPL-3.0 inherited automatically

> The upstream chaogei repo has its own WeChat/QQ communities, **but this fork's author is not in them**. Please don't ask about this fork in chaogei's groups to avoid confusion for everyone.

---

## 🖥️ Software Interface

<p align="center">
  <img src="public/主页.png" alt="Main Interface" width="800">
</p>

<p align="center">
  <img src="public/账号信息.png" alt="Account Info" width="800">
</p>

<p align="center">
  <img src="public/设置.png" alt="Settings" width="800">
</p>

---

## ✨ Features

### 🔐 Account Management
- ✅ **Add/Edit/Delete Accounts** - Complete CRUD operations for accounts
- ✅ **Account Grouping** - Custom groups for managing multiple accounts
- ✅ **Tag System** - Add custom tags to accounts
- ✅ **Real-time Status Display** - Show plan type, credit balance, expiration time, etc.
- ✅ **Batch Operations** - Batch select, reset, delete accounts
- ✅ **Encrypted Storage** - AES-256-GCM encryption, keys stored in system keychain

### 💳 Credit Reset
- ✅ **One-click Credit Reset** - Reset credits via seat update API
- ✅ **Smart Seat Rotation** - Auto-rotate between 3/4/5 seats
- ✅ **Batch Reset** - Support multiple accounts simultaneously (max 5 concurrent)
- ✅ **Team Batch Reset** - One-click reset all team members' credits
- ✅ **Auto Reset Schedule** - Set scheduled tasks for automatic credit reset

### 👥 Team Management
- ✅ **View Team Members** - List all team members
- ✅ **Invite Members** - Invite new members via email
- ✅ **Remove Members** - Remove specified members from team
- ✅ **Team Credit Management** - Unified management of team member credits

### 🔄 One-click Account Switching
- ✅ **Quick Account Switch** - Fast switch to other Windsurf accounts
- ✅ **Auto Token Refresh** - Auto-refresh access_token using refresh_token
- ✅ **OAuth Callback** - Auto-complete login via windsurf:// protocol
- ✅ **Machine ID Reset** - Reset device identifier for multi-device use (requires admin privileges)

### 🔧 Seamless Switching Patch
- ✅ **Auto-detect Windsurf Path** - Automatically find Windsurf installation
- ✅ **One-click Patch** - Modify extension.js for seamless switching
- ✅ **Remove Timeout Limit** - Remove 180-second OAuth timeout
- ✅ **Auto Backup** - Auto-backup original file before patching (max 3 copies)
- ✅ **One-click Restore** - Restore original state from backup
- ✅ **Auto Restart Windsurf** - Auto-restart Windsurf after patch application

### 🛑 Cunzhi Feature
- ✅ **MCP Tool Integration** - Provide dialog-helper and confirm tools
- ✅ **Dialog Assistant** - Help users confirm operations and input information
- ✅ **Cross-platform Support** - Support Windows, Linux, macOS
- ✅ **Binary File Management** - Copy platform-specific files during build
- ✅ **Smart Packaging** - Avoid packaging all platform files, reduce installer size

### 💰 Payment Related
- ✅ **Virtual Card Generation** - Generate virtual credit card info for payment testing
- ✅ **Custom BIN** - Support custom BIN number or BIN range
- ✅ **Private Payment Window** - Open Stripe payment page in incognito browser window
- ✅ **Alipay/WeChat Payment** - Support domestic payment methods (donation)

### 📊 Data Query
- ✅ **Billing Query** - Query plan, quota, usage, etc.
- ✅ **Subscription Status** - Show subscription type, expiration, next billing date
- ✅ **Usage Statistics** - View credit usage and remaining quota
- ✅ **Global Refresh** - One-click refresh all account info

### ⚙️ System Settings
- ✅ **Proxy Configuration** - Support HTTP proxy settings
- ✅ **Lightweight API Mode** - Use GetPlanStatus instead of GetCurrentUser to reduce requests
- ✅ **Detailed Results** - Optional display of detailed API response
- ✅ **Operation Logs** - Record all operation history, support export

### 🔒 Data Security
- ✅ **System Keychain** - Use Windows Credential Manager to store encryption keys
- ✅ **AES-256-GCM Encryption** - All sensitive data encrypted
- ✅ **Local Storage** - Data stored locally only, no server upload
- ✅ **Operation Logs** - Complete operation records for auditing

### 🌐 Multi-client Support
- ✅ **Windsurf Standard** - Full support for official Windsurf client
- ✅ **Windsurf - Next** - Support Next version with independent configuration and management
- ✅ **Auto-detection** - Automatically identify and adapt to different client versions
- ✅ **Independent Configuration** - Different clients use independent config files and data storage

### 🤖 Devin Account System
- ✅ **auth1_token Authentication** - Support Devin's auth1_token authentication mechanism
- ✅ **session_token Management** - Auto-refresh session_token with 32-day placeholder mechanism
- ✅ **Multi-organization Support** - Support Devin multi-organization account management
- ✅ **Smart Refresh** - Force refresh triggered by 401 errors
- ✅ **Key Field Backfill** - Auto-supplement key fields like product parameter

---

## 📜 Version History

### v1.8.1 (2026-05-12) — Auto-Switch Selection Logic Improvement

**🐛 Fixed (user-reported)**

- **Auto-switch picks accounts with very low remaining quota (e.g. 4%)**: v1.8.0's "top-1 live verify + exclude-and-repick" loop still sorted candidates by DB cache values. If the pool had multiple stale-100% zombie accounts, they ranked first, got verified, found exhausted, were excluded — until finally the truly-low-but-real account got picked. **Fix**: before sorting, **batch-refresh live quota of all eligible candidates in parallel** and write back to DB. Sort then sees real data and naturally picks the truly-most-remaining account.

**🛡️ Safeguards**

- Refresh concurrency capped at **5** (avoid Windsurf API rate limit)
- Per-candidate query timeout **8 seconds** (one slow account can't stall the whole flow)
- Failed/timed-out accounts keep DB stale value; Step 6 single-candidate verification still runs as a safety net

**📦 Side-effect bonus**

- One auto-switch trigger = refresh live quota for the entire account pool (max 5 concurrent). UI immediately shows real remaining values for every candidate, **stale-100% zombie accounts all drop to 0%** — no more need to manually click "Refresh Account Info" one by one.

---

### v1.8.0 (2026-05-12) — Apyrenia Fork Initial Release

**🆕 Added**

- **Auto-switch on quota exhaustion** (core new feature): periodically polls current account quota; when usage threshold or remaining threshold is reached, switches to next available account. Fully compatible with both new QUOTA billing (daily/weekly percentage) and legacy CREDITS billing (points).
- **"Auto Switch" sidebar entry** with green ON badge when enabled.
- **`WindsurfService::query_quota_summary` public method**: standardized `QuotaSummary` output, all upper-layer logic uses one entry, agnostic to billing type.

**🐛 Fixed**

- **Daily quota exhausted but UI shows 100%**: Windsurf's new API omits `int_14` for some accounts (e.g., Devin Trial after exhaustion). Original "update only if field exists" logic preserved stale full values forever. **Fix**: backend `apply_plan_status_to_account` + frontend `handleRefreshToken` merge logic both treat missing fields as 0% (exhausted).
- **"Refresh Account Info" button does nothing, only "Re-login" works**: frontend merge logic was out of sync with backend. **Fix**: aligned frontend with backend.
- **`auto_switch` / `auto_reset` judgement broken on new-style accounts**: original code only parsed legacy CREDITS fields. **Fix**: replaced with `query_quota_summary`.
- **Auto-switch repeatedly switches to "DB shows 100% but actually exhausted" zombie accounts**: selection only read local DB cache. **Fix**: after picking top candidate, call `query_quota_summary` once to fetch live quota, write back to DB, verify still passes threshold; if not, exclude and re-pick (max 5 retries to prevent infinite loop).
- **1-minute polling interval triggers rate limit**: `GetPlanStatus` is rate-limited. **Fix**: lower bound clamped to 3 minutes (UI + runtime), with warning.

**🧹 UI Simplification**

- Account card buttons: 18 → 6 (Refresh Token / Account Info / Delete / Edit / Re-login / Seamless Switch).
- Hide "Auto Reset" sidebar entry (code preserved for easy restore).
- WelcomeDialog: WeChat group QR code replaced with GitHub Issues / Discussions guidance; this fork's author runs no community channels.

**📦 Metadata**

- Version 1.7.11 → **1.8.0** (per SemVer: new features = MINOR bump).
- README / README_EN top section adds fork explanation, explicitly opposing closed-source distribution.
- Git remote: `origin` = this fork, `upstream` = chaogei upstream, for future sync.

---

### v1.7.10 (2026-05-08)
- **Initial UI load performance optimization**: Added lazy rendering to heavy `el-tab-pane` sections in Settings, Team Management, Auto Reset, Account Info, and related dialogs, preventing all tables, forms, and information panels from mounting when the parent dialog first opens
- **On-demand mounting for nested dialogs**: Hidden child dialogs in Team Management, Team Settings, Account Info, Settings backups, Auto Reset editing, and related flows now use `v-if` on-demand mounting to reduce initial DOM size and hidden form initialization cost
- **Deferred chart library loading**: ECharts in `AnalyticsDialog` is now dynamically imported only when the analytics dialog initializes charts, keeping the charting library out of the account page initial loading path
- **Responsive account panel layout optimization**: Account card grids now adjust column count and minimum card width based on available space; card footer action buttons now fluidly scale button size, icon size, and gaps based on card width, while the top selected-account batch toolbar stays on a single row and fluidly expands or tightens button size, icon size, search/sort widths, and button gaps based on available space. This fixes right-side button clipping and uneven toolbar spacing in narrow-screen, high-column, or multi-selection layouts
- **Account card max-width fix**: Account grid columns now use fixed maximum widths with left alignment, fixing the issue where the only card on the last page or in filtered results could be stretched across the full row by `auto-fit + 1fr`
- **Default window width optimization**: Adjusted the default window width to fit three account-card columns with the sidebar collapsed, reducing the horizontal empty space caused by only showing two columns on first launch
- **Pagination switching performance optimization**: Switching to 50/100 items per page or changing pages now renders the first 20 account cards immediately and fills the rest in animation-frame batches, while paginated requests ignore stale responses and skip duplicate requests to reduce long stalls from mounting many complex cards at once
- **Themed blank-card fix**: Removed CSS `content-visibility` lazy-paint placeholders from themed account cards, fixing the issue where lower cards on 100-items-per-page views could show blank shells without their contents being painted promptly
- **Sidebar expansion performance optimization**: Sidebar expand/collapse now staggers the container width animation and Element Plus menu collapse state updates, keeps the title text mounted and switches it with opacity/translate transitions, and reduces themed sidebar backdrop blur and large-shadow repaint cost to minimize click-to-expand stutter
- **Default theme adjustment**: Fresh installs and missing-theme configurations now default to the native light theme `original-light`, and the legacy default `aurora` is migrated to `original-light` once; frontend theme defaults, settings defaults, and backend configuration defaults stay aligned
- **On-demand Team Management data loading**: Team Management now loads only the default members tab on open; invitations, my invitation, pending requests, and other tab data are requested only when their tabs become active, while avoiding duplicate member loading when reopening from a non-default tab
- **Account Info rendering optimization**: User details, Firebase, API Keys, Provider Key, and other information tabs now render lazily while preserving the existing logic that loads API Key / Provider Key data only when those tabs are selected
- **Build error fix**: Removed an invalid leftover `clientDisplayName` assignment from `SettingsDialog.vue`, fixing `vue-tsc` TS2304 / TS2339 build errors

### v1.7.8 (2026-05-04)
- **Windsurf new login protocol adaptation**: Windsurf backend updated `WindsurfPostAuth` endpoint to require `X-Devin-Auth1-Token` HTTP header. Added the header in `devin_auth_service.rs::windsurf_post_auth`; all Devin login/registration/multi-org selection flows automatically benefit
- **Cross-page selection batch operation optimization**: New `get_accounts_by_ids` backend API (SQLite `WHERE id IN (...)` precise query) replaces the old approach of `getAllAccounts()` loading 100K+ records then filtering on frontend. Export-selected, batch plan change, etc. improve from seconds to milliseconds
  - SQLite 32766 variable limit: `get_accounts_by_ids` and `update_group_by_ids` auto-batch queries (500 per batch), supporting arbitrary ID counts
  - Smart full-select export: when selected count ≥ total, falls back to single `getAllAccounts()` query to avoid unnecessary batching
- **Batch import real-time progress feedback**: Full-concurrent import mode now shows per-account completion count (`Import progress: 3/10（✓ xxx@email.com）`), replacing the previous static message that only updated after all completed; throttled UI refresh (500ms) after each successful account
- **Registration/import email dedup optimization**: All write paths (`add_account_by_refresh_token` / `add_account_by_devin_login` / `add_account_by_devin_with_org` / `persist_devin_account_from_login_result`, 6 locations total) changed email dedup from `get_all_accounts()` full-load iteration to SQLite `email_exists()` index query. Memory overhead drops from tens of MB to ~0; 20-concurrent registrations no longer spike memory
- **Pro trial eligibility check fix**: `check_pro_trial_eligibility` command changed from receiving frontend `authToken` to receiving `account_id`; backend reads full account from SQLite to construct correct `AuthContext` (auto-distinguishes Firebase/Devin). Fixes Devin accounts always falsely reporting "eligible". Retains protobuf body token + adds `with_auth` headers for dual authentication
- **Auto-open payment page auth fix**: `get_trial_payment_link_enhanced` command similarly changed to receive `account_id`; backend fetches token + constructs correct AuthContext. Fixes Devin account 401 errors when "auto-open payment page" is enabled
- **Seamless account switch patch synced with main project**: `switch_account_commands.rs` adds `OperationLog` import, `already_patched` check (avoids misleading re-application logs), operation log recording after successful switch; `patch_commands.rs` apply/revert messages now use dynamic `config.process_name` instead of hardcoded strings, `check_seamless_status` variable names aligned
- **Backup system overhaul**: Upgraded from single-file `accounts_*.json` to directory-based `backup_YYYYMMDD_HHMMSS/` containing `accounts.db` + all JSON files (6 total); `list_backups` / `restore_from_backup` / `delete_backup` / `cleanup` all switched to directory operations; added `calculate_dir_size` helper; Settings gains `auto_backup_enabled` / `backup_interval` / `backup_max_count` fields; `lib.rs` adds auto-backup task (1-minute startup delay, configurable interval loop)
- **Statistics page upgrade**: Backend `get_stats` adds 13 new fields (plan distribution, active/inactive subscription counts, team owners, disabled accounts, quota stats, tag/group/operation-type distributions, Token/RefreshToken counts); frontend `StatsDialog.vue` upgraded from 3 simple number cards to full 9-section panel (account overview, subscriptions & auth, special accounts, quota, plan/group/tag/operation-type distributions, system settings)
- **Silent account add mode**: `addAccount` / `appendLocalAccount` no longer trigger `fetchPage()` full-page refresh or `loading` state; only async aggregate stats refresh (sidebar counts). New accounts appear on the correct SQLite-sorted page when user navigates
- **TS build error fix**: Removed unused `isMasterAccount` function from `AutoResetDialog.vue`, fixing `vue-tsc` TS6133 strict check build failure
- **Theme-aware visual detail refinement**: Pagination no longer uses a fixed light-blue palette and now derives low-saturation colors from `--theme-primary`, covering the pagination container, page buttons, active-page highlight, and jump input; subscription/plan badges gain stronger contrast and visual hierarchy across themes while preserving semantic Free / Pro / Teams / Enterprise colors
- **Batch account export refactored into a standalone component**: Added `BatchExportDialog.vue` to replace the hardcoded `ElMessageBox + dangerouslyUseHTMLString` export dialog in `MainLayout.vue`, removing inline styles and aligning the dialog with the current theme system. The export entry now only resolves the target account set and opens the dialog
- **Batch export fields and format enhancement**: Supports freely combining email, password, Refresh Token, Access/Session Token, Windsurf API Key, Devin Auth1 Token, Devin Session Token, remark, group, tags, status, plan, auth provider, creation time, token expiry time, subscription expiry time, and more. Includes common presets, export preview, and both copy-to-clipboard and file-download targets
- **Batch export text delimiter enhancement**: TXT export supports space, Tab, `|`, `,`, `---`, `----`, and custom delimiters, with validation that custom delimiters cannot be empty. CSV/JSON are generated as standard formats; CSV can optionally include headers and handles cell escaping plus file BOM automatically

### v1.7.7 (2026-05-03)
- **Batch delete O(N) disk-write black hole fix (synced with main project)**: Users reported noticeable lag when batch-deleting multiple accounts, seemingly "lacking concurrent deletion support". The root cause turned out to be a **backend loop-serial × per-account full-file disk write** double IO bottleneck, not a concurrency issue: the original `account_commands::delete_accounts_batch` used `for id_str in ids { store.delete_account(uuid).await; }` to loop-call `DataStore::delete_account`, where each call went through `save()` + `save_logs()` — i.e. 2 full `atomic_write` cycles (write temp file → JSON validation → create `.backup` → atomic rename). 50 accounts = 100 atomic_write calls, taking seconds to tens of seconds under NTFS / real-time AV scan. **Going concurrent (`futures::join_all`) is NOT the correct fix** — all deletes still contend the same `config.write()` lock (pseudo-parallel, actually serial), and independent saves racing for file handles on Windows may even trigger SharingViolation. Architectural fix:
  - Added `DataStore::delete_accounts_batch(ids: &[Uuid]) -> AppResult<(Vec<Uuid>, Vec<Uuid>)>`: single `config.write()` lock + `HashSet::intersection` to identify actually-existing accounts + single `retain` batch removal + single `logs.write()` lock + single `retain` batch cleanup of related logs + one `save()` / `save_logs()` each. Returns `(deleted_ids, not_found_ids)`; empty input returns fast without touching disk
  - `commands::delete_accounts_batch` Tauri command now calls the new method: first splits ids by UUID validity into valid/invalid, batch-calls `DataStore::delete_accounts_batch` to get deleted + not_found, merges not_found into failed_ids. **Return structure `{success_count, failed_ids}` preserves original semantics, zero frontend changes**
  - **Performance metrics**: IO count drops from `O(2N)` to `O(2)`, lock acquisitions from `O(2N)` to `O(2)`. 50 accounts: 5~15 seconds → <100ms, estimated 50~150× improvement
  - **Implementation trade-off**: `retain` callback uses `HashSet` lookup instead of linear-scanning the input Vec, avoiding `O(N×M)` quadratic degradation on large batches (>100)
- **Batch import O(N) disk-write black hole fix + DataStore save coalescer architecture (synced with main project)**: Continuing the batch-delete theme, users reported batch-importing accounts also lags. Root cause turned out worse than delete: every import path follows **`store.add_account()` → backfill fields → `store.update_account()` double-save** pattern, so a single account triggers 2 `atomic_write`s. Frontend `Promise.all` looks concurrent but all saves contend the same `config.write()` lock (serialized in practice). 50 accounts = 100 serialized `atomic_write`s. Fix: introduce a "save coalescer" layer at the DataStore level:
  - **B.1 DataStore gains save coalescer infrastructure**: `data_store.rs` adds 4 new fields — `save_pending: Arc<AtomicBool>` / `save_logs_pending: Arc<AtomicBool>` / `save_coalesce_lock: Arc<tokio::sync::Mutex<()>>` / `save_logs_coalesce_lock: Arc<tokio::sync::Mutex<()>>`. Core API trio: ① `request_save_coalesced(self: &Arc<Self>)` non-blocking CAS merge, ② `request_save_logs_coalesced` symmetric variant for logs.json, ③ `flush_pending_saves()` blocking force-flush (exit hook only). Worker design guarantees N concurrent requests aggregate into at most 2 actual saves
  - **C.1 DataStore gains `add_account_no_save` variant**: Original `add_account` split into `add_account_no_save` (pure in-memory, no save), original method becomes `add_account_no_save + save` composition; `update_account_no_save` already existed (wraps `update_account_internal(account, false)`), reused as-is
  - **C.2 Simple-end 8 add_account_by_xxx paths refactored**: `account_commands::add_account_by_refresh_token` + `devin_commands` session_token / login / native_login / with_org / register / native_register / email_login / auth1_token. Direct add+update callers (login / with_org / session_token / auth1_token) change to `_no_save` + trailing `request_save_coalesced`; callers going through shared helper `persist_devin_account_from_login_result` (native_login / register / native_register / email_login) have the helper internally switch to `_no_save` (signature `&DataStore` can't hold an Arc), and each of the 4 callers appends `store.inner().request_save_coalesced()` after `.await?;`
  - **B.2 tauri `RunEvent::ExitRequested` flush hook**: `lib.rs` migrates from `.run(ctx)` to `.build(ctx).run(|handle, event| ...)`, with `tauri::async_runtime::block_on` running `store.flush_pending_saves()` on ExitRequested — force-sync-flush before exit to prevent batch modifications from being lost
  - **Performance metrics**: 50-account batch-import `atomic_write` count drops from **100~150 → 1~2**, 50~150× IO-layer improvement; actual wall time is network-RTT bound
  - **Files changed**: `src-tauri/src/repository/data_store.rs` (coalescer + add_account_no_save split), `src-tauri/src/commands/account_commands.rs`, `src-tauri/src/commands/devin_commands.rs` (8 commands + persist helper refactor), `src-tauri/src/lib.rs` (build + run callback + ExitRequested hook). Zero frontend changes
- **Batch credit reset migrated to save coalescer (synced with main project)**: After auditing all batch commands, one final gap was found — `batch_reset_credits` calls `reset_credits_internal` concurrently via `futures::stream::buffer_unordered(5)`, and the internal function ended with `store.update_account(account)` synchronous save. N accounts = N serialized `atomic_write`s (all contending the same config write lock). Fix: `api_commands::reset_credits_internal` switches to `store.update_account_no_save(account)` + `store.request_save_coalesced()`. Since this function is also the shared backend for the single-account `reset_credits` entry, both entries now uniformly run under coalescer semantics
  - **Performance metrics**: 50-account batch credit reset `atomic_write` count drops from 50 → 1~2 (network RTT now dominates wall time; save is no longer a bottleneck)
  - **Files changed**: `src-tauri/src/commands/api_commands.rs::reset_credits_internal`. Zero frontend changes
- **Batch operations status summary (v1.7.7 complete)**: At this point, **all** Simple-end batch commands involving account persistence run through save coalescer or DataStore-native batch methods: `delete_accounts_batch` ✓, `batch_update_account_tags` ✓, `update_accounts_order` ✓, `batch_refresh_tokens` ✓ (`update_account_no_save` + final `flush`), `batch_reset_credits` ✓ (this release), batch-import 8 commands ✓ (this release). Batch commands that don't touch the account store (`batch_get_users_parsed`) need no optimization
- **Scope**: Simple end v1.7.7 synchronizes the batch-delete, batch-import, and batch-credit-reset performance fixes from the main project. As Simple is a lightweight edition without the "task queue" or "seamless registration" modules, the other two main-project v1.7.7 fixes ("Task queue throughput + seamless registration double fix (F1/F2/F3/B1/U1)" and "Task queue max concurrency cap raised from 10 to 10000") do not apply to Simple. The save coalescer architecture automatically benefits all batch scenarios
- **Account Storage Layer SQLite Refactor (10W+ Account Performance Optimization)**: Migrated `accounts.json` full JSON storage to SQLite database (`accounts.db`), completely resolving slow startup, laggy operations, and excessive disk IO with large account volumes. Auto-detects and migrates old JSON data to SQLite on startup; clears JSON accounts field after migration to prevent re-migration
  - **New `SqliteAccountStore` module** (`src-tauri/src/repository/sqlite_account_store.rs`): Complete SQLite account storage layer with CRUD, paginated queries (server-side filtering/searching/sorting), aggregate statistics (group/plan/domain/tag counts), batch operations, status filtering (SQL CASE expression mirroring frontend `getAccountStatusType` priority chain), and expiry days filtering (`julianday` computation)
  - **Backend DataStore delegates all account methods to SQLite**: 20+ methods including `add_account` / `update_account` / `delete_account` / `get_account` / `get_all_accounts` / `update_account_token` / `export_data` / `import_data` etc. all changed from operating on `config.accounts` to delegating to `account_store`. Method signatures unchanged, zero changes to `commands/*.rs`
  - **3 new Tauri commands**: `get_accounts_page` (paginated query), `get_account_aggregates` (aggregate statistics), `get_all_account_ids` (lightweight ID list)
  - **Backend `get_stats` fix**: Reads account data from SQLite instead of emptied `config.accounts`, fixing stats page showing all zeros
  - **Frontend accounts store refactor**: `accounts` array only holds current page data (20-100 items); all filtering/searching/sorting delegated to backend SQLite; `allTags` / `allPlanNames` / `allDomains` / `allGroups` fetched from backend aggregate queries; `setFilter` / `setCurrentPage` / `setPageSize` changes trigger `fetchPage()` backend re-query
  - **Frontend statistics/count fixes**: Sidebar group counts from `aggregates.group_counts`; tag usage counts from `aggregates.tag_counts`; edit account available tags use `allTags`; auto-reset group stats use `aggregates.group_counts`; batch refresh uses `getAllAccountIds` for full ID list from backend
  - **Backup/restore includes `accounts.db`**: Backup and restore logic now includes SQLite database file
  - **Performance metrics**: IPC transfer 300MB → <100KB (99.97% reduction); disk writes from full JSON dump → single SQLite UPDATE ~1KB; frontend computed iterating 100K objects → directly returning 20 current-page items; startup no longer needs to read 300MB JSON

### v1.7.6 (2026-04-21)
- **Ported "API Keys" + "Provider Key" tabs to the simple AccountInfoDialog**: `AccountInfoDialog.vue` now adds an "API Keys" tab (API key generation / deletion / copy, migrate API Key, and model leaderboard query — 4 sub-sections) and a "Provider Key" tab (third-party AI provider key CRUD: OpenAI / Anthropic / Gemini / XAI / OpenRouter / Groq / Fireworks / Cerebras / Together AI / Azure and other 13 providers) after the Firebase tab. The existing User Details / Local Info / Firebase tabs get `name` attributes + `activeInfoTab` + `onInfoTabChange` so switching to the new tabs auto-loads data. Two sub-dialogs added: "New API Key Generated" and "Add Provider Key", plus 340+ lines of CSS with dark-mode adaptation
- **Stable display of current session_token at top of API Keys tab for Devin accounts**: When the active account has `auth_provider === 'devin'`, a stable block is rendered right after the "API Keys" tab header, directly reading the local `account.token` (already prefixed with `devin-session-token$` by the backend, zero frontend concatenation) + copy button; a green theme distinguishes it from the Firebase sk-ws-01 style. `AccountInfoDialog.vue` imports `useAccountsStore` and adds 3 computed refs (`currentAccount` / `isDevinAccount` / `devinSessionApiKey`) with zero new backend calls (pure frontend read from the Pinia cache). The block is not rendered for Firebase accounts, keeping the existing "click Generate New Key" flow unchanged
- **Backend: 10 new Tauri commands + 12 new service methods**: `api_commands.rs` adds `get_api_key_summary` / `delete_api_key` / `register_user_api_key` / `get_set_user_api_provider_keys` / `set_user_api_provider_key` / `delete_user_api_provider_key` / `migrate_api_key` / `get_primary_api_key_for_devs` / `get_global_leaderboard_api_key` / `get_leaderboard`. `windsurf_service.rs` adds `register_user` (generates sk-ws-01 API Key via Firebase ID Token through RegisterUser), `provider_id_to_name` / `provider_name_to_id` (33-provider bidirectional mapping), `parse_leaderboard_result` (parses `BigQueryResult.leaderboard_result.model_stats`), and the corresponding Windsurf SeatManagement / UserAnalytics gRPC calls. `lib.rs` registers the new commands grouped by function. Reuses existing simple-side helpers `encode_string_field` / `encode_varint` / `proto_parser::ProtobufParser` / `ensure_valid_token` / `AuthContext::from_account` with zero new helpers
- **Ported "Check Pro Trial Eligibility" feature to simple**: `WindsurfService::check_pro_trial_eligibility` + `parse_is_eligible_strict` + `read_varint` strictly parse the `CheckProTrialEligibility` response per protobuf wire format, combined with Content-Type validation for fail-fast, avoiding the risk of "expired but structurally valid Firebase ID Tokens being mistakenly judged as `is_eligible=true` by the backend". The frontend first calls `get_account_valid_token` to force-refresh the token before calling the check; when eligible, it auto-adds a "Trial Eligible" tag (color `#E6A23C`); when not eligible, it auto-removes any legacy tag for data consistency. A new "Check Pro Trial Eligibility" button (Trophy icon, info plain style) is placed after the "Get Trial Link" button
- **Adjusted AccountCard.vue button layout in both projects**: Moved the "Convert Login Method" button from the end of the second row (after one-click switch) to the first row right after "Account Info", now sitting in the same row as high-frequency management actions like "Delete Account". Tooltip dynamically shows `Firebase→Devin` / `Devin→Firebase` based on `auth_provider`
- **Auto-display of "Current Account" + "XXX Version" by active client (Windsurf / Windsurf - Next dual)**: Refactored `windsurf_info.rs` with a "process-first + `state.vscdb` mtime fallback" strategy — only one process running → pick it; both running → more recent `state.vscdb` mtime; neither running → the last-opened one. Added `ClientCandidate` const table (Windsurf + Windsurf - Next), `is_process_running` (Windows `tasklist` / Unix `pgrep`), `resolve_active_client` decision function, and a generalized `read_client_info` reader. `WindsurfCurrentInfo` struct gains 3 new fields: `client_type` / `client_display_name` / `is_running`. Frontend `api/index.ts` type updated; `MainLayout.vue` adds `windsurfClientDisplayName` ref; `AboutDialog.vue` changes the hard-coded "Windsurf Version" label to a dynamic `clientDisplayName + ' Version'` that switches between "Windsurf Version ↔ Windsurf - Next Version"; a `.client-hint` badge next to the "Current Account" label indicates the source client
- **Fixed simple-end "Current Account" retrieval failure**: The simple-end original `windsurf_info.rs` parsed only legacy top-level JSON fields; after Windsurf migrated account info into the `userStatusProtoBinaryBase64` protobuf, that path silently returned empty. This release aligns the simple end fully with the main project implementation (keeping `apiKey` dual-path compatibility + protobuf email/name extraction + legacy JSON field fallback) with zero new dependencies (`base64` / `regex` / `rusqlite` / `serde_json` were already in both Cargo.toml)
- **Scope**: simple-side account info dialog (3 tabs → 5 tabs); AccountCard first-row buttons (convert login + check Pro trial added); backend Tauri commands active count 38 → 48; the "About" dialog now recognizes both Windsurf and Windsurf - Next clients and automatically switches title and account data source based on which is active. Main project synchronously adjusts button position and active-client logic so both projects share the same UI interaction model

### v1.7.5 (2026-04-20)
- **Added Firebase ↔ Devin account auth-provider conversion**: Addresses the scenario where the official backend migrated some old Firebase accounts to Devin while passwords remained unchanged, making local `refresh_token` refresh fail. Added two backend commands `convert_account_to_devin` / `convert_account_to_firebase` that reuse the account's stored plaintext password to call the target system's official login endpoint and perform an atomic field switch. AccountCard second action row adds a "Convert Login Method" button with dynamic tooltip (`Firebase→Devin` / `Devin→Firebase`) based on current `auth_provider`; for Firebase→Devin multi-org scenarios, an inline `ElMessageBox + h()` radio list lets the user pick `org_id` and the command is called again with the selection. Design highlights: idempotent (when target == current, returns `already_converted=true` without any network call), atomic (network failure leaves local fields untouched), and post-switch enrichment via GetCurrentUser + GetPlanStatus to backfill quota fields
- **Merged AddAccountDialog cards + mode-card grid restyle**: Removed the standalone "Devin Native Signup" card and merged it into the unified "Devin Email Code" card. After entering, users pick from two-level mode-card grids: `source` (Devin Native / Windsurf Side) + conditional `flow` (login / signup, only for Windsurf Side); Step 1 renders password/name fields based on `source+flow`. Added `sourceOptions` / `flowOptions` / `selectSource` / `selectFlow`; validation rules consolidated to `devinEmailCodeStep0/Step1/Step1Signup`; removed all legacy `devinNative*` state; `completeDevinNativeRegister` renamed to `completeDevinEmailCodeNativeRegister` reusing the unified `devinEmailCodeEmailToken` / `formData.devinEmailCodeOtp` state
- **Fixed first-time WindsurfPostAuth 404 race condition**: `devin_auth_service.rs::post_auth_to_login_result` now does exponential backoff retry (1.5s / 3s / 5s, 3 attempts) specifically for `404 + no_eligible_organizations / not_found` errors, targeting the Devin server-side org sync delay that happens right after account creation (new `is_post_auth_org_sync_pending` helper precisely identifies the transient error). Covers `login_with_password` / `register_with_email_code` / `login_with_email_code` / `register_native_with_email_code`; all other errors remain fail-fast
- **Added `enrich_account_with_plan_status` / `enrich_account_with_plan_status_inner`** (`devin_commands.rs`): Dedicated entry point for "call GetPlanStatus only and backfill account fields", sharing ctx construction logic with `enrich_account_with_user_info`
- **Fixed QUOTA mode field loss on Devin register/login paths (A.1)**: `enrich_account_with_user_info` now calls GetPlanStatus after GetCurrentUser, uniformly backfilling `billing_strategy` / `daily_quota_remaining_percent` / `weekly_quota_remaining_percent` / `daily/weekly_quota_reset_at_unix` / `overage_balance_micros` etc., preventing the account card from downgrading to CREDITS-style display
- **Fixed QUOTA field loss on Devin token import paths (A.2)**: `add_account_by_devin_session_token` / `add_account_by_devin_auth1_token` now explicitly call `enrich_account_with_plan_status` after `apply_user_info_to_account` to supplement QUOTA fields; the auth1_token path passes `&ctx.token` to avoid the E0502 borrow conflict where `account.token` has been moved into `Option`
- **Fixed QUOTA field loss on Firebase refresh_token import path**: `account_commands.rs::add_account_by_refresh_token` adds a GetPlanStatus call + `apply_plan_status_to_account` after GetCurrentUser; added `use crate::commands::api_commands::apply_plan_status_to_account` import
- **Fixed E0502 borrow conflict**: At `devin_commands.rs:1144`, `account.token` was moved into `Some(session_token)`, so later borrowing it immutably via `account.token.as_deref()` violated Rust borrow rules — switched to using the already-constructed `ctx.token` (same value as session_token, still owned and borrowable) for `enrich_account_with_plan_status`
- **Unified version numbers**: `package.json` / `Cargo.toml` bumped from `1.7.3` to `1.7.5` to align with `tauri.conf.json`, eliminating the `tauri-action` version consistency check warning
- **Scope**: All Devin Native / Windsurf Side email-code account creation paths and the Firebase refresh_token import path; systematically fixes the CREDITS downgrade issue; UI: 2 Devin email-code cards → 1 (main cards reduced from 8 to 7)

### v1.7.4 (2026-04-20)
- **Added Auto-Update Feature (tauri-plugin-updater)**: Integrated official updater + process plugins. On app startup (3s delay), silently checks the latest release on GitHub Releases and automatically pops up the update dialog when a new version is available. Supports real-time download progress, one-click restart after install, per-version "skip this version" memory, and 24-hour check debounce
- **Added UpdateDialog.vue (standalone update dialog)**: Displays current version / new version / release notes / download progress / install status; supports "Install Now / Skip This Version / Remind Me Later"
- **Added "Check for Updates" button in About dialog**: Next to version number, manually triggers update check (bypassing 24h debounce); button label reflects state dynamically (Checking / Install Now / Downloading / Installing / Restart)
- **Added Pinia updater store** (`src/store/modules/updater.ts`): Unified management of update check, download, install, restart, and skip-version state & actions
- **Updated GitHub Actions workflow** (`.github/workflows/build.yml`): Injected `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` env vars across all 6 build jobs for signing; Upload artifacts now also collects `.msi.sig` / `.exe.sig` / `.app.tar.gz.sig` / `.AppImage.sig`; Release job adds a new step to merge per-platform signatures into a unified `latest.json` and upload to the Release
- **Added Tauri config**: `src-tauri/tauri.conf.json` now includes `plugins.updater` (endpoints pointing to GitHub Releases latest.json, Windows passive install mode, embedded minisign pubkey) and `bundle.createUpdaterArtifacts: true`; `capabilities/default.json` grants `updater:default` + `process:default`
- **Added frontend dependencies**: `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`
- **Scope**: All platforms (Windows x64/arm64, macOS x64/arm64, Linux x64 AppImage); Linux ARM64 does not support auto-update due to pre-existing cross-compilation limitation (unchanged)

### v1.7.3 (2026-04-20)
- **Fixed Devin Refresh Logic**: Added key field backfill, fixed product parameter case sensitivity, and improved batch refresh store synchronization
- Optimized Devin session_token expiration placeholder mechanism
- Improved seamless switching progress event system

### v1.7.2 (2026-04-18)
- **Added Complete Devin Account System Support**: Implemented auth1_token authentication, session_token refresh, and 5-header authentication mechanism
- Optimized update_codeium_access error handling: Added Connect Protocol error code mapping and transparent logging
- Improved Devin session_token management logic

### v1.7.1 (2026-04-14)
- **Added Multi-client Support**: Support independent configuration and management for Windsurf and Windsurf - Next
- Optimized client detection logic
- Improved configuration file path management

### v1.7.0 (2026-04-10)
- **Added Billing Strategy and Quota Detail Field Support**: Support for billing_strategy, daily_quota, weekly_quota, etc.
- Extended OperationType enum: Added group, tag, team, data management, account switching, subscription, and registration operation types
- Optimized batch refresh logic: Reuse apply_plan_status_to_account function and simplify account update process

### v1.6.7 (2026-01-09)
- Fixed Cunzhi feature issues
- Optimized stability

### v1.6.6 (2026-01-08)
- **Refactored Subscription Payment Feature**: Support Teams/Pro plans and monthly/yearly billing
- Refactored MCP tool naming: Unified `windsurf-cunzhi` and `zhi` tools to `dialog-helper` and `confirm`

### v1.6.5 (2025-12-30)
- Restored full functionality version
- Removed unused Rank icon import
- Added QQ group QR code to README community section
- Updated README.md: Improved feature description and interface display

### v1.6.4 (2025-12-24)
- **Optimized Analytics Data Retrieval Logic**: Improved GetAnalytics API call efficiency
- Updated windsurf-cunzhi and windsurf-cunzhi-ui binary files for all platforms
- Optimized cunzhi file packaging strategy: Copy platform-specific files during build to avoid packaging all platform files
- Fixed Linux ARM64 connector issue
- Skipped AppImage packaging in Linux ARM64 build, only generate deb and rpm packages

---

## Tech Stack

### Frontend
- **Framework**: Vue 3.5.13 + TypeScript 5.6.2
- **UI Components**: Element Plus 2.11.8
- **Icon Library**: @element-plus/icons-vue 2.3.2
- **State Management**: Pinia 3.0.4
- **Build Tool**: Vite 6.0.3
- **Styling**: CSS3 + Element Plus Theme + Sass 1.94.2
- **HTTP Client**: Axios 1.13.2
- **Encryption Library**: Crypto-js 4.2.0
- **Date Processing**: Dayjs 1.11.19
- **Chart Library**: ECharts 6.0.0
- **Drag & Drop**: Vuedraggable 4.1.0
- **UUID Generation**: UUID 13.0.0

### Backend
- **Framework**: Tauri 2.x
- **Language**: Rust 2021 Edition
- **Encryption**: AES-256-GCM (aes-gcm 0.10)
- **Key Management**: Windows Credential Manager / Keyring 2.0
- **Network Requests**: Reqwest 0.11 (supports JSON, SOCKS proxy)
- **Async Runtime**: Tokio 1.x (full features)
- **Serialization**: Serde 1.0 + Serde JSON 1.0
- **Database**: SQLite 3.31 (bundled) - For Windsurf state.vscdb operations
- **Time Processing**: Chrono 0.4
- **Encoding**: Base64 0.21 + Hex 0.4
- **Error Handling**: Anyhow 1.0 + Thiserror 1.0
- **Logging**: Log 0.4 + Env Logger 0.10
- **File Compression**: Zip 0.6
- **Regular Expression**: Regex 1.10

### Windows-specific Dependencies
- **Registry Operations**: Winreg 0.52
- **Windows API**: Winapi 0.3 (securitybaseapi, processthreadsapi, winnt, handleapi, minwindef, shellapi, winuser)

### Unix-specific Dependencies
- **System Calls**: Libc 0.2

## Installation and Running

### Prerequisites
- Node.js 16+
- Rust 1.70+
- Windows 10/11 (currently only supports Windows)

### Development Environment

```bash
# Clone the project
git clone https://github.com/chaogei/windsurf-account-manager-simple.git
cd windsurf-account-manager-simple

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Build Release Version

```bash
# Build Windows installer
npm run tauri build
```

After building, the installer is located at `src-tauri/target/release/bundle/`

## Usage Guide

### 1. First Use

1. After starting the application, click the "Add Account" button (+ icon) in the top right
2. Enter Windsurf account information in the dialog:
   - **Email**: Your Windsurf account email
   - **Password**: Account password
   - **Display Name**: Easy-to-identify name (optional)
   - **Group**: Select or create a group (optional)
   - **Tags**: Add custom tags (optional)
3. Click "OK" to save the account
4. The application will automatically log in and fetch account information

### 2. Credit Reset

**Single Account Reset**:
1. Click the "Credit Reset" button (refresh icon) on the account card
2. The application will automatically:
   - Log in to get Token (if needed)
   - Execute a seat update (auto-switch between 3/4/5)
   - Credit reset is complete when seat update succeeds
3. Operation result will be notified
4. Enable "Show Detailed Results" in settings to view specific seat update information

**Batch Reset**:
1. Check multiple account cards
2. Click the "Batch Reset Credits" button at the top
3. Confirm to execute in batch (max 5 concurrent)
4. View operation logs for detailed results

**Auto Reset**:
1. Click the "Auto Reset" button
2. Set reset schedule (time, frequency, accounts)
3. System will automatically execute reset at specified time
4. View auto reset records in operation logs

### 3. Batch Operations

**Batch Import**:
1. Click the "Batch Import" button (multiple squares icon)
2. Prepare account list file, format:
   ```
   email password note
   user1@example.com password123 account1
   user2@example.com password456 account2
   ```
3. Select file and import
4. Confirm import information

**Batch Delete**:
1. Check the account cards to delete
2. Click the "Batch Delete" button (trash icon)
3. Confirm delete operation

**Batch Refresh**:
1. Check the account cards to refresh
2. Click the "Batch Refresh Status" button
3. System will batch update account credits, plans, etc.

**Export Accounts**:
1. Click the "Export Accounts" button (download icon)
2. Select export format (CSV/JSON/Text)
3. File will be automatically downloaded locally

### 4. Account Grouping

**Create Group**:
1. Click the "Groups" menu in the sidebar
2. Click the "Add Group" button
3. Enter group name and color
4. Save group

**Manage Groups**:
1. Edit or delete groups in the group list
2. Select group when adding/editing accounts
3. Groups help you better manage multiple accounts

### 5. Tag Management

**Add Tags**:
1. Click the "Tags" input field when adding/editing accounts
2. Enter tag name
3. Select tag color
4. Save tag

**Manage Tags**:
1. Click "Tag Management" in settings
2. Edit or delete existing tags
3. Tags help you quickly filter accounts

### 6. Team Management

**View Team Members**:
1. Click the "Team Management" button on the account card
2. View all team member information
3. View member credit usage

**Invite Members**:
1. Click "Invite Member" in the team management dialog
2. Enter member email
3. Set member permissions
4. Send invitation

**Remove Members**:
1. Select the member to remove in the member list
2. Click the "Remove Member" button
3. Confirm removal operation

### 7. One-click Account Switching

**Standard Switching**:
1. Click the "Switch" button (Switch icon) on the account card
2. System will automatically:
   - Use refresh_token to get new access_token
   - Call RegisterUser API to get api_key
   - Encrypt sessions data and write to Windsurf database
   - Update Windsurf configuration files
   - Reset machine ID
   - Auto-restart Windsurf
3. Wait for Windsurf restart to complete

**Seamless Switching (requires patch)**:
1. Click the "Seamless Switching Patch" button
2. System will automatically detect Windsurf installation path
3. Click "Apply Patch" to modify extension.js
4. Patch will remove 180-second timeout limit
5. Restart Windsurf after applying patch
6. Afterwards, you can click "Seamless Switch" button anytime for quick switching

### 8. View Logs

1. Click "Operation Logs" in the sidebar
2. View all operation records
3. Filter logs by type and time
4. Support clearing or exporting logs

### 9. Billing Query

**View Billing Information**:
1. Click the "Billing" button on the account card
2. View subscription type, expiration time
3. View payment method, next billing date
4. View invoice link

**Update Plan**:
1. Click the "Update Plan" button
2. Select new plan type (Teams/Pro)
3. Select billing cycle (monthly/yearly)
4. Confirm update

### 10. Usage Analytics

**View Usage Statistics**:
1. Click the "Analytics" button on the account card
2. View daily active statistics
3. View tool usage
4. View model usage
5. View Token consumption statistics

## Data Storage

### Application Data Storage

Application data is stored locally:
- **Windows**: `%APPDATA%\com.chao.windsurf-account-manager\accounts.json`

Data structure includes:
- Account information (encrypted passwords and Tokens)
- Group list
- System settings
- Operation logs

### Windsurf Data Storage Architecture

Windsurf uses a three-layer storage architecture to manage login credentials:

**1. Encrypted Layer**
- **Storage Location**: `secret://windsurf_auth.sessions`
- **Encryption Method**: Windows DPAPI encryption
- **Data Format**: Buffer byte array
- **Purpose**: Store session data and API Key

**2. Status Layer**
- **Storage Location**: `windsurfAuthStatus`
- **Data Format**: Plain text JSON
- **Contains**: API Key (sk-ws-01 prefix), user information, team configuration
- **Purpose**: Store authentication status and basic user information

**3. Plain Text Layer**
- **Storage Location**: `codeium.windsurf-windsurf_auth` and `codeium.windsurf`
- **Data Format**: Plain text
- **Contains**: Username, basic configuration
- **Purpose**: Store user identifier and configuration information

### Key File Paths

**Database File**:
- **Path**: `%APPDATA%\Windsurf\User\globalStorage\state.vscdb`
- **Format**: SQLite
- **Table Name**: ItemTable (key-value storage)

**Configuration File**:
- **Path**: `%APPDATA%\Windsurf\User\globalStorage\storage.json`
- **Format**: JSON
- **Contains**: Machine ID, device identifier, etc.

### Security Mechanisms

- **Windows Credential Manager Integration**: Encryption keys stored in system keychain
- **DPAPI Encryption**: Bound to user account, only current user can decrypt
- **Protobuf Encoding**: Permission configuration uses Protobuf encoding
- **Important Note**: API Key is stored in plain text in windsurfAuthStatus, protecting the database file is crucial

## Security Notes

1. **Password Security**: All passwords are encrypted with AES-256-GCM
2. **Key Management**: Encryption keys are stored in the system keychain
3. **Token Refresh**: Tokens are automatically refreshed 5 minutes before expiration
4. **Local Storage**: All data is stored locally only, not uploaded to any server

## Important Notes

1. Please keep your account information safe
2. Regularly backup the `accounts.json` file
3. Pay attention to API rate limiting during batch operations
4. It is recommended to use the grouping feature to manage multiple accounts

## Development Guide

### Project Structure

```
windsurf-account-manager-simple/
├── src/                           # Vue frontend source code
│   ├── views/                     # Page components
│   │   └── MainLayout.vue         # Main layout page (contains all features)
│   ├── components/                # Reusable components
│   │   ├── AboutDialog.vue        # About dialog
│   │   ├── AccountCard.vue        # Account card component
│   │   ├── AccountInfoDialog.vue  # Account detail dialog
│   │   ├── AddAccountDialog.vue   # Add account dialog
│   │   ├── AnalyticsDialog.vue    # Usage analytics dialog
│   │   ├── AutoRefillDialog.vue   # Auto refill dialog
│   │   ├── AutoResetDialog.vue    # Auto reset dialog
│   │   ├── BatchImportDialog.vue  # Batch import dialog
│   │   ├── BatchUpdatePlanDialog.vue  # Batch update plan dialog
│   │   ├── BillingDialog.vue      # Billing info dialog
│   │   ├── CardGeneratorDialog.vue   # Virtual card generation dialog
│   │   ├── CreditHistoryDialog.vue    # Credit history dialog
│   │   ├── EditAccountDialog.vue  # Edit account dialog
│   │   ├── LogsDialog.vue        # Operation logs dialog
│   │   ├── SettingsDialog.vue    # Settings dialog
│   │   ├── StatsDialog.vue       # Statistics dialog
│   │   ├── TagColorPicker.vue    # Tag color picker
│   │   ├── TagManageDialog.vue   # Tag management dialog
│   │   ├── TeamManagementDialog.vue  # Team management dialog
│   │   ├── TeamSettingsDialog.vue    # Team settings dialog
│   │   ├── TurnstileDialog.vue   # Turnstile verification dialog
│   │   ├── UpdatePlanDialog.vue  # Update plan dialog
│   │   ├── UpdateSeatsResultDialog.vue  # Seat update result dialog
│   │   └── WelcomeDialog.vue     # Welcome dialog
│   ├── api/                       # API wrapper layer
│   ├── services/                  # Business service layer
│   ├── store/                     # Pinia state management
│   ├── types/                     # TypeScript type definitions
│   ├── utils/                     # Utility functions
│   ├── assets/                    # Static resources
│   ├── App.vue                    # Root component
│   └── main.ts                    # Application entry
├── src-tauri/                     # Rust backend source code
│   ├── src/
│   │   ├── commands/              # Tauri command layer (frontend-backend communication interface)
│   │   │   ├── account_commands.rs    # Account management commands
│   │   │   ├── analytics_commands.rs  # Analytics data commands
│   │   │   ├── api_commands.rs        # API call commands
│   │   │   ├── auto_reset_commands.rs # Auto reset commands
│   │   │   ├── cunzhi_commands.rs     # Cunzhi feature commands
│   │   │   ├── devin_commands.rs      # Devin account commands
│   │   │   ├── payment_commands.rs    # Payment related commands
│   │   │   ├── patch_commands.rs      # Patch application commands
│   │   │   ├── proto_commands.rs      # Protobuf parsing commands
│   │   │   ├── settings_commands.rs   # Settings related commands
│   │   │   ├── switch_account_commands.rs  # Account switching commands
│   │   │   ├── team_commands.rs       # Team management commands
│   │   │   ├── app_info.rs            # App info commands
│   │   │   ├── validate_path.rs       # Path validation commands
│   │   │   ├── windsurf_info.rs       # Windsurf info commands
│   │   │   └── mod.rs                 # Command module entry
│   │   ├── models/               # Data models
│   │   │   ├── account.rs       # Account model
│   │   │   ├── group.rs         # Group model
│   │   │   ├── operation_log.rs # Operation log model
│   │   │   └── settings.rs      # Settings model
│   │   ├── repository/           # Data access layer
│   │   │   └── data_store.rs    # Data storage implementation
│   │   ├── services/             # Business logic layer
│   │   │   ├── auth_service.rs  # Authentication service
│   │   │   └── windsurf_service.rs  # Windsurf API service
│   │   ├── utils/                # Utility functions
│   │   │   ├── crypto.rs        # Encryption tools
│   │   │   ├── proto_parser.rs  # Protobuf parser
│   │   │   └── error.rs         # Error handling
│   │   ├── lib.rs               # Rust library entry
│   │   └── main.rs              # Tauri application entry
│   ├── Cargo.toml               # Rust dependency configuration
│   ├── tauri.conf.json          # Tauri configuration file
│   └── build.rs                 # Build script
├── public/                      # Public resources
│   ├── 交流群.png               # WeChat group QR code
│   ├── QQ群.jpg                 # QQ group QR code
│   └── 主页.png                 # Interface screenshot
├── scripts/                     # Script tools
│   └── sync-version.js          # Version sync script
├── docs/                        # Documentation directory
├── package.json                 # Node dependency configuration
├── vite.config.ts               # Vite configuration
├── tsconfig.json                # TypeScript configuration
├── tsconfig.node.json           # Node environment TypeScript configuration
├── build_with_admin.bat         # Admin privilege build script
├── set_admin_manifest.ps1       # Set admin manifest script
└── README.md                    # Project documentation
```

### API Integration

The application integrates the following Windsurf APIs:

#### Firebase Authentication API
- **Endpoint**: `https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword`
- **Endpoint**: `https://securetoken.googleapis.com/v1/token`
- **Function**: User login, Token refresh
- **Authentication**: API Key + Referer header
- **Important**: Must carry `Referer: https://windsurf.com/` header, otherwise returns 403 error (`API_KEY_HTTP_REFERRER_BLOCKED`)
- **Recommended**: Also add `X-Client-Version: Chrome/JsCore/11.0.0/FirebaseCore-web` header

#### Seat Management API
- **UpdateSeats**: Update team seat count
  - Endpoint: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/UpdateSeats`
  - Function: Reset credits by modifying seat count
  - Protocol: Connect-Web (gRPC-Web)
  - Request Format: Protobuf

- **GetPlanStatus**: Get account plan status
  - Endpoint: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetPlanStatus`
  - Function: Get credit balance, quota info, billing strategy
  - Response Fields: available_flex_credits, available_prompt_credits, used_flex_credits, used_prompt_credits, billing_strategy, daily_quota, weekly_quota

#### Billing Query API
- **GetTeamBilling**: Get team billing information
  - Endpoint: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetTeamBilling`
  - Function: Query subscription status, payment method, billing cycle
  - Response Fields: Subscription type, seat count, unit price, payment method, next billing date

#### User Management API
- **GetCurrentUser**: Get current user information
  - Endpoint: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetCurrentUser`
  - Function: Get user basic info, team info, plan info, permission info
  - Response Fields: UserBasicInfo, TeamInfo, PlanInfo, UserRole

- **GetUsers**: Get team member list
  - Endpoint: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetUsers`
  - Function: Batch get team member information
  - Response Fields: users[], user_roles[], user_team_details[], user_cascade_details[]

- **RegisterUser**: Register user to get API Key
  - Endpoint: `https://register.windsurf.com/exa.seat_management_pb.SeatManagementService/RegisterUser`
  - Function: Use Firebase Token to register and get sk-ws-01 format API Key
  - Response Fields: api_key, name, api_server_url

**API Key Format Description**:
- **Format**: sk-ws-01-[88-94 characters Base64 encoded]
- **Total Length**: 103 characters
- **Structure**: sk(Secret Key) + ws(WindSurf) + 01(version) + Base64 encoded payload
- **Character Distribution**: Uppercase 35%, lowercase 45%, numbers 17%, hyphens 4%
- **Randomness**: 96.7% (close to maximum entropy, high security)
- **Encoding**: Base64 encoding, decodes to 32 bytes binary data
- **Acquisition Flow**: Firebase login to get ID Token → Call RegisterUser API → Return sk-ws-01 format API Key
- **Important**: RegisterUser API supports both register.windsurf.com and web-backend.windsurf.com

#### Team Management API
- **InviteTeamMember**: Invite team member
- **RemoveTeamMember**: Remove team member
- **UpdateTeamSettings**: Update team settings

#### Analytics Data API
- **GetAnalytics**: Get usage analytics data
  - Endpoint: `https://web-backend.windsurf.com/exa.user_analytics_pb.UserAnalyticsService/GetAnalytics`
  - Function: Get daily active statistics, tool usage, model usage, Token consumption
  - Response Fields: cascade_lines, cascade_tool_usage, cascade_runs, daily_active_user_counts

#### Credit History API
- **GetTeamCreditEntries**: Get credit acquisition history
  - Endpoint: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetTeamCreditEntries`
  - Function: Query credit acquisition records (referral rewards, purchases, etc.)
  - Response Fields: FlexCreditChronicalEntry (team ID, grant time, credit amount, type, reason)

#### Referral Code API
- **ProcessReferralCode**: Process referral code
  - Endpoint: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/ProcessReferralCode`
  - Function: Use referral code to get 25,000 PROMPT credit reward
  - Request Fields: auth_token, referral_code

#### Devin API
- **auth1_token Authentication**: Devin exclusive authentication mechanism
- **session_token Refresh**: Auto-refresh session_token
- **5-header Authentication**: Complete authentication header setup

---

### Request Header Format Description

Windsurf API calls require complete browser-related request headers:

**Required Headers**:
- `connect-protocol-version: "1"` - Connect-Web protocol version
- `x-debug-email: ""` - Debug email (empty string)
- `x-debug-team-name: ""` - Debug team name (empty string)
- `Referer: https://windsurf.com/` - Referer header (required for Firebase API)
- `X-Client-Version: Chrome/JsCore/11.0.0/FirebaseCore-web` - Client version (recommended)

**Browser Identification Headers**:
- `sec-ch-ua` - User agent hint
- `sec-ch-ua-mobile` - Mobile device hint
- `sec-ch-ua-platform` - Platform hint
- `User-Agent` - Complete user agent string

### Request Body Format

**UpdateSeats Request Body**:
- Prefix: `0x0a, 0xa1, 0x07` (fixed protocol identifier)
- Token: Authentication Token
- Seat Bytes: `0x10, seat_count`

**GetTeamBilling Request Body**:
- Prefix: `0x0a, 0xa1, 0x07` (fixed protocol identifier)
- Token: Authentication Token

**Note**: Prefix `0xa1 0x07` is a fixed protocol identifier, do not modify

---

### Architecture Design

#### Overall Architecture
The application adopts **Tauri desktop application architecture** with frontend-backend separation:

```
┌─────────────────────────────────────────────────────────┐
│                     Frontend (Vue 3)                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  UI Layer    │  │  State Layer │  │  API Layer    │ │
│  │  Components  │  │    Pinia     │  │   Axios/Tauri │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────┘
                           │
                    Tauri IPC
                           │
┌─────────────────────────────────────────────────────────┐
│                    Backend (Rust)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  Command     │  │   Service    │  │   Repository │ │
│  │  Layer       │  │    Layer     │  │     Layer     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   Model      │  │    Utils     │  │    Crypto     │ │
│  │   Layer      │  │    Layer     │  │    Layer      │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────┘
```

#### Layered Design

**1. Frontend Layering**
- **UI Component Layer**: Reusable Vue components (dialogs, cards, forms, etc.)
- **State Management Layer**: Pinia Store manages application state
- **API Call Layer**: Encapsulates Tauri IPC calls, communicates with backend

**2. Backend Layering**
- **Command Layer (Commands)**: Tauri command interfaces, handle frontend requests
- **Service Layer (Services)**: Business logic processing (authentication, API calls, etc.)
- **Data Layer (Repository)**: Data persistence (JSON files, keychain)
- **Model Layer (Models)**: Data structure definitions
- **Utility Layer (Utils)**: Common utility functions (encryption, parsing, etc.)

#### Data Flow

```
User Action → Frontend Component → Pinia Store → Tauri Command → Rust Service → API Call
                                                                    ↓
Response ← Frontend Component ← Pinia Store ← Tauri Command ← Rust Service ← Windsurf API
```

#### Security Architecture

**1. Encrypted Storage**
- Account Passwords: AES-256-GCM encryption
- Encryption Keys: Windows Credential Manager storage
- Tokens: Memory cache + auto-refresh mechanism

**2. Authentication Flow**
- Firebase authentication to get ID Token
- ID Token refreshes expired Tokens
- API calls carry complete authentication headers

**3. Privacy Protection**
- All data stored locally
- No information uploaded to servers
- Payment window uses incognito mode

#### Version Management

Adopts **Single Source of Truth** strategy:
- `version` field in `src-tauri/tauri.conf.json` is the single source of truth
- Frontend dynamically gets version number via `get_app_version` API
- `build.rs` automatically reads `tauri.conf.json` during build to generate Windows manifest
- Use `node scripts/sync-version.js` to sync `package.json` and `Cargo.toml`

#### Error Handling

**1. Frontend Error Handling**
- Try-catch captures asynchronous errors
- ElMessage displays error prompts
- Error logs recorded to operation logs

**2. Backend Error Handling**
- Anyhow unified error handling
- Result<T, String> return type
- Custom error types (AppError)
- Connect Protocol error code mapping

**3. API Error Handling**
- 401 errors trigger Token refresh
- Network errors auto-retry
- Timeout errors prompt user

#### Performance Optimization

**1. Frontend Optimization**
- Virtual scrolling for large account lists
- Debounce/throttle controls request frequency
- Component lazy loading
- Pinia persistent cache

**2. Backend Optimization**
- Async concurrent processing for batch operations
- Token reuse to avoid duplicate logins
- SQLite connection pool
- Protobuf efficient serialization

**3. Network Optimization**
- Lightweight API mode (GetPlanStatus replaces GetCurrentUser)
- Request header caching
- HTTP/2 support
- Proxy support

#### Backup File Management Strategy

**Seamless Switching Patch Backup**:
- **Max 3 Backups**: Prevent unlimited backup file accumulation
- **Auto Clean Old Backups**: When backup count reaches or exceeds 3, automatically delete the oldest backup file
- **Sort by Time**: Sort by file modification time to ensure the oldest backup is deleted
- **Backup File Naming Format**: `extension.js.backup.YYYYMMDD_HHMMSS`
- **Example**: `extension.js.backup.20251122_192115`

**Implementation Logic**:
```rust
// Find all backup files (starting with "extension.js.backup.")
// Sort by modification time
// Delete old backups exceeding the limit (keep latest 3)
```

---

#### UpdateSeats API Response Data Description

The seat update API returns parsed JSON containing:
- `success`: Whether the operation succeeded
- `total_seats`: Total seat count (corresponds to int_4 field)
- `used_seats`: Used seat count (corresponds to int_5 field)
- `seat_usage`: Seat usage (e.g., "1 / 5")
- `seat_usage_percentage`: Seat usage percentage (calculated)
- `price_per_seat`: Monthly price per seat (corresponds to float_3 field, unit: USD)
- `total_monthly_price`: Total monthly price (corresponds to float_6 field, unit: USD)
- `billing_start_time`: Current billing cycle start time (corresponds to subMesssage_7.int_1 timestamp)
- `next_billing_time`: Next billing time (corresponds to subMesssage_8.int_1 timestamp)
- `billing_start_timestamp`: Billing start timestamp (Unix timestamp)
- `next_billing_timestamp`: Next billing timestamp (Unix timestamp)

**Protobuf Field Mapping**:
- `float_3`: Monthly price per seat (e.g., 60 = $60/month/seat)
- `int_4`: Total seat count (e.g., 5)
- `int_5`: Used seat count (e.g., 1)
- `float_6`: Total monthly price (e.g., 300 = $300/month)
- `subMesssage_7.int_1`: Current billing cycle start timestamp
- `subMesssage_8.int_1`: Next billing timestamp

#### GetTeamBilling API Response Data Description

The billing query API returns parsed JSON containing:
- `plan_name`: Plan name
- `base_quota`: Plan base quota (corresponds to int_8 field)
- `extra_credits`: Extra credits (corresponds to int_4 field, optional)
- `total_quota`: Total quota (base_quota + extra_credits)
- `used_quota`: Used quota (corresponds to int_6 field, optional, default 0)
- `cache_limit`: Cache limit (corresponds to int_9 field)
- `payment_method`: Payment method information
- `next_billing_date`: Next billing date
- `invoice_url`: Invoice link
- `monthly_price`: Monthly price

**Protobuf Field Mapping**:
- `int_4`: Extra credits (gifted or purchased extra quota, may not exist)
- `int_6`: Used credits (used quota, may not exist)
- `int_8`: Plan quota (base plan quota)
- `int_9`: Plan cache limit (cannot continue to use when exceeded)

**Note**: When cache usage reaches the cache limit, even if total quota remains, usage cannot continue.

## FAQ

### Q: Why does credit reset fail?
A: Please check the following:
1. **Account password is correct** - Confirm the entered email and password are correct
2. **Network connection is normal** - Check network connection, configure proxy if necessary
3. **Token is expired** - Try re-login to refresh Token
4. **Account is team admin** - Only team admins can execute seat updates
5. **API rate limiting** - Control concurrency during batch operations to avoid triggering rate limits

### Q: How to backup account data?
A: Copy the following file to a safe location:
- **Windows**: `%APPDATA%\com.chao.windsurf-account-manager\accounts.json`
- **Note**: accounts.json contains encrypted sensitive information, please keep it safe when backing up

### Q: What if I forgot my account password?
A: The password stored in the application is AES-256-GCM encrypted and cannot be viewed in plain text. You need to:
1. Reset password on Windsurf official website
2. Delete old account in the application
3. Re-add the account

### Q: What if I encounter "Decryption error: Encoded text cannot have a 6-bit remainder" error?
A: This is caused by changed or damaged encryption key in the keychain. Solution:
1. **Delete old account data file**:
   ```powershell
   Remove-Item "$env:APPDATA\com.chao.windsurf-account-manager\accounts.json" -Force
   ```
2. **Clean old key in Windows keychain**:
   ```powershell
   # Find key
   cmdkey /list | Select-String "WindsurfAccountManager"
   # Delete key
   cmdkey /delete:LegacyGeneric:target=MasterKey.WindsurfAccountManager
   ```
3. **Restart application** - Application will automatically generate new encryption key

### Q: How to configure proxy?
A: Configure HTTP proxy in settings:
1. Click "Settings" in the sidebar
2. Find "Proxy Configuration" section
3. Enter proxy address and port
4. Click "Test Connection" to verify proxy availability
5. Save settings

### Q: What if I encounter "API_KEY_HTTP_REFERRER_BLOCKED" error?
A: This is caused by new HTTP Referer restriction in Google Firebase project:
1. **Error Cause**: All requests to identitytoolkit.googleapis.com and securetoken.googleapis.com must carry `Referer: https://windsurf.com/` header
2. **Solution**: Application has added Referer header to all Firebase API calls
3. **If still encountering error**: Ensure you are using the latest version (v1.6.6+)
4. **Temporary Solution**: Check network environment, some proxies may modify Referer header

### Q: What is lightweight API mode?
A: Lightweight API mode uses `GetPlanStatus` API instead of `GetCurrentUser` API:
- **Advantage**: Reduce request count, improve response speed
- **Disadvantage**: Get less information (only credits and plan status)
- **Use Case**: When only need to view credit balance
- **Enable Method**: Check "Lightweight API Mode" in settings

### Q: What if Windsurf cannot start after one-click switching?
A: Possible causes and solutions:
1. **Machine ID reset failed** - Manually reset machine ID in Windows registry
2. **Sessions data corrupted** - Delete `%APPDATA%\Windsurf\User\globalStorage\state.vscdb` file
3. **Configuration file error** - Delete `%APPDATA%\Windsurf\User\globalStorage\storage.json` file
4. **Re-login** - Re-login to Windsurf after deleting data

### Q: Is the seamless switching patch safe?
A: Patch security description:
1. **Only modifies extension.js** - Only removes 180-second timeout limit
2. **Auto backup** - Automatically backs up original file before patching (max 3 copies)
3. **Reversible** - Can click "Restore Patch" anytime to restore original state
4. **Open source and transparent** - All code is open source, can be reviewed

### Q: How to view detailed API responses?
A: Enable "Show Detailed Results" in settings:
1. Click "Settings" in the sidebar
2. Check "Show Detailed Results"
3. Complete API response information will be displayed during operations
4. Facilitates debugging and troubleshooting

### Q: Why do batch operations fail?
A: Possible causes:
1. **Concurrency limit** - Batch reset supports max 5 concurrent, exceeding will fail
2. **Some accounts invalid** - Some account passwords wrong or banned
3. **Network unstable** - Network fluctuations cause some requests to fail
4. **API rate limiting** - Large number of requests in short time triggers rate limiting
5. **Solution**: View operation logs for specific failure reasons, execute in batches

### Q: What is the difference between Devin accounts and regular accounts?
A: Devin account system characteristics:
1. **Authentication method** - Uses auth1_token and session_token
2. **Token management** - session_token has no explicit expiration time, uses 32-day placeholder mechanism
3. **Refresh mechanism** - Force refresh triggered by 401 errors
4. **Multi-organization support** - Supports multi-organization account management
5. **Key fields** - Auto-supplements key fields like product parameter

### Q: How to use referral code to get credits?
A: Steps to use referral code:
1. Find "Referral Code" field in account info dialog
2. Click link icon next to it to copy referral link
3. Share link with friends to register
4. Both parties get 25,000 PROMPT credits after friend registers

### Q: Which Windsurf versions are supported?
A: Currently supported versions:
1. **Windsurf Standard** - Full support for official Windsurf client
2. **Windsurf - Next** - Support Next version with independent configuration and management
3. **Auto detection** - Application automatically identifies and adapts to different versions

### Q: How to migrate account data to another computer?
A: Migration steps:
1. Backup `accounts.json` file from original computer
2. Backup keys in Windows keychain (use `cmdkey /list`)
3. Install application on new computer
4. Restore `accounts.json` file
5. Restore keychain keys (use `cmdkey /add`)
6. Start application to verify

### Q: Will the application collect my data?
A: Data privacy description:
1. **Local storage** - All data stored locally only
2. **No upload** - No information uploaded to any server
3. **Open source and transparent** - Code completely open source, can be reviewed
4. **Encrypted protection** - Sensitive information encrypted with AES-256-GCM

### Q: How to update the application?
A: Update method:
1. Visit project GitHub Releases page
2. Download latest version installer
3. Uninstall old version (optional)
4. Install new version
5. Data will be automatically retained

### Q: How to report issues?
A: Feedback channels:
1. **GitHub Issues** - Submit Issue on project GitHub page
2. **Community** - Scan QR code in README to join community group
3. **Email** - Send email to developer
4. When reporting, please provide: error information, operation steps, system version

## Contribution Guide

Issues and Pull Requests are welcome!

### Development Workflow

1. **Fork Project** - Click Fork button in top right of [GitHub page](https://github.com/chaogei/windsurf-account-manager-simple)
2. **Clone Repository** - `git clone https://github.com/your-username/windsurf-account-manager-simple.git`
3. **Create Branch** - `git checkout -b feature/your-feature-name`
4. **Install Dependencies** - `npm install`
5. **Development and Debug** - `npm run tauri dev`
6. **Commit Changes** - `git commit -m "Add your feature"`
7. **Push Branch** - `git push origin feature/your-feature-name`
8. **Create PR** - Create Pull Request on GitHub page

### Code Standards

**Frontend Code**:
- Use TypeScript strict mode
- Follow Vue 3 Composition API style
- Component naming uses PascalCase
- Function naming uses camelCase
- Constant naming uses UPPER_SNAKE_CASE

**Backend Code**:
- Follow Rust official code style
- Use `cargo fmt` to format code
- Use `cargo clippy` to check code
- Function naming uses snake_case
- Type naming uses PascalCase

### Commit Message Convention

Commit message format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

Types (type):
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation update
- `style`: Code format adjustment
- `refactor`: Refactoring
- `test`: Test related
- `chore`: Build/tool related

Example:
```
feat(api): add GetTeamCreditEntries API support

- Add GetTeamCreditEntries command
- Add credit history dialog component
- Update API integration documentation

Closes #123
```

### Tauri ACL Permission Configuration

Tauri v2 uses ACL (Access Control List) system to manage permissions, each plugin command needs explicit authorization in capabilities.

**Permission Configuration File**: `src-tauri/capabilities/default.json`

**Common Permission Examples**:
```json
{
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "dialog:allow-open",      // Allow open file/folder selection dialog
    "dialog:allow-save",      // Allow save file dialog
    "dialog:allow-message",   // Allow show message dialog
    "dialog:allow-ask",       // Allow ask dialog
    "dialog:allow-confirm"    // Allow confirm dialog
  ]
}
```

**Permission Naming Format**:
- `plugin:command` - Plugin command
- `plugin:allow-command` - Allow execution of plugin command

**Notes**:
- If encountering `Command plugin:dialog|open not allowed by ACL` error, need to add corresponding permission in capabilities
- Permission name format must strictly follow specification

### Version Release Workflow

1. Update version number in `src-tauri/tauri.conf.json`
2. Run `node scripts/sync-version.js` to sync other files
3. Update version history in README.md
4. Commit changes and create tag:
   ```bash
   git add .
   git commit -m "release: version x.x.x"
   git tag -a vx.x.x -m "Release version x.x.x"
   git push origin main
   git push origin vx.x.x
   ```
5. Create Release on GitHub

## License

AGPL-3.0

## Disclaimer

This tool is for learning and personal use only. Please comply with Windsurf Terms of Service. The author is not responsible for any issues caused by using this tool.

---

## Project Summary

Windsurf Account Manager - Simple is a fully-featured Windsurf multi-account management desktop application developed with Tauri + Vue 3 + Rust technology stack. The project started development in late 2025 and after multiple version iterations, has achieved the following core features:

### Core Features
- ✅ Complete account management (CRUD, grouping, tags)
- ✅ Credit reset (single account, batch, auto)
- ✅ Team management (member management, invite, remove)
- ✅ One-click switching (standard switching, seamless switching patch)
- ✅ Billing query (subscription status, payment info)
- ✅ Usage analytics (daily statistics, tool usage, model usage)
- ✅ Subscription payment (virtual card generation, private payment)
- ✅ Data security (AES-256-GCM encryption, system keychain)

### Technical Highlights
- 🏗️ Clear layered architecture (command layer, service layer, data layer)
- 🔒 Complete security mechanism (encrypted storage, authentication flow, privacy protection)
- ⚡ High performance optimization (async concurrency, token reuse, lightweight API)
- 🎯 Multi-client support (Windsurf Standard, Windsurf - Next)
- 🤖 Complete Devin account system support (auth1_token, session_token)

### Version Evolution
- **v1.6.x**: Basic feature completion, subscription payment, analytics optimization
- **v1.7.0**: Billing strategy and quota detail support, OperationType extension
- **v1.7.1**: Multi-client support (Windsurf and Windsurf - Next)
- **v1.7.2**: Complete Devin account system support
- **v1.7.3**: Devin refresh logic fix, key field backfill
- **v1.7.4**: Auto-update integration (tauri-plugin-updater) with signed releases and unified latest.json
- **v1.7.5**: Firebase↔Devin account auth-provider conversion, merged Devin email-code cards (mode-card grid), WindsurfPostAuth 404 retry, QUOTA field backfill across Devin register/login/import + Firebase refresh_token paths

### Future Plans
- [ ] Support Linux and macOS platforms
- [ ] Add more data analytics charts
- [ ] Support custom API endpoints
- [ ] Add automated testing
- [ ] Optimize UI/UX design

---

## Acknowledgments

Thanks to the following open source projects and services:

- [Tauri](https://tauri.app/) - Modern desktop application development framework
- [Vue.js](https://vuejs.org/) - Progressive JavaScript framework
- [Element Plus](https://element-plus.org/) - Vue 3 based component library
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Reqwest](https://docs.rs/reqwest/) - Rust HTTP client
- [Windsurf](https://windsurf.com/) - AI programming assistant

Thanks to all contributors and users for their feedback and support!

---

## Contact

- **GitHub**: [Project Repository](https://github.com/chaogei/windsurf-account-manager-simple)
- **Author**: chaogei666
- **License**: AGPL-3.0

---

**Last Updated**: 2026-04-20
**Document Version**: 1.7.3
