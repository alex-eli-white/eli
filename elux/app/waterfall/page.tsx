"use client";

import { useEffect, useMemo, useState } from "react";
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

function formatHz(hz: number): string {
    if (hz >= 1_000_000) {
        return `${(hz / 1_000_000).toFixed(3)} MHz`;
    }

    if (hz >= 1_000) {
        return `${(hz / 1_000).toFixed(3)} kHz`;
    }

    return `${hz.toFixed(0)} Hz`;
}

export default function WaterfallPage() {
    const [recordSocketState, setRecordSocketState] = useState<
        "connecting" | "open" | "closed"
    >("connecting");
    const [hitSocketState, setHitSocketState] = useState<
        "connecting" | "open" | "closed"
    >("connecting");
    const [waterfallSocketState, setWaterfallSocketState] = useState<
        "connecting" | "open" | "closed"
    >("connecting");

    const [latestRecord, setLatestRecord] = useState<RecordMessage | null>(null);
    const [recentHits, setRecentHits] = useState<HitMessage[]>([]);
    const [latestWaterfallFrame, setLatestWaterfallFrame] =
        useState<WaterfallMessage | null>(null);

    useEffect(() => {
        const wsBase = `ws://${window.location.hostname}:9001`;

        console.log("Connecting to websocket base:", wsBase);

        const recordWs = new WebSocket(`${wsBase}/ws/records`);
        const hitWs = new WebSocket(`${wsBase}/ws/hits`);
        const waterfallWs = new WebSocket(`${wsBase}/ws/waterfall`);

        recordWs.onopen = () => {
            console.log("records websocket open");
            setRecordSocketState("open");
        };

        recordWs.onclose = (event) => {
            console.log("records websocket closed", event.code, event.reason);
            setRecordSocketState("closed");
        };

        recordWs.onerror = () => {
            setRecordSocketState("closed");
        };

        recordWs.onmessage = (event) => {
            console.log("record raw message", event.data);

            try {
                const msg = JSON.parse(event.data) as RecordMessage;

                if (msg.type !== "record") {
                    return;
                }

                setLatestRecord(msg);
            } catch (error) {
                console.error("Failed to parse record message", error);
            }
        };

        hitWs.onopen = () => {
            console.log("hits websocket open");
            setHitSocketState("open");
        };

        hitWs.onclose = (event) => {
            console.log("hits websocket closed", event.code, event.reason);
            setHitSocketState("closed");
        };

        hitWs.onerror = () => {
            setHitSocketState("closed");
        };

        hitWs.onmessage = (event) => {
            console.log("hit raw message", event.data);

            try {
                const msg = JSON.parse(event.data) as HitMessage;

                if (msg.type !== "hit") {
                    return;
                }

                setRecentHits((prev) => [msg, ...prev].slice(0, 20));
            } catch (error) {
                console.error("Failed to parse hit message", error);
            }
        };

        waterfallWs.onopen = () => {
            console.log("waterfall websocket open");
            setWaterfallSocketState("open");
        };

        waterfallWs.onclose = (event) => {
            console.log("waterfall websocket closed", event.code, event.reason);
            setWaterfallSocketState("closed");
        };

        waterfallWs.onerror = () => {
            setWaterfallSocketState("closed");
        };

        waterfallWs.onmessage = (event) => {
            console.log("waterfall raw message", event.data);

            try {
                const msg = JSON.parse(event.data) as WaterfallMessage;

                if (msg.type !== "waterfall_frame") {
                    return;
                }

                setLatestWaterfallFrame(msg);
            } catch (error) {
                console.error("Failed to parse waterfall message", error);
            }
        };

        return () => {
            recordWs.close();
            hitWs.close();
            waterfallWs.close();
        };
    }, []);

    const strongestHit = useMemo(() => {
        if (recentHits.length === 0) {
            return null;
        }

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

                    <div className="mt-5 grid gap-3 md:grid-cols-4">
                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">
                                Source
                            </div>
                            <div className="mt-1 text-sm font-medium">
                                {latestRecord?.source_id ?? "No source yet"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">
                                Center
                            </div>
                            <div className="mt-1 text-sm font-medium">
                                {latestRecord ? formatHz(latestRecord.center_hz) : "—"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">
                                Window
                            </div>
                            <div className="mt-1 text-sm font-medium">
                                {latestRecord
                                    ? `${formatHz(latestRecord.lower_edge_hz)} → ${formatHz(latestRecord.upper_edge_hz)}`
                                    : "—"}
                            </div>
                        </div>

                        <div className="rounded-xl border border-neutral-800 bg-neutral-950 p-3">
                            <div className="text-xs uppercase tracking-wide text-neutral-500">
                                Strongest Hit
                            </div>
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
                                        ? `Live FFT bins: ${latestWaterfallFrame.bins.length}`
                                        : "Waiting for real FFT bin stream from backend."}
                                </p>
                            </div>

                            <div className="text-xs text-neutral-500">
                                {latestRecord
                                    ? `peak ${formatHz(latestRecord.estimated_peak_hz)}`
                                    : "No record"}
                            </div>
                        </div>

                        <WaterfallCanvas bins={latestWaterfallFrame?.bins ?? []} />
                    </div>

                    <aside className="rounded-2xl border border-neutral-800 bg-neutral-900 p-4 shadow-sm">
                        <div className="mb-3">
                            <h2 className="text-lg font-semibold">Recent Hits</h2>
                            <p className="text-sm text-neutral-400">
                                Strong detections emitted by the backend detector.
                            </p>
                        </div>

                        <div className="flex max-h-[480px] flex-col gap-3 overflow-y-auto pr-1">
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
                                            <div>
                                                <span className="text-neutral-500">Peak:</span>{" "}
                                                {hit.peak_power.toFixed(3)}
                                            </div>
                                            <div>
                                                <span className="text-neutral-500">Floor:</span>{" "}
                                                {hit.noise_floor.toFixed(3)}
                                            </div>
                                            <div>
                                                <span className="text-neutral-500">Avg:</span>{" "}
                                                {hit.avg_power.toFixed(3)}
                                            </div>
                                            <div>
                                                <span className="text-neutral-500">Bin:</span>{" "}
                                                {hit.peak_bin}
                                            </div>
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