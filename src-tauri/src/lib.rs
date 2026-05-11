mod models;
mod repository;
mod services;
mod commands;
mod utils;

use repository::DataStore;
use commands::{AutoResetStore, ResetRecordStore};
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // 自动更新能力（静默检查 + 下载 + 安装）：
        // - endpoints / pubkey 在 tauri.conf.json 的 plugins.updater 中配置
        // - 前端通过 @tauri-apps/plugin-updater 触发 check / downloadAndInstall
        .plugin(tauri_plugin_updater::Builder::new().build())
        // 更新完成后需要调用 process.relaunch() 重启应用以加载新版本
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // 初始化数据存储
            let store = DataStore::new(app.handle())
                .expect("Failed to initialize data store");
            let store = Arc::new(store);
            
            // 将数据存储注入到应用状态
            app.manage(store.clone());
            
            // 初始化自动重置配置存储
            let auto_reset_store = AutoResetStore::new(app.handle())
                .expect("Failed to initialize auto reset store");
            app.manage(Arc::new(auto_reset_store));
            
            // 初始化重置记录存储
            let reset_record_store = ResetRecordStore::new(app.handle())
                .expect("Failed to initialize reset record store");
            app.manage(Arc::new(reset_record_store));
            
            // 初始化代理配置
            let store_for_proxy = store.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(settings) = store_for_proxy.get_settings().await {
                    if settings.proxy_enabled || settings.proxy_url.is_some() {
                        println!("[Init] Loading proxy config: enabled={}, url={:?}", 
                            settings.proxy_enabled, settings.proxy_url);
                        services::update_proxy_config(
                            settings.proxy_enabled,
                            settings.proxy_url
                        );
                    }
                }
            });
            
            // 启动自动备份任务
            let store_for_backup = store.clone();
            tauri::async_runtime::spawn(async move {
                println!("[Backup] 自动备份任务已启动");
                
                // 首次备份延迟1分钟执行
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                
                loop {
                    // 获取当前备份设置
                    let (enabled, interval_minutes) = match store_for_backup.get_settings().await {
                        Ok(settings) => (settings.auto_backup_enabled, settings.backup_interval),
                        Err(_) => (true, 10), // 默认值
                    };
                    
                    // 执行备份（如果启用）
                    if enabled {
                        match store_for_backup.create_timestamped_backup().await {
                            Ok(path) => println!("[Backup] 自动备份成功: {:?}", path.file_name()),
                            Err(e) => println!("[Backup] 自动备份失败: {}", e),
                        }
                    }
                    
                    // 等待下一次备份
                    let interval = std::time::Duration::from_secs(interval_minutes.max(1) as u64 * 60);
                    tokio::time::sleep(interval).await;
                }
            });
            
            // 获取版本号并设置窗口标题
            let version = app.package_info().version.to_string();
            if let Some(window) = app.get_webview_window("main") {
                let title = format!("windsurf-account-manager-simple v{}", version);
                window.set_title(&title).ok();
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 账号管理命令
            commands::add_account,
            commands::add_account_by_refresh_token,
            commands::get_all_accounts,
            commands::get_account,
            commands::update_account,
            commands::delete_account,
            commands::delete_accounts_batch,
            commands::search_accounts,
            commands::filter_accounts_by_group,
            commands::filter_accounts_by_tags,
            // v1.7.8 方案 B：分页查询 + 聚合统计
            commands::get_accounts_page,
            commands::get_account_aggregates,
            commands::get_all_account_ids,
            commands::get_accounts_by_ids,
            commands::batch_update_group_by_ids,
            
            // API操作命令
            commands::login_account,
            commands::refresh_token,
            commands::get_plan_status,
            commands::reset_credits,
            commands::update_seats,
            commands::get_billing,
            commands::update_plan,
            commands::cancel_subscription,
            commands::resume_subscription,
            commands::get_account_info,
            commands::get_current_user,
            commands::batch_reset_credits,
            commands::batch_refresh_tokens,
            commands::get_team_credit_entries,
            commands::get_trial_payment_link,
            commands::get_team_config,
            commands::update_team_config,
            commands::get_cascade_model_configs,
            commands::get_command_model_configs,
            commands::get_team_organizational_controls,
            commands::upsert_team_organizational_controls,
            commands::get_available_mcp_plugins,
            commands::delete_windsurf_user,
            // Pro 试用资格检查
            commands::check_pro_trial_eligibility,
            commands::get_account_valid_token,
            // 用户API密钥管理
            commands::get_api_key_summary,
            commands::delete_api_key,
            commands::register_user_api_key,
            // 第三方API Provider Key管理
            commands::get_set_user_api_provider_keys,
            commands::set_user_api_provider_key,
            commands::delete_user_api_provider_key,
            // 迁移 / 开发者主密钥 / 排行榜
            commands::migrate_api_key,
            commands::get_primary_api_key_for_devs,
            commands::get_global_leaderboard_api_key,
            commands::get_leaderboard,
            
            // 支付相关命令
            commands::generate_virtual_card,
            commands::open_payment_window,
            commands::inject_card_info,
            commands::validate_card_number,
            commands::auto_fill_payment_form,
            commands::get_trial_payment_link_enhanced,
            commands::open_external_link,
            commands::open_external_link_incognito,
            commands::inject_auto_submit_script,
            commands::close_payment_window,
            commands::get_success_bins,
            commands::add_success_bin,
            commands::clear_success_bins,
            commands::get_random_success_bin,
            commands::reset_test_mode_progress,
            commands::get_test_mode_progress,
            
            // Protobuf解析API命令（返回解析后的数据）
            commands::get_current_user_parsed,
            commands::get_billing_parsed,
            commands::batch_get_users_parsed,

            // Analytics 分析命令
            commands::get_account_analytics,

            // 设置管理命令
            commands::get_settings,
            commands::update_settings,
            commands::get_groups,
            commands::add_group,
            commands::delete_group,
            commands::rename_group,
            commands::get_tags,
            commands::add_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::batch_update_account_tags,
            commands::get_logs,
            commands::clear_logs,
            commands::get_stats,
            commands::export_data,
            
            // 切号相关命令
            commands::switch_account,
            commands::reset_machine_id,
            commands::check_admin_privileges,
            
            // Windsurf信息命令
            commands::get_current_windsurf_info,
            
            // 应用信息命令
            commands::get_app_version,
            commands::get_app_title,
            commands::reset_http_client,
            
            // 无感换号补丁命令
            commands::get_windsurf_path,
            commands::apply_seamless_patch,
            commands::restore_seamless_patch,
            commands::check_patch_status,
            commands::validate_windsurf_path,
            
            // 伟哥(寸止)命令
            commands::check_cunzhi_status,
            commands::install_cunzhi,
            commands::uninstall_cunzhi,
            
            // 数据备份命令
            commands::create_backup,
            commands::list_backups,
            commands::restore_backup,
            commands::delete_backup,
            commands::export_data_to_file,
            commands::import_data_from_file,
            commands::get_data_directory,
            
            // 排序命令
            commands::get_sorted_accounts,
            commands::update_accounts_order,
            commands::update_sort_config,
            commands::get_sort_config,
            
            // 团队管理命令
            commands::get_team_members,
            commands::invite_team_members,
            commands::remove_team_member,
            commands::revoke_invitation,
            commands::get_pending_invitations,
            commands::get_my_pending_invitation,
            commands::accept_invitation,
            commands::reject_invitation,
            commands::request_team_access,
            commands::approve_team_join_request,
            // 自动充值管理
            commands::get_credit_top_up_settings,
            commands::update_credit_top_up_settings,
            // 成员权限管理
            commands::update_codeium_access,
            commands::add_user_role,
            commands::remove_user_role,
            commands::transfer_subscription,
            
            // 自动重置命令
            commands::get_auto_reset_configs,
            commands::add_auto_reset_config,
            commands::update_auto_reset_config,
            commands::delete_auto_reset_config,
            commands::check_and_auto_reset,
            commands::force_reset_config,
            commands::get_reset_records,
            commands::get_reset_stats,
            commands::clear_reset_records,

            // 额度耗尽自动切换命令（Fork 新增）
            commands::check_and_auto_switch,

            // Devin Session 账密登录
            commands::devin_check_connections,
            commands::devin_password_login,
            commands::devin_windsurf_post_auth,
            commands::add_account_by_devin_login,
            commands::add_account_by_devin_native_login,
            commands::add_account_by_devin_with_org,
            commands::refresh_devin_session,
            commands::add_account_by_devin_session_token,
            commands::add_account_by_devin_auth1_token,
            // Firebase ↔ Devin 账号互转
            commands::convert_account_to_devin,
            commands::convert_account_to_firebase,

            // 登录流派智能嗅探（方案 B：自动嗅探 + 统一入口）
            commands::devin_check_user_login_method,
            commands::sniff_login_method,

            // Devin 邮箱注册 / 无密码邮件登录 / 忘记密码（Windsurf 侧 _devin-auth 通道）
            commands::devin_email_start,
            commands::devin_email_complete,
            commands::devin_password_reset_start,
            commands::devin_password_reset_complete,
            commands::add_account_by_devin_register,
            commands::add_account_by_devin_email_login,

            // Devin 原生站点（app.devin.ai）注册通道：独立端口 /api/auth1/*，注册后自动桥接 Windsurf
            commands::devin_app_check_connections,
            commands::devin_app_email_start,
            commands::devin_app_email_complete,
            commands::add_account_by_devin_native_register,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // v1.7.8 批量导入性能修复配套：退出前强制 flush DataStore 的 coalesced saves
            //
            // 背景：批量导入等批量场景改用 `request_save_coalesced` 非阻塞合并写盘，
            // tauri command 返回时数据可能还在内存中。正常退出路径下 ExitRequested
            // 事件是我们同步落盘的最后机会——阻塞 block_on 执行 flush_pending_saves
            // 确保两个配置文件（accounts.json / logs.json）都已原子写入磁盘。
            //
            // 进程异常终止（kill -9、断电等）场景下本 hook 不会被调用，数据可能丢失
            // 最后几百 ms 的批量改动——这是 coalescer 架构的已知权衡。
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let store = app_handle.state::<Arc<DataStore>>();
                tauri::async_runtime::block_on(async {
                    if let Err(e) = store.flush_pending_saves().await {
                        eprintln!("[lib::run] flush_pending_saves on exit failed: {}", e);
                    }
                });
            }
        });
}
