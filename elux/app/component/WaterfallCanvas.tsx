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

/**
 * Very simple gradient:
 * 0.00 -> black
 * 0.20 -> dark blue
 * 0.45 -> cyan/green
 * 0.70 -> yellow/orange
 * 1.00 -> white
 */
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

function normalizeBins(bins: number[]): number[] {
    if (bins.length === 0) {
        return [];
    }

    // Log-compress the bins, but do not assume backend units are true dB.
    const compressed = bins.map((value) => {
        const safe = Number.isFinite(value) ? Math.max(value, 1e-12) : 1e-12;
        return Math.log10(safe);
    });

    let min = Infinity;
    let max = -Infinity;

    for (const value of compressed) {
        if (!Number.isFinite(value)) {
            continue;
        }

        if (value < min) {
            min = value;
        }

        if (value > max) {
            max = value;
        }
    }

    if (!Number.isFinite(min) || !Number.isFinite(max)) {
        return new Array(bins.length).fill(0);
    }

    const span = max - min;

    if (span < 1e-9) {
        return new Array(bins.length).fill(0);
    }

    return compressed.map((value) => clamp01((value - min) / span));
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

        // Shift existing image down by 1 pixel row.
        ctx.drawImage(canvas, 0, 0, width, height - 1, 0, 1, width, height - 1);

        // Draw newest row at the top.
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