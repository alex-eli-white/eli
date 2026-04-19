"use client";

import { useEffect, useRef } from "react";

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

type WaterfallCanvasProps = {
    width?: number;
    height?: number;
    bins: number[];
    hits?: HitMessage[];
    paused?: boolean;
};

function clamp01(value: number): number {
    return Math.max(0, Math.min(1, value));
}

function lerp(a: number, b: number, t: number): number {
    return a + (b - a) * t;
}

function colorMap(t: number): [number, number, number] {
    const x = clamp01(t);

    if (x < 0.2) {
        const k = x / 0.2;
        return [
            Math.round(lerp(0, 10, k)),
            Math.round(lerp(0, 20, k)),
            Math.round(lerp(0, 80, k)),
        ];
    }

    if (x < 0.45) {
        const k = (x - 0.2) / (0.45 - 0.2);
        return [
            Math.round(lerp(10, 0, k)),
            Math.round(lerp(20, 180, k)),
            Math.round(lerp(80, 200, k)),
        ];
    }

    if (x < 0.7) {
        const k = (x - 0.45) / (0.7 - 0.45);
        return [
            Math.round(lerp(0, 255, k)),
            Math.round(lerp(180, 220, k)),
            Math.round(lerp(200, 0, k)),
        ];
    }

    const k = (x - 0.7) / (1.0 - 0.7);
    return [
        Math.round(lerp(255, 255, k)),
        Math.round(lerp(220, 255, k)),
        Math.round(lerp(0, 255, k)),
    ];
}

function percentile(sorted: number[], q: number): number {
    if (sorted.length === 0) {
        return 0;
    }

    const idx = Math.min(
        sorted.length - 1,
        Math.max(0, Math.floor(q * (sorted.length - 1)))
    );

    return sorted[idx];
}

function drawPausedOverlay(ctx: CanvasRenderingContext2D) {
    ctx.fillStyle = "rgba(0, 0, 0, 0.55)";
    ctx.fillRect(10, 10, 82, 24);
    ctx.fillStyle = "#ffffff";
    ctx.font = "12px sans-serif";
    ctx.fillText("PAUSED", 22, 26);
}

function normalizeWithRolling(
    inputBins: number[],
    rollingMin: number[],
    rollingMax: number[],
    rollingWindow: number
): number[] {
    if (inputBins.length === 0) {
        return [];
    }

    const dbVals = inputBins.map((v) => 10 * Math.log10(Math.max(v, 1e-12)));
    const sorted = [...dbVals].sort((a, b) => a - b);

    const low = percentile(sorted, 0.1);
    const high = percentile(sorted, 0.995);

    rollingMin.push(low);
    rollingMax.push(high);

    if (rollingMin.length > rollingWindow) {
        rollingMin.shift();
    }

    if (rollingMax.length > rollingWindow) {
        rollingMax.shift();
    }

    const globalMin = Math.min(...rollingMin);
    const globalMax = Math.max(...rollingMax);
    const span = Math.max(globalMax - globalMin, 1e-6);

    return dbVals.map((db) => {
        const n = (db - globalMin) / span;
        return Math.pow(clamp01(n), 1.4);
    });
}

export default function WaterfallCanvas({
                                            width = 768,
                                            height = 480,
                                            bins,
                                            hits = [],
                                            paused = false,
                                        }: WaterfallCanvasProps) {
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const rollingMinRef = useRef<number[]>([]);
    const rollingMaxRef = useRef<number[]>([]);
    const rollingWindow = 40;

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) {
            return;
        }

        const ctx = canvas.getContext("2d");
        if (!ctx) {
            return;
        }

        ctx.fillStyle = "black";
        ctx.fillRect(0, 0, width, height);

        if (paused) {
            drawPausedOverlay(ctx);
        }
    }, [width, height, paused]);

    useEffect(() => {
        if (paused) {
            return;
        }

        if (bins.length === 0) {
            return;
        }

        const canvas = canvasRef.current;
        if (!canvas) {
            return;
        }

        const ctx = canvas.getContext("2d");
        if (!ctx) {
            return;
        }

        const normalizedBins = normalizeWithRolling(
            bins,
            rollingMinRef.current,
            rollingMaxRef.current,
            rollingWindow
        );

        if (normalizedBins.length === 0) {
            return;
        }

        ctx.drawImage(canvas, 0, 0, width, height - 1, 0, 1, width, height - 1);

        const row = ctx.createImageData(width, 1);

        for (let x = 0; x < width; x += 1) {
            const sourceIndex = Math.floor((x / width) * normalizedBins.length);
            const value = normalizedBins[Math.min(sourceIndex, normalizedBins.length - 1)];
            const [r, g, b] = colorMap(value);

            const idx = x * 4;
            row.data[idx] = r;
            row.data[idx + 1] = g;
            row.data[idx + 2] = b;
            row.data[idx + 3] = 255;
        }

        ctx.putImageData(row, 0, 0);

        for (const hit of hits) {
            const x = Math.round(
                (hit.peak_bin / Math.max(normalizedBins.length - 1, 1)) * (width - 1)
            );
            const clampedX = Math.max(0, Math.min(width - 1, x));

            ctx.strokeStyle = "rgba(255, 80, 80, 0.95)";
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(clampedX + 0.5, 0);
            ctx.lineTo(clampedX + 0.5, 10);
            ctx.stroke();

            ctx.fillStyle = "rgba(255, 230, 120, 0.95)";
            ctx.beginPath();
            ctx.arc(clampedX, 1, 2.5, 0, Math.PI * 2);
            ctx.fill();
        }
    }, [bins, width, height, hits, paused]);

    useEffect(() => {
        if (!paused) {
            return;
        }

        const canvas = canvasRef.current;
        if (!canvas) {
            return;
        }

        const ctx = canvas.getContext("2d");
        if (!ctx) {
            return;
        }

        drawPausedOverlay(ctx);
    }, [paused]);

    return (
        <div className="overflow-hidden rounded-xl border border-neutral-800 bg-black">
            <canvas
                ref={canvasRef}
                width={width}
                height={height}
                className="block h-auto w-full"
            />
        </div>
    );
}