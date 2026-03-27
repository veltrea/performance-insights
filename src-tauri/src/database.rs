use rusqlite::{Connection, Result};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct DbState {
    pub conn: Mutex<Connection>,
}

pub fn init_db(app_handle: &AppHandle) -> Result<()> {
    // Windows環境でも安全に保存できるようAppDataディレクトリを使用
    let app_dir = app_handle.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&app_dir).ok();
    let db_path = app_dir.join("metrics_history.db");

    let conn = Connection::open(db_path)?;

    // メトリクス保存用テーブル作成
    conn.execute(
        "CREATE TABLE IF NOT EXISTS metrics (
            id INTEGER PRIMARY KEY,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            cpu REAL NOT NULL,
            network REAL NOT NULL
        )",
        [],
    )?;

    // 古いデータを削除するなどのパージや丸め処理も今後ここに追加可能

    app_handle.manage(DbState {
        conn: Mutex::new(conn),
    });

    Ok(())
}

pub fn insert_metric(app_handle: &AppHandle, cpu: f64, network: f64) {
    if let Some(state) = app_handle.try_state::<DbState>() {
        let conn = state.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO metrics (cpu, network) VALUES (?1, ?2)",
            &[&cpu, &network],
        );
    }
}

// React側から履歴データ（SQLiteの全ログ）をフェッチするためのコマンド
#[tauri::command]
pub fn get_history(app_handle: tauri::AppHandle, limit: usize) -> std::result::Result<String, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.conn.lock().unwrap();
    
    // SQLiteから最新の `limit` 件を取得
    let mut stmt = conn.prepare("SELECT id, cpu, network FROM metrics ORDER BY id DESC LIMIT ?1").map_err(|e| e.to_string())?;
    
    let metrics_iter = stmt.query_map([limit as i64], |row| {
        Ok(serde_json::json!({
            "time": row.get::<_, i64>(0)?,
            "cpu": row.get::<_, f64>(1)?,
            "network": row.get::<_, f64>(2)?
        }))
    }).map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for metric in metrics_iter {
        if let Ok(m) = metric {
            result.push(m);
        }
    }
    
    // グラフの左から右へ流れるようにするために結果を反転（古い順にする）
    result.reverse();
    
    // 文字列(JSON)として返す
    Ok(serde_json::to_string(&result).unwrap())
}

// 24時間または7日間の集計グラフデータを取得するコマンド
// range: "24h" または "7d"
#[tauri::command]
pub fn get_history_aggregated(app_handle: tauri::AppHandle, range: String) -> std::result::Result<String, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.conn.lock().unwrap();

    // 時間範囲とグルーピング間隔をrange指定から決定
    let (hours_back, group_minutes) = match range.as_str() {
        "7d" => (7 * 24, 120),  // 7日間: 2時間平均
        _    => (24, 30),        // 24時間（デフォルト）: 30分平均
    };

    // SQLiteのstrftime でグループ化して平均値を計算する
    let sql = format!(
        "SELECT 
            strftime('%Y-%m-%dT%H:%M', timestamp, '-' || (strftime('%M', timestamp) % {gm}) || ' minutes') as bucket,
            AVG(cpu) as cpu,
            AVG(network) as network
         FROM metrics
         WHERE timestamp >= datetime('now', '-{h} hours')
         GROUP BY bucket
         ORDER BY bucket ASC",
        gm = group_minutes,
        h  = hours_back
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let metrics_iter = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "time": row.get::<_, String>(0)?,
            "cpu":  row.get::<_, f64>(1)?,
            "network": row.get::<_, f64>(2)?
        }))
    }).map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for metric in metrics_iter {
        if let Ok(m) = metric {
            result.push(m);
        }
    }

    Ok(serde_json::to_string(&result).unwrap())
}
