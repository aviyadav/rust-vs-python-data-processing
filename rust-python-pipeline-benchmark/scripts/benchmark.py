"""
Benchmark Script for Python vs Rust Pipeline Comparison

Runs both pipelines multiple times and compares performance metrics.
"""

import argparse
import json
import statistics
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path


class BenchmarkRunner:
    def __init__(self, data_dir: str, runs: int = 3):
        self.data_dir = data_dir
        self.runs = runs
        self.results = {"python": [], "rust": []}

    def run_python_pipeline(self) -> dict:
        """Run Python pipeline and capture metrics"""
        print("\n🐍 Running Python pipeline...")

        start_time = time.time()

        result = subprocess.run(
            [
                "python",
                "python-pipeline/pipeline-polars.py",
                "--data-dir",
                self.data_dir,
            ],
            capture_output=True,
            text=True,
        )

        duration = time.time() - start_time

        if result.returncode != 0:
            print(f"❌ Python pipeline failed: {result.stderr}")
            return None

        # Parse output for metrics
        output = result.stdout
        memory_mb = self._extract_metric(output, "Peak Memory:", "MB")

        return {"duration": duration, "memory_mb": memory_mb, "success": True}

    def run_rust_pipeline(self) -> dict:
        """Run Rust pipeline and capture metrics"""
        print("\n🦀 Running Rust pipeline...")

        # Build first (only once)
        if not hasattr(self, "_rust_built"):
            print("Building Rust pipeline...")
            build_result = subprocess.run(
                ["cargo", "build", "--release"],
                cwd="rust-pipeline",
                capture_output=True,
            )
            if build_result.returncode != 0:
                print(f"❌ Rust build failed: {build_result.stderr.decode()}")
                return None
            self._rust_built = True

        # Resolve to absolute path so the Rust binary can find it regardless
        # of the cwd="rust-pipeline" that cargo run is executed with.
        data_dir_abs = str(Path(self.data_dir).resolve())

        start_time = time.time()

        result = subprocess.run(
            ["cargo", "run", "--release", "--", data_dir_abs],
            cwd="rust-pipeline",
            capture_output=True,
            text=True,
        )

        duration = time.time() - start_time

        if result.returncode != 0:
            print(f"❌ Rust pipeline failed: {result.stderr}")
            return None

        # Parse output for metrics
        output = result.stdout
        memory_mb = self._extract_metric(output, "Peak Memory:", "MB")

        return {"duration": duration, "memory_mb": memory_mb, "success": True}

    def _extract_metric(self, output: str, prefix: str, suffix: str) -> float:
        """Extract numeric metric from output"""
        try:
            for line in output.split("\n"):
                if prefix in line:
                    # Extract number between prefix and suffix
                    start = line.find(prefix) + len(prefix)
                    end = line.find(suffix, start)
                    value_str = line[start:end].strip()
                    return float(value_str)
        except:
            pass
        return 0.0

    def run_benchmarks(self):
        """Run all benchmarks"""
        print(f"\n{'=' * 60}")
        print(f"Starting Benchmark Suite")
        print(f"Data directory: {self.data_dir}")
        print(f"Number of runs: {self.runs}")
        print(f"{'=' * 60}")

        for run in range(1, self.runs + 1):
            print(f"\n{'=' * 60}")
            print(f"Run {run}/{self.runs}")
            print(f"{'=' * 60}")

            # Run Python
            python_result = self.run_python_pipeline()
            if python_result:
                self.results["python"].append(python_result)
                print(
                    f"✓ Python: {python_result['duration']:.2f}s, {python_result['memory_mb']:.2f}MB"
                )

            # Run Rust
            rust_result = self.run_rust_pipeline()
            if rust_result:
                self.results["rust"].append(rust_result)
                print(
                    f"✓ Rust: {rust_result['duration']:.2f}s, {rust_result['memory_mb']:.2f}MB"
                )

            # Brief pause between runs
            if run < self.runs:
                time.sleep(2)

    def calculate_statistics(self, values: list) -> dict:
        """Calculate statistics for a list of values"""
        if not values:
            return {}

        return {
            "mean": statistics.mean(values),
            "median": statistics.median(values),
            "min": min(values),
            "max": max(values),
            "stdev": statistics.stdev(values) if len(values) > 1 else 0,
        }

    def print_results(self):
        """Print benchmark results"""
        print(f"\n{'=' * 60}")
        print(f"Benchmark Results Summary")
        print(f"{'=' * 60}\n")

        # Python results
        if self.results["python"]:
            python_times = [r["duration"] for r in self.results["python"]]
            python_memory = [r["memory_mb"] for r in self.results["python"]]

            python_time_stats = self.calculate_statistics(python_times)
            python_mem_stats = self.calculate_statistics(python_memory)

            print("🐍 Python + Pandas:")
            print(
                f"  Duration:    {python_time_stats['mean']:.2f}s (±{python_time_stats['stdev']:.2f}s)"
            )
            print(f"               {python_time_stats['mean'] / 60:.2f} minutes")
            print(
                f"  Memory:      {python_mem_stats['mean']:.2f}MB (±{python_mem_stats['stdev']:.2f}MB)"
            )
            print(f"               {python_mem_stats['mean'] / 1024:.2f}GB")
            print()

        # Rust results
        if self.results["rust"]:
            rust_times = [r["duration"] for r in self.results["rust"]]
            rust_memory = [r["memory_mb"] for r in self.results["rust"]]

            rust_time_stats = self.calculate_statistics(rust_times)
            rust_mem_stats = self.calculate_statistics(rust_memory)

            print("🦀 Rust + DuckDB:")
            print(
                f"  Duration:    {rust_time_stats['mean']:.2f}s (±{rust_time_stats['stdev']:.2f}s)"
            )
            print(f"               {rust_time_stats['mean'] / 60:.2f} minutes")
            print(
                f"  Memory:      {rust_mem_stats['mean']:.2f}MB (±{rust_mem_stats['stdev']:.2f}MB)"
            )
            print(f"               {rust_mem_stats['mean'] / 1024:.2f}GB")
            print()

        # Comparison
        if self.results["python"] and self.results["rust"]:
            python_avg_time = statistics.mean(
                [r["duration"] for r in self.results["python"]]
            )
            rust_avg_time = statistics.mean(
                [r["duration"] for r in self.results["rust"]]
            )

            python_avg_mem = statistics.mean(
                [r["memory_mb"] for r in self.results["python"]]
            )
            rust_avg_mem = statistics.mean(
                [r["memory_mb"] for r in self.results["rust"]]
            )

            speedup = python_avg_time / rust_avg_time
            memory_reduction = ((python_avg_mem - rust_avg_mem) / python_avg_mem) * 100

            print("📊 Comparison:")
            print(f"  Speed improvement:   {speedup:.1f}× faster")
            print(f"  Memory reduction:    {memory_reduction:.1f}% less")
            print()

        print(f"{'=' * 60}\n")

    def save_results(self, output_file: str):
        """Save results to JSON file"""
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        results_data = {
            "timestamp": datetime.now().isoformat(),
            "data_dir": self.data_dir,
            "runs": self.runs,
            "results": self.results,
        }

        with open(output_path, "w") as f:
            json.dump(results_data, f, indent=2)

        print(f"Results saved to: {output_path}")


def main():
    parser = argparse.ArgumentParser(
        description="Benchmark Python vs Rust data pipelines",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run 3 benchmarks with default data directory
  python scripts/benchmark.py --runs 3

  # Run 5 benchmarks with custom data directory
  python scripts/benchmark.py --data-dir data/ --runs 5

  # Save results to custom file
  python scripts/benchmark.py --output results/my_benchmark.json
        """,
    )

    parser.add_argument(
        "--data-dir",
        default="data",
        help="Directory containing CSV files (default: data)",
    )

    parser.add_argument(
        "--runs", type=int, default=3, help="Number of benchmark runs (default: 3)"
    )

    parser.add_argument(
        "--output",
        default="results/benchmark_results.json",
        help="Output file for results (default: results/benchmark_results.json)",
    )

    args = parser.parse_args()

    # Check if data directory exists
    if not Path(args.data_dir).exists():
        print(f"❌ Error: Data directory '{args.data_dir}' does not exist")
        print(f"\nGenerate sample data first:")
        print(f"  python scripts/generate_data.py --size 1GB --output {args.data_dir}")
        sys.exit(1)

    # Run benchmarks
    runner = BenchmarkRunner(args.data_dir, args.runs)
    runner.run_benchmarks()
    runner.print_results()
    runner.save_results(args.output)


if __name__ == "__main__":
    main()
