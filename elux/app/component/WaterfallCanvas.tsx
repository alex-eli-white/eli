"use client";

import { useEffect, useMemo, useRef } from "react";

type WaterfallCanvasProps = {
    width?: number;
    height?: number;
    bins: number[];
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

function normalizeBins(bins: number[]): number[] {
    if (bins.length === 0) {
        return [];
    }

    const dbVals = bins.map((value) =>
        10 * Math.log10(Math.max(value, 1e-12))
    );

    const sorted = [...dbVals].sort((a, b) => a - b);

    // Robust display window:
    // low percentile ≈ background
    // high percentile ≈ strong signal
    const lowDb = percentile(sorted, 0.10);
    const highDb = percentile(sorted, 0.995);

    const span = Math.max(highDb - lowDb, 1e-6);

    return dbVals.map((db) => {
        const normalized = (db - lowDb) / span;
        return Math.pow(clamp01(normalized), 1.4);
    });
}

export default function WaterfallCanvas({
                                            width = 768,
                                            height = 480,
                                            bins,
                                        }: WaterfallCanvasProps) {
    const canvasRef = useRef<HTMLCanvasElement | null>(null);

    const normalizedBins = useMemo(() => normalizeBins(bins), [bins]);

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
    }, [width, height]);

    useEffect(() => {
        if (normalizedBins.length === 0) {
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
    }, [normalizedBins, width, height]);

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