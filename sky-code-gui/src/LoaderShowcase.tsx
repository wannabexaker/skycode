/**
 * SkyCode Loader Components Demo
 * 
 * Demonstrates all available loading animations with SkyCode branding.
 * All loaders use Sky Blue (#0EA5E9) as the accent color.
 */

import { useState } from "react";
import { Sun, Moon } from "lucide-react";
import { Button } from "./components/ui/button";
import { LoadingBreadcrumb, SkyCodeZapLoader } from "./components/ui/animated-loading-svg-text-shimmer";
import { Loader } from "./components/ui/loader";

export default function LoaderShowcase() {
  const [isDark, setIsDark] = useState(true);

  // All available loader variants
  const loaderVariants = [
    "circular",
    "classic",
    "pulse",
    "pulse-dot",
    "dots",
    "typing",
    "wave",
    "bars",
    "terminal",
  ] as const;

  const textLoaderVariants = [
    { variant: "text-blink" as const, text: "Thinking" },
    { variant: "text-shimmer" as const, text: "Analyzing" },
    { variant: "loading-dots" as const, text: "Processing" },
  ];

  return (
    <div className={isDark ? "dark" : ""}>
      <div className="min-h-screen bg-bg-0 text-text-100 transition-colors duration-200">
        {/* Header */}
        <div className="border-b border-bg-300 px-6 py-4">
          <div className="mx-auto flex max-w-6xl items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold font-mono text-accent">
                ϟ SkyCode Loaders
              </h1>
              <p className="text-sm text-text-400 mt-1">
                All loading animations with SkyCode brand identity
              </p>
            </div>
            <Button
              variant="outline"
              size="icon"
              onClick={() => setIsDark(!isDark)}
              aria-label="Toggle theme"
            >
              {isDark ? <Sun size={16} /> : <Moon size={16} />}
            </Button>
          </div>
        </div>

        {/* Content */}
        <div className="mx-auto max-w-6xl px-6 py-12 space-y-16">
          
          {/* Animated SVG Loaders */}
          <section>
            <h2 className="text-lg font-semibold mb-6 font-mono text-text-200">
              Animated SVG Text Shimmer
            </h2>
            <div className="grid gap-8 sm:grid-cols-2">
              <div className="flex flex-col items-center gap-4 rounded-xl border border-bg-300 bg-bg-100 p-8">
                <LoadingBreadcrumb />
                <p className="text-xs text-text-400 font-mono">Default</p>
              </div>
              
              <div className="flex flex-col items-center gap-4 rounded-xl border border-bg-300 bg-bg-100 p-8">
                <LoadingBreadcrumb text="Analyzing code" showChevron />
                <p className="text-xs text-text-400 font-mono">With chevron</p>
              </div>
              
              <div className="flex flex-col items-center gap-4 rounded-xl border border-bg-300 bg-bg-100 p-8">
                <SkyCodeZapLoader text="Powering up" />
                <p className="text-xs text-text-400 font-mono">Zap variant</p>
              </div>
              
              <div className="flex flex-col items-center gap-4 rounded-xl border border-bg-300 bg-bg-100 p-8">
                <SkyCodeZapLoader text="Building project" />
                <p className="text-xs text-text-400 font-mono">Zap custom text</p>
              </div>
            </div>
          </section>

          {/* Icon-based Loaders */}
          <section>
            <h2 className="text-lg font-semibold mb-6 font-mono text-text-200">
              Icon-based Loaders
            </h2>
            <div className="grid gap-6 sm:grid-cols-3 md:grid-cols-4">
              {loaderVariants.map((variant) => (
                <div
                  key={variant}
                  className="flex flex-col items-center justify-center gap-4 rounded-xl border border-bg-300 bg-bg-100 p-8"
                >
                  <Loader variant={variant} size="md" />
                  <p className="text-xs text-text-400 font-mono">{variant}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Text-based Loaders */}
          <section>
            <h2 className="text-lg font-semibold mb-6 font-mono text-text-200">
              Text-based Loaders
            </h2>
            <div className="grid gap-6 sm:grid-cols-3">
              {textLoaderVariants.map(({ variant, text }) => (
                <div
                  key={variant}
                  className="flex flex-col items-center justify-center gap-4 rounded-xl border border-bg-300 bg-bg-100 p-8"
                >
                  <Loader variant={variant} text={text} size="md" />
                  <p className="text-xs text-text-400 font-mono">{variant}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Size Variants */}
          <section>
            <h2 className="text-lg font-semibold mb-6 font-mono text-text-200">
              Size Variants
            </h2>
            <div className="grid gap-6 sm:grid-cols-3">
              {(["sm", "md", "lg"] as const).map((size) => (
                <div
                  key={size}
                  className="flex flex-col items-center justify-center gap-4 rounded-xl border border-bg-300 bg-bg-100 p-8"
                >
                  <Loader variant="circular" size={size} />
                  <p className="text-xs text-text-400 font-mono uppercase">{size}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Usage Examples */}
          <section className="rounded-xl border border-bg-300 bg-bg-100 p-6">
            <h2 className="text-lg font-semibold mb-4 font-mono text-text-200">
              Usage Examples
            </h2>
            <div className="space-y-4 font-mono text-sm">
              <div className="rounded-lg bg-bg-200 p-4">
                <p className="text-text-300 mb-2">// Animated SVG shimmer text</p>
                <code className="text-accent">
                  {'<LoadingBreadcrumb text="SkyCode thinking" />'}
                </code>
              </div>
              
              <div className="rounded-lg bg-bg-200 p-4">
                <p className="text-text-300 mb-2">// Icon loader</p>
                <code className="text-accent">
                  {'<Loader variant="circular" size="md" />'}
                </code>
              </div>
              
              <div className="rounded-lg bg-bg-200 p-4">
                <p className="text-text-300 mb-2">// Text loader</p>
                <code className="text-accent">
                  {'<Loader variant="text-shimmer" text="Processing" />'}
                </code>
              </div>
              
              <div className="rounded-lg bg-bg-200 p-4">
                <p className="text-text-300 mb-2">// Zap variant</p>
                <code className="text-accent">
                  {'<SkyCodeZapLoader text="Building" />'}
                </code>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
