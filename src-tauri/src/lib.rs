pub mod monitor;
pub mod database;

use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            // データベースの初期化
            crate::database::init_db(app.handle()).expect("Failed to initialize database");
            
            // アプリ起動時にモニタリングプロセスを裏側で発火させる
            monitor::start_monitor(app.handle().clone());

            // システムトレイの右クリックメニュー
            use tauri::menu::{Menu, MenuItem};
            let hide_item = MenuItem::with_id(app, "hide", "トレイにしまう", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&hide_item, &quit_item])?;

            // システムトレイアイコンの設定
            let tray = tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Performance Insights")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "hide" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.hide();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::TrayIconEvent;
                    if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;
            
            // 起動直後はウィンドウを表示（ユーザーが存在を確認できるように）
            drop(tray);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, crate::database::get_history, crate::database::get_history_aggregated])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
