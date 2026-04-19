"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import WaterfallCanvas from "../component/WaterfallCanvas";

type RecordMessage = {
    type: "record";
    source_id: string;
    timestamp_ms: number;
    center_hz: number;
    lower_edge_hz: number;
    upper_edge_hz: number;
    avg_power: number;
    noise_floor: number;
    peak_power: number;
    peak_bin: number;
    estimated_peak_hz: number;
};

type HitMessage = {
    type: "hit";
    source_id: string;
    timestamp_ms: number;
    center_hz: number;
    peak_hz: number;
    lower_edge_hz: number;
    upper_edge_hz: number;
    peak_bin: number;
    peak_power: number;
    noise_floor: number;
    avg_power: number;
    snr_db: number;
};

type WaterfallMessage = {
    type: "waterfall_frame";
    source_id: string;
    timestamp_ms: number;
    center_hz: number;
    lower_edge_hz: number;
    upper_edge_hz: number;
    bins: number[];
};

type ScannerStatus = {
    is_running: boolean;
};

function formatHz(hz: number): string {
    if (hz >= 1_000_000) return `${(hz / 1_000_000).toFixed(3)} MHz`;
    if (hz >= 1_000) return `${(hz / 1_000).toFixed(3)} kHz`;
    return `${hz.toFixed(0)} Hz`;
}

function sameFrame(hit: HitMessage, frame: WaterfallMessage): boolean {
    return (
        hit.source_id === frame.source_id &&
        hit.timestamp_ms === frame.timestamp_ms &&
        Math.abs(hit.center_hz - frame.center_hz) < 1
    );
}

