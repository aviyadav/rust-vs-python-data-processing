import time
from multiprocessing import Pool, cpu_count
from typing import Tuple
import os

import numpy as np
import polars as pl

# Constants
NUM_ROWS = 5_000_000  # Total number of rows to generate
CHUNK_SIZE = 500_000  # Larger chunks for better efficiency
OUTPUT_FILE = "events.csv"
COUNTRIES = [
    "US",
    "CA",
    "GB",
    "DE",
    "FR",
    "IT",
    "ES",
    "JP",
    "AU",
    "IN",
    "BR",
    "MX",
    "CN",
    "RU",
    "KR",
]
CHANNELS = [
    "online",
    "retail",
    "mobile",
    "social",
    "email",
    "direct",
    "referral",
    "affiliate",
]
CATEGORIES = [
    "electronics",
    "clothing",
    "food",
    "books",
    "sports",
    "home",
    "beauty",
    "toys",
    "automotive",
    "health",
]


def generate_chunk_vectorized(chunk_size: int, chunk_id: int, seed: int) -> pl.DataFrame:
    """Generate a chunk of random data using vectorized operations."""
    # Use new numpy Generator API - faster than legacy np.random
    rng = np.random.Generator(np.random.PCG64(seed + chunk_id))
    
    start_id = chunk_id * CHUNK_SIZE
    
    # Generate all data using vectorized numpy operations
    ids = np.arange(start_id + 1, start_id + chunk_size + 1, dtype=np.int64)
    
    # Use integer indices instead of string choice - faster
    country_idx = rng.integers(0, len(COUNTRIES), size=chunk_size, dtype=np.int8)
    channel_idx = rng.integers(0, len(CHANNELS), size=chunk_size, dtype=np.int8)
    category_idx = rng.integers(0, len(CATEGORIES), size=chunk_size, dtype=np.int8)
    
    # Map indices to strings using numpy advanced indexing
    countries_arr = np.array(COUNTRIES)
    channels_arr = np.array(CHANNELS)
    categories_arr = np.array(CATEGORIES)
    
    countries = countries_arr[country_idx]
    channels = channels_arr[channel_idx]
    categories = categories_arr[category_idx]
    
    # Generate prices and quantities
    prices = np.round(rng.uniform(10.0, 1000.0, size=chunk_size), 2).astype(np.float32)
    quantities = rng.integers(1, 101, size=chunk_size, dtype=np.int16)
    
    # Generate dates using numpy datetime64 - MUCH faster than datetime objects
    base_date = np.datetime64('2025-01-01')
    days_offset = rng.integers(0, 365, size=chunk_size, dtype=np.int16)
    dates = base_date + days_offset.astype('timedelta64[D]')
    
    # Create Polars DataFrame directly with proper types
    df = pl.DataFrame({
        "id": ids,
        "country": countries,
        "price": prices,
        "qty": quantities,
        "date": dates,
        "channel": channels,
        "category": categories,
    })
    
    return df


def process_chunk(args: Tuple[int, int, int]) -> pl.DataFrame:
    """Process a chunk of data and return a Polars DataFrame."""
    chunk_size, chunk_id, seed = args
    return generate_chunk_vectorized(chunk_size, chunk_id, seed)


def write_chunk_to_file(args: Tuple[int, int, int, str, bool]) -> dict:
    """Generate and write a chunk directly to file (streaming approach)."""
    chunk_size, chunk_id, seed, temp_dir, _ = args
    df = generate_chunk_vectorized(chunk_size, chunk_id, seed)
    
    # Write to temporary parquet file (much faster than CSV)
    temp_file = f"{temp_dir}/chunk_{chunk_id:05d}.parquet"
    df.write_parquet(temp_file, compression="lz4")
    
    return {"chunk_id": chunk_id, "rows": len(df), "file": temp_file}


def main():
    """Main function to generate the events.csv file."""
    print(f"Generating {NUM_ROWS:,} rows of data using optimized vectorized operations...")
    start_time = time.time()

    seed = int(time.time())  # Base seed
    
    # Calculate number of chunks
    num_chunks = (NUM_ROWS + CHUNK_SIZE - 1) // CHUNK_SIZE
    print(f"Using {num_chunks} chunks of size ~{CHUNK_SIZE:,}")

    # Determine number of processes to use
    num_processes = min(cpu_count(), num_chunks)
    print(f"Using {num_processes} CPU cores")
    
    # Create temp directory for intermediate files
    temp_dir = "temp_chunks"
    os.makedirs(temp_dir, exist_ok=True)

    # Create arguments for each chunk
    chunk_args = []
    for i in range(num_chunks):
        if i == num_chunks - 1:  # Last chunk might be smaller
            remaining_rows = NUM_ROWS - (i * CHUNK_SIZE)
            chunk_args.append((remaining_rows, i, seed, temp_dir, i == 0))
        else:
            chunk_args.append((CHUNK_SIZE, i, seed, temp_dir, i == 0))

    # Process chunks in parallel and write to temp files
    print("Generating and writing chunks in parallel...")
    with Pool(processes=num_processes) as pool:
        results = pool.map(write_chunk_to_file, chunk_args)
    
    total_rows = sum(r["rows"] for r in results)
    gen_time = time.time() - start_time
    print(f"✅ Generated {total_rows:,} rows in {gen_time:.2f}s ({total_rows/gen_time:,.0f} rows/sec)")

    # Combine parquet files and write to final CSV
    print("Combining chunks and writing to CSV...")
    combine_start = time.time()
    
    # Use Polars lazy API to stream through files
    parquet_files = sorted([r["file"] for r in results])
    
    # Stream write to CSV to avoid memory issues
    final_df = pl.scan_parquet(parquet_files).collect(streaming=True)
    final_df.write_csv(OUTPUT_FILE, separator=",")
    
    combine_time = time.time() - combine_start
    print(f"✅ Combined and wrote CSV in {combine_time:.2f}s")
    
    # Cleanup temp files
    print("Cleaning up temporary files...")
    for f in parquet_files:
        os.remove(f)
    os.rmdir(temp_dir)

    end_time = time.time()
    elapsed_time = end_time - start_time

    print(f"\n✅ Successfully generated {total_rows:,} rows in {OUTPUT_FILE}")
    print(f"⏱️  Total time: {elapsed_time:.2f} seconds")
    print(f"🚀 Overall speed: {total_rows / elapsed_time:,.0f} rows/second")

    # Display sample data
    print(f"\n📊 Sample data (first 5 rows):")
    sample_df = pl.read_csv(OUTPUT_FILE, n_rows=5)
    print(sample_df)


if __name__ == "__main__":
    main()
