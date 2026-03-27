import { useState, useEffect } from "react";
import { LineChart, Line, YAxis, XAxis, Tooltip, ResponsiveContainer } from "recharts";
import { Activity, Clock, CalendarDays } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

interface PerformanceMetrics {
  cpu: number;
  network: number;
}

interface DataPoint {
  time: number | string;
  cpu: number;
  network: number;
}

type ViewMode = "live" | "24h" | "week";

function App() {
  const [viewMode, setViewMode] = useState<ViewMode>("live");
  const [liveData, setLiveData] = useState<DataPoint[]>(
    Array.from({ length: 60 }, (_, i) => ({ time: i, cpu: 0, network: 0 }))
  );
  const [historyData, setHistoryData] = useState<DataPoint[]>([]);

  // 起動時に直近60件の履歴を読み込む（liveビュー用）
  useEffect(() => {
    async function fetchInit() {
      try {
        const str = await invoke<string>("get_history", { limit: 60 });
        const history: DataPoint[] = JSON.parse(str);
        if (history.length > 0) setLiveData(history);
      } catch (e) { console.error(e); }
    }
    fetchInit();
  }, []);

  // viewModeが24h/7dの時は集計データを取得
  useEffect(() => {
    if (viewMode === "live") return;
    async function fetchHistory() {
      try {
        const str = await invoke<string>("get_history_aggregated", { range: viewMode === "week" ? "7d" : viewMode });
        const data: DataPoint[] = JSON.parse(str);
        setHistoryData(data);
      } catch (e) {
        console.error(e);
        setHistoryData([]);
      }
    }
    fetchHistory();
    // 5分おきに更新
    const timer = setInterval(fetchHistory, 5 * 60 * 1000);
    return () => clearInterval(timer);
  }, [viewMode]);

  // リアルタイムストリーム
  useEffect(() => {
    let unlisten: () => void;
    let timeIndex = Date.now();
    async function setup() {
      unlisten = await listen<PerformanceMetrics>("performance-metrics", (event) => {
        setLiveData((prev) => {
          const next = prev.length >= 60 ? [...prev.slice(prev.length - 59)] : [...prev];
          next.push({ time: timeIndex++, cpu: event.payload.cpu, network: event.payload.network });
          return next;
        });
      });
    }
    setup();
    return () => { if (unlisten) unlisten(); };
  }, []);

  const currentData = viewMode === "live" ? liveData : historyData;
  const currentCpu = liveData[liveData.length - 1].cpu;
  const isHighLoad = currentCpu > 80;

  const tabs: { key: ViewMode; label: string; icon: React.ReactNode }[] = [
    { key: "live", label: "LIVE", icon: <Activity className="w-3 h-3" /> },
    { key: "24h", label: "24H", icon: <Clock className="w-3 h-3" /> },
    { key: "week", label: "WEEK", icon: <CalendarDays className="w-3 h-3" /> },
  ];

  return (
    <div
      className={`w-full h-screen p-3 flex flex-col gap-2 transition-colors duration-1000 select-none relative
        ${isHighLoad ? "bg-red-950/90 backdrop-blur-md" : "bg-black/80 backdrop-blur-md"}`}
    >
      {/* ドラッグカバー（最前面・全面） */}
      <div
        className="absolute inset-0 z-50 cursor-move"
        onMouseDown={async (e) => {
          if (e.button === 0) await getCurrentWindow().startDragging();
        }}
      />

      {/* Header */}
      <div className="flex justify-between items-center pointer-events-none z-10 relative">
        <div className="flex items-center gap-2">
          <Activity className={`w-4 h-4 ${isHighLoad ? "text-red-400 animate-pulse" : "text-cyan-400"}`} />
          <span className="text-white/80 text-xs font-semibold tracking-wider">PERFORMANCE INSIGHTS</span>
        </div>
        <div className="flex gap-3">
          <div className="text-right">
            <div className="text-white/40 text-[10px]">CPU</div>
            <div className={`text-base font-black ${isHighLoad ? "text-red-400 animate-pulse" : "text-cyan-400"}`}>
              {Math.round(currentCpu)}%
            </div>
          </div>
          <div className="text-right">
            <div className="text-white/40 text-[10px]">BANDWIDTH</div>
            <div className="text-base font-black text-emerald-400">
              {liveData[liveData.length - 1].network.toFixed(1)} Mbps
            </div>
          </div>
        </div>
      </div>

      {/* タブ切り替え（z-indexをドラッグカバーより上に設定） */}
      <div className="flex gap-1 z-[60] relative pointer-events-auto">
        {tabs.map((t) => (
          <button
            key={t.key}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={() => setViewMode(t.key)}
            className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-bold transition-colors
              ${viewMode === t.key
                ? "bg-cyan-500/30 text-cyan-300 border border-cyan-500/50"
                : "text-white/40 hover:text-white/70"}`}
          >
            {t.icon}{t.label}
          </button>
        ))}
        {viewMode !== "live" && historyData.length === 0 && (
          <span className="text-white/30 text-[10px] self-center ml-2">データ蓄積待ち…</span>
        )}
      </div>

      {/* グラフ */}
      <div className="flex-1 z-10 relative pointer-events-none">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={currentData}>
            {viewMode !== "live" && (
              <XAxis
                dataKey="time"
                tick={{ fill: "rgba(255,255,255,0.3)", fontSize: 9 }}
                tickFormatter={(v) => {
                  if (!v) return "";
                  const d = new Date(v);
                  if (viewMode === "week") return `${d.getMonth()+1}/${d.getDate()}`;
                  return `${d.getHours()}:${String(d.getMinutes()).padStart(2,"0")}`;
                }}
                interval="preserveStartEnd"
              />
            )}
            <YAxis yAxisId="left" domain={[0, 100]} hide />
            <YAxis yAxisId="right" orientation="right" domain={["auto", "auto"]} hide />
            {viewMode !== "live" && (
              <Tooltip
                contentStyle={{ background: "#111", border: "1px solid rgba(255,255,255,0.1)", fontSize: 11 }}
                labelStyle={{ color: "rgba(255,255,255,0.5)" }}
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                formatter={(v: any, name: any) => {
                  return name === "cpu" ? [`${Math.round(Number(v))}%`, "CPU"] : [`${Number(v).toFixed(1)} Mbps`, "帯域"];
                }}
              />
            )}
            <Line yAxisId="left" type="monotone" dataKey="cpu"
              stroke={isHighLoad && viewMode === "live" ? "#f87171" : "#22d3ee"}
              strokeWidth={viewMode === "live" ? 2 : 1.5} dot={false} isAnimationActive={viewMode === "live"} />
            <Line yAxisId="right" type="monotone" dataKey="network"
              stroke="#34d399" strokeWidth={viewMode === "live" ? 1.5 : 1} dot={false} isAnimationActive={viewMode === "live"} />
          </LineChart>
        </ResponsiveContainer>

        {isHighLoad && viewMode === "live" && (
          <div className="absolute inset-0 flex items-center justify-center">
            <span className="text-red-400/20 text-4xl font-black tracking-widest">OVERLOAD</span>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