export default function WaterfallPage() {
    const [recordSocketState, setRecordSocketState] = useState<"connecting" | "open" | "closed">("connecting");
    const [hitSocketState, setHitSocketState] = useState<"connecting" | "open" | "closed">("connecting");
    const [waterfallSocketState, setWaterfallSocketState] = useState<"connecting" | "open" | "closed">("connecting");

    const [latestRecord, setLatestRecord] = useState<RecordMessage | null>(null);
    const [recentHits, setRecentHits] = useState<HitMessage[]>([]);
    const [latestWaterfallFrame, setLatestWaterfallFrame] = useState<WaterfallMessage | null>(null);

    const [renderPaused, setRenderPaused] = useState(false);
    const [scannerRunning, setScannerRunning] = useState(true);
    const [scannerBusy, setScannerBusy] = useState(false);

    const renderPausedRef = useRef(false);
    const latestIncomingFrameRef = useRef<WaterfallMessage | null>(null);
    const reconnectTimers = useRef<number[]>([]);

    useEffect(() => {
        renderPausedRef.current = renderPaused;

        if (!renderPaused && latestIncomingFrameRef.current) {
            setLatestWaterfallFrame(latestIncomingFrameRef.current);
        }
    }, [renderPaused]);

    const clearReconnectTimers = () => {
        reconnectTimers.current.forEach((id) => window.clearTimeout(id));
        reconnectTimers.current = [];
    };

    const setScannerState = useCallback(async (running: boolean) => {
        setScannerBusy(true);

        try {
            const endpoint = running ? "start" : "stop";
            const res = await fetch(`http://${window.location.hostname}:9001/api/scanner/${endpoint}`, {
                method: "POST",
            });

            const data = (await res.json()) as ScannerStatus;
            setScannerRunning(data.is_running);
        } catch (err) {
            console.error("failed to update scanner state", err);
        } finally {
            setScannerBusy(false);
        }
    }, []);

    useEffect(() => {
        const host = window.location.hostname;
        const wsBase = `ws://${host}:9001`;
        let cancelled = false;

        const connectRecordSocket = () => {
            if (cancelled) return null;

            setRecordSocketState("connecting");
            const ws = new WebSocket(`${wsBase}/ws/records`);

            ws.onopen = () => {
                if (!cancelled) setRecordSocketState("open");
            };

            ws.onerror = () => {
                if (!cancelled) setRecordSocketState("closed");
            };

            ws.onclose = () => {
                if (cancelled) return;

                setRecordSocketState("closed");
                const timer = window.setTimeout(connectRecordSocket, 1000);
                reconnectTimers.current.push(timer);
            };

            ws.onmessage = (event) => {
                try {
                    const msg = JSON.parse(event.data) as RecordMessage;
                    if (msg.type === "record") {
                        setLatestRecord(msg);
                    }
                } catch (error) {
                    console.error("Failed to parse record message", error);
                }
            };

            return ws;
        };

        const connectHitSocket = () => {
            if (cancelled) return null;

            setHitSocketState("connecting");
            const ws = new WebSocket(`${wsBase}/ws/hits`);

            ws.onopen = () => {
                if (!cancelled) setHitSocketState("open");
            };

            ws.onerror = () => {
                if (!cancelled) setHitSocketState("closed");
            };

            ws.onclose = () => {
                if (cancelled) return;

                setHitSocketState("closed");
                const timer = window.setTimeout(connectHitSocket, 1000);
                reconnectTimers.current.push(timer);
            };

            ws.onmessage = (event) => {
                try {
                    const msg = JSON.parse(event.data) as HitMessage;
                    if (msg.type === "hit") {
                        setRecentHits((prev) => [msg, ...prev].slice(0, 50));
                    }
                } catch (error) {
                    console.error("Failed to parse hit message", error);
                }
            };

            return ws;
        };

        const connectWaterfallSocket = () => {
            if (cancelled) return null;

            setWaterfallSocketState("connecting");
            const ws = new WebSocket(`${wsBase}/ws/waterfall`);

            ws.onopen = () => {
                if (!cancelled) setWaterfallSocketState("open");
            };

            ws.onerror = () => {
                if (!cancelled) setWaterfallSocketState("closed");
            };

            ws.onclose = () => {
                if (cancelled) return;

                setWaterfallSocketState("closed");
                const timer = window.setTimeout(connectWaterfallSocket, 1000);
                reconnectTimers.current.push(timer);
            };

            ws.onmessage = (event) => {
                try {
                    const parsed = JSON.parse(event.data) as WaterfallMessage;

                    if (parsed.type !== "waterfall_frame" || !Array.isArray(parsed.bins)) {
                        return;
                    }

                    latestIncomingFrameRef.current = parsed;

                    if (!renderPausedRef.current) {
                        setLatestWaterfallFrame(parsed);
                    }
                } catch (err) {
                    console.error("parse error", err);
                }
            };

            return ws;
        };

        const recordWs = connectRecordSocket();
        const hitWs = connectHitSocket();
        const waterfallWs = connectWaterfallSocket();

        (async () => {
            try {
                const res = await fetch(`http://${host}:9001/api/scanner/status`);
                const data = (await res.json()) as ScannerStatus;

                if (!cancelled) {
                    setScannerRunning(data.is_running);
                }
            } catch (err) {
                console.error("failed to fetch scanner status", err);
            }
        })();

        return () => {
            cancelled = true;
            clearReconnectTimers();
            recordWs?.close();
            hitWs?.close();
            waterfallWs?.close();
        };
    }, []);

    const strongestHit = useMemo(() => {
        if (recentHits.length === 0) return null;
        return [...recentHits].sort((a, b) => b.snr_db - a.snr_db)[0];
    }, [recentHits]);

    const overallConnectionState = useMemo(() => {
        if (
            recordSocketState === "open" ||
            hitSocketState === "open" ||
            waterfallSocketState === "open"
        ) {
            return "open";
        }

        if (
            recordSocketState === "connecting" ||
            hitSocketState === "connecting" ||
            waterfallSocketState === "connecting"
        ) {
            return "connecting";
        }

        return "closed";
    }, [recordSocketState, hitSocketState, waterfallSocketState]);

    const displayedFrameHits = useMemo(() => {
        if (!latestWaterfallFrame) {
            return [];
        }

        return recentHits.filter((hit) => sameFrame(hit, latestWaterfallFrame));
    }, [recentHits, latestWaterfallFrame]);

    return (
        <main className="min-h-screen bg-neutral-950 text-neutral-100">
            <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 p-6">
                <header className="rounded-2xl border border-neutral-800 bg-neutral-900 p-5 shadow-sm">
                    <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
                        <div>
                            <h1 className="text-2xl font-semibold tracking-tight">
                                Elux Waterfall
                            </h1>
                            <p className="mt-1 text-sm text-neutral-400">
                                Live RF view for spectra, peaks, and hits.
                            </p>
                        </div>

                        <div className="flex items-center gap-3">
                            <button
                                onClick={() => setRenderPaused((prev) => !prev)}
                                className="rounded-xl border border-neutral-700 bg-neutral-950 px-4 py-2 text-sm font-medium hover:bg-neutral-800"
                            >
                                {renderPaused ? "Resume View" : "Pause View"}
                            </button>

                            <button
                                onClick={() => setScannerState(!scannerRunning)}
                                disabled={scannerBusy}
                                className="rounded-xl border border-neutral-700 bg-neutral-950 px-4 py-2 text-sm font-medium hover:bg-neutral-800 disabled:opacity-50"
                            >
                                {scannerBusy ? "Working..." : scannerRunning ? "Stop Scanner" : "Start Scanner"}
                            </button>

                            <div className="rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm">
                                <span className="text-neutral-400">Connection: </span>
                                <span
                                    className={
                                        overallConnectionState === "open"
                                            ? "text-green-400"
                                            : overallConnectionState === "connecting"
                                                ? "text-yellow-400"
                                                : "text-red-400"
                                    }
                                >
                                    {overallConnectionState}
                                </span>
                            </div>
                        </div>
                    </div>

                    <div className="mt-5 grid gap-3 md:grid-cols-6">
                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">Scanner</div>
                            <div className="mt-1 text-sm font-medium">
                                {scannerRunning ? "Running" : "Stopped"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">Render</div>
                            <div className="mt-1 text-sm font-medium">
                                {renderPaused ? "Paused" : "Live"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">Source</div>
                            <div className="mt-1 text-sm font-medium">
                                {latestRecord?.source_id ?? "No source yet"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">Center</div>
                            <div className="mt-1 text-sm font-medium">
                                {latestRecord ? formatHz(latestRecord.center_hz) : "—"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">Window</div>
                            <div className="mt-1 text-sm font-medium">
                                {latestRecord
                                    ? `${formatHz(latestRecord.lower_edge_hz)} → ${formatHz(latestRecord.upper_edge_hz)}`
                                    : "—"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">Strongest Hit</div>
                            <div className="mt-1 text-sm font-medium">
                                {strongestHit
                                    ? `${formatHz(strongestHit.peak_hz)} @ ${strongestHit.snr_db.toFixed(1)} dB`
                                    : "—"}
                            </div>
                        </div>
                    </div>
                </header>

                <section className="grid gap-6 lg:grid-cols-[1.8fr_0.9fr]">
                    <div className="rounded-2xl border border-neutral-800 bg-neutral-900 p-4 shadow-sm">
                        <div className="mb-3 flex items-center justify-between">
                            <div>
                                <h2 className="text-lg font-semibold">Waterfall</h2>
                                <p className="text-sm text-neutral-400">
                                    {latestWaterfallFrame
                                        ? `FFT bins: ${latestWaterfallFrame.bins.length}`
                                        : "Waiting for real FFT bin stream from backend."}
                                </p>
                            </div>

                            <div className="text-right text-xs text-neutral-500">
                                <div>{renderPaused ? "frame frozen" : "live rendering"}</div>
                                <div>
                                    {latestRecord
                                        ? `peak ${formatHz(latestRecord.estimated_peak_hz)}`
                                        : "No record"}
                                </div>
                            </div>
                        </div>

                        <WaterfallCanvas
                            bins={latestWaterfallFrame?.bins ?? []}
                            hits={displayedFrameHits}
                            paused={renderPaused}
                        />

                        <div className="mt-3 grid grid-cols-3 gap-2 text-xs text-neutral-500">
                            <div className="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-2 text-left">
                                <div className="uppercase tracking-wide text-neutral-600">Lower Edge</div>
                                <div className="mt-1 text-sm text-neutral-300">
                                    {latestWaterfallFrame ? formatHz(latestWaterfallFrame.lower_edge_hz) : "—"}
                                </div>
                            </div>

                            <div className="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-2 text-center">
                                <div className="uppercase tracking-wide text-neutral-600">Center</div>
                                <div className="mt-1 text-sm text-neutral-300">
                                    {latestWaterfallFrame ? formatHz(latestWaterfallFrame.center_hz) : "—"}
                                </div>
                            </div>

                            <div className="rounded-lg border border-neutral-800 bg-neutral-950 px-3 py-2 text-right">
                                <div className="uppercase tracking-wide text-neutral-600">Upper Edge</div>
                                <div className="mt-1 text-sm text-neutral-300">
                                    {latestWaterfallFrame ? formatHz(latestWaterfallFrame.upper_edge_hz) : "—"}
                                </div>
                            </div>
                        </div>
                    </div>

                    <aside className="rounded-2xl border border-neutral-800 bg-neutral-900 p-4 shadow-sm">
                        <div className="mb-3">
                            <h2 className="text-lg font-semibold">Recent Hits</h2>
                            <p className="text-sm text-neutral-400">
                                Strong detections emitted by the backend detector.
                            </p>
                        </div>

                        <div className="flex max-h-480-px flex-col gap-3 overflow-y-auto pr-1">
                            {recentHits.length === 0 ? (
                                <div className="rounded-xl border border-dashed border-neutral-700 bg-neutral-950 p-4 text-sm text-neutral-500">
                                    No hits yet.
                                </div>
                            ) : (
                                recentHits.map((hit) => (
                                    <div
                                        key={`${hit.timestamp_ms}-${hit.peak_bin}-${hit.peak_hz}`}
                                        className="rounded-xl border border-neutral-800 bg-neutral-950 p-3"
                                    >
                                        <div className="flex items-start justify-between gap-3">
                                            <div>
                                                <div className="text-sm font-semibold">
                                                    {formatHz(hit.peak_hz)}
                                                </div>
                                                <div className="mt-1 text-xs text-neutral-500">
                                                    {hit.source_id}
                                                </div>
                                            </div>

                                            <div className="rounded-lg border border-neutral-800 px-2 py-1 text-xs font-medium text-green-300">
                                                {hit.snr_db.toFixed(1)} dB
                                            </div>
                                        </div>

                                        <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-neutral-400">
                                            <div><span className="text-neutral-500">Peak:</span> {hit.peak_power.toFixed(3)}</div>
                                            <div><span className="text-neutral-500">Floor:</span> {hit.noise_floor.toFixed(3)}</div>
                                            <div><span className="text-neutral-500">Avg:</span> {hit.avg_power.toFixed(3)}</div>
                                            <div><span className="text-neutral-500">Bin:</span> {hit.peak_bin}</div>
                                        </div>
                                    </div>
                                ))
                            )}
                        </div>
                    </aside>
                </section>
            </div>
        </main>
    );
}