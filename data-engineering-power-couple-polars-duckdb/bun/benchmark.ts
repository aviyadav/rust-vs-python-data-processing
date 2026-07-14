import { sleep } from "bun";
import { existsSync, unlinkSync } from "node:fs";
import path from "node:path";

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------
const PROJECT_ROOT = path.resolve(import.meta.dirname, "..");
const PYTHON_SCRIPT = path.join(PROJECT_ROOT, "python", "generate_events.py");
const RUST_BIN = path.join(PROJECT_ROOT, "rust", "target", "release", "gen_events");

interface BenchRow {
  rows: number;
  lang: string;
  wallSec: number;
  genSec: number;
  writeSec: number;
  peakMb: number;
  csvMb: number;
}

type ParsedOutput = {
  genSec: number;
  writeSec: number;
  wallSec: number;
  peakKb: number;
};

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------
function parsePythonOutput(stdout: string): ParsedOutput | null {
  const gen = stdout.match(/Generation done in ([\d.]+) s/);
  const write = stdout.match(/CSV written in ([\d.]+) s/);
  const wall = stdout.match(/Wall time: ([\d.]+) s/);
  if (!gen || !write || !wall) return null;
  return {
    genSec: parseFloat(gen[1]),
    writeSec: parseFloat(write[1]),
    wallSec: parseFloat(wall[1]),
    peakKb: 0,
  };
}

function parseRustStderr(stderr: string): ParsedOutput | null {
  const gen = stderr.match(/Generation done in ([\d.]+)(?:ms|s)/);
  const write = stderr.match(/CSV written in ([\d.]+)(?:ms|s)/);
  const wall = stderr.match(/Wall time: ([\d.]+)(?:ms|s)/);
  if (!gen || !write || !wall) return null;

  const toSec = (v: string, unit: string) =>
    unit === "ms" ? parseFloat(v) / 1000 : parseFloat(v);

  const genRaw = gen[1];
  const writeRaw = write[1];
  const wallRaw = wall[1];
  const genUnit = gen[0].endsWith("ms") ? "ms" : "s";
  const writeUnit = write[0].endsWith("ms") ? "ms" : "s";
  const wallUnit = wall[0].endsWith("ms") ? "ms" : "s";

  return {
    genSec: toSec(genRaw, genUnit),
    writeSec: toSec(writeRaw, writeUnit),
    wallSec: toSec(wallRaw, wallUnit),
    peakKb: 0,
  };
}

// ---------------------------------------------------------------------------
// Run with /usr/bin/time -v for memory
// ---------------------------------------------------------------------------
async function runWithMemory(
  cmd: string,
  args: string[],
  env: Record<string, string>,
  cwd: string
): Promise<{ stdout: string; stderr: string; exitCode: number; peakKb: number }> {
  const fullCmd = ["/usr/bin/time", "-v", cmd, ...args];
  const proc = Bun.spawnSync(fullCmd, {
    env: { ...process.env, ...env },
    cwd,
  });

  const stdout = proc.stdout.toString();
  const stderr = proc.stderr.toString();
  const exitCode = proc.exitCode;

  // Parse peak memory from /usr/bin/time -v stderr
  const memMatch = stderr.match(/Maximum resident set size \(kbytes\): (\d+)/);
  const peakKb = memMatch ? parseInt(memMatch[1], 10) : 0;

  return { stdout, stderr, exitCode, peakKb };
}

