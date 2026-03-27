use std::time::Duration;
use sysinfo::{System, Networks};
use tauri::{AppHandle, Emitter};

pub fn start_monitor(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut sys = System::new();
        // ネットワーク使用量の測定セットアップ
        let mut networks = Networks::new_with_refreshed_list();
        
        sys.refresh_cpu_usage();
        
        // 初回は短めにウェイト
        tokio::time::sleep(Duration::from_millis(400)).await;

        loop {
            // CPU使用率のリフレッシュ
            sys.refresh_cpu_usage();
            let cpu_usage = sys.global_cpu_usage() as f64;
            
            // ネットワーク通信量の測定
            // 差分だけをリフレッシュする
            networks.refresh(true);
            
            let mut total_bytes = 0_u64;
            for (_name, data) in &networks {
                total_bytes += data.transmitted();
                total_bytes += data.received();
            }
            
            // 2秒間隔での差分を取得しているため、1秒あたりの通信量(Byte/sec)を算出
            let bytes_per_sec = total_bytes as f64 / 2.0;

            // Mbps (メガビット/秒) に変換
            let mbps = (bytes_per_sec * 8.0) / 1_000_000.0;

            // JSONペイロードを作成
            let payload = serde_json::json!({
                "cpu": cpu_usage,
                "network": mbps
            });

            // フロントエンドへ送信
            let _ = app_handle.emit("performance-metrics", &payload);

            // DBへストック
            crate::database::insert_metric(&app_handle, cpu_usage, mbps);

            // 2秒待機
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}