// ---------------------------------------------------------------------------
// Run one benchmark iteration
// ---------------------------------------------------------------------------
async function runOne(
  lang: "python" | "rust",
  rows: number,
  batchSize: number,
  outputPath: string,
  workers: number
): Promise<ParsedOutput | null> {
  // Clean up prior output
  if (existsSync(outputPath)) unlinkSync(outputPath);

  if (lang === "python") {
    const args = [
      PYTHON_SCRIPT,
      "--rows", String(rows),
      "--batch-size", String(batchSize),
      "-o", outputPath,
      "--workers", String(workers),
    ];
    const r = await runWithMemory("python3", args, {}, PROJECT_ROOT);
    if (r.exitCode !== 0) {
      console.error("Python stderr:", r.stderr);
      return null;
    }
    const parsed = parsePythonOutput(r.stdout);
    if (!parsed) return null;
    parsed.peakKb = r.peakKb;
    return parsed;
  } else {
    // Rust
    const env: Record<string, string> = {
      ROWS: String(rows),
      BATCH_SIZE: String(batchSize),
      OUTPUT: outputPath,
    };
    const r = await runWithMemory(RUST_BIN, [], env, PROJECT_ROOT);
    if (r.exitCode !== 0) {
      console.error("Rust stderr:", r.stderr);
      return null;
    }
    // Rust writes progress to stderr (via eprintln)
    const parsed = parseRustStderr(r.stderr);
    if (!parsed) return null;
    parsed.peakKb = r.peakKb;
    return parsed;
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  console.log(`\n  Benchmark: User-Event CSV Generator`);
  console.log(`  ${"=".repeat(50)}\n`);

  // Defaults from CLI args or hardcoded
  const rowCounts: number[] = [100_000, 500_000, 1_000_000];
  const runs = 3; // each config runs N times, results averaged
  const batchSize = 50_000;
  const workers = 8;

  // Validate binaries exist
  if (!existsSync(PYTHON_SCRIPT)) {
    console.error(`  ERROR: Python script not found at ${PYTHON_SCRIPT}`);
    process.exit(1);
  }
  if (!existsSync(RUST_BIN)) {
    console.error(
      `  ERROR: Rust binary not found at ${RUST_BIN}\n` +
      `  Build it first: cd rust && cargo build --release`
    );
    process.exit(1);
  }

  // Check /usr/bin/time
  const timeCheck = Bun.spawnSync(["/usr/bin/time", "--version"]);
  if (timeCheck.exitCode !== 0) {
    console.error("  ERROR: /usr/bin/time not found (needed for memory measurement)");
    process.exit(1);
  }

  const results: BenchRow[] = [];
  const tmpDir = "/tmp";

  for (const rows of rowCounts) {
    for (const lang of ["python", "rust"] as const) {
      const outputPath = path.join(tmpDir, `bench_${lang}_${rows}.csv`);
      const attempts: ParsedOutput[] = [];

      for (let i = 0; i < runs; i++) {
        process.stdout.write(
          `  [${lang.padEnd(6)}] ${String(rows).padStart(8)} rows  (run ${i + 1}/${runs}) ... `
        );
        const r = await runOne(lang, rows, batchSize, outputPath, workers);
        if (r) {
          attempts.push(r);
          process.stdout.write(`ok\n`);
        } else {
          process.stdout.write(`FAILED\n`);
        }
        // brief cooldown
        await sleep(1000);
      }

      if (attempts.length === 0) {
        console.error(`  All runs failed for ${lang} ${rows} rows. Skipping.`);
        continue;
      }

      const avgWall = avgSec(attempts.map((a) => a.wallSec));
      const avgGen = avgSec(attempts.map((a) => a.genSec));
      const avgWrite = avgSec(attempts.map((a) => a.writeSec));
      const avgPeakKb = avg(attempts.map((a) => a.peakKb));
      const csvMb = existsSync(outputPath)
        ? (Bun.spawnSync(["du", "-k", outputPath]).stdout.toString().match(/^(\d+)/)?.[1] ?? "0")
        : "0";

      results.push({
        rows,
        lang,
        wallSec: avgWall,
        genSec: avgGen,
        writeSec: avgWrite,
        peakMb: avgPeakKb / 1024,
        csvMb: parseInt(csvMb) / 1024,
      });
    }
  }

  // -----------------------------------------------------------------------
  // Print results table
  // -----------------------------------------------------------------------
  console.log(`\n  ${"=".repeat(80)}`);
  console.log(`  RESULTS (averaged over ${runs} runs each)`);
  console.log(`  ${"=".repeat(80)}`);

  // Manual table formatting
  const hr = () => "  " + "─".repeat(78);
  const headers = ["Lang    ", "Rows     ", "Gen (s)  ", "Write (s)", "Wall (s) ", "Peak RSS  ", "CSV Size "];
  console.log(hr());
  console.log("  │ " + headers.join("│ ") + "│");
  console.log(hr());
  for (const r of results) {
    const row = [
      r.lang.padEnd(7),
      r.rows.toLocaleString().padStart(8),
      r.genSec.toFixed(3).padStart(8),
      r.writeSec.toFixed(3).padStart(8),
      r.wallSec.toFixed(3).padStart(8),
      `${r.peakMb.toFixed(1)} MB`.padStart(9),
      `${r.csvMb.toFixed(1)} MB`.padStart(9),
    ];
    console.log("  │ " + row.join("│ ") + "│");
  }
  console.log(hr());
  console.log();

  // -----------------------------------------------------------------------
  // Summary
  // -----------------------------------------------------------------------
  console.log(`  ${"=".repeat(80)}`);
  console.log(`  SUMMARY`);
  console.log(`  ${"=".repeat(80)}`);
  console.log(`  Batch size: ${batchSize.toLocaleString()}  |  Workers: ${workers}`);
  console.log(`  Output files written to /tmp/bench_*.csv (cleaned between runs)`);
  console.log();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function avg(nums: number[]): number {
  if (nums.length === 0) return 0;
  return nums.reduce((a, b) => a + b, 0) / nums.length;
}

function avgSec(secs: number[]): number {
  return avg(secs);
}

main().catch((err) => {
  console.error("FATAL:", err);
  process.exit(1);
});
