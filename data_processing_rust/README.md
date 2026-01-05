# Data Processing Example - Rust using polars

A high-performance Rust application for processing CSV event data using the Polars dataframe library.

## Overview

This project demonstrates efficient data processing by reading CSV files, applying filters and transformations, performing aggregations, and exporting results to optimized Parquet format. It leverages Polars' lazy evaluation and query optimization capabilities for maximum performance.

## Features

- **Fast CSV Processing**: Reads large CSV files with header detection
- **Lazy Evaluation**: Uses Polars lazy API for query optimization
- **Data Filtering**: Filters events by country (India/IN)
- **Computed Columns**: Calculates revenue amount from price × quantity
- **Aggregation**: Groups data by date, channel, and category with revenue and order count metrics
- **Sorted Output**: Results sorted by revenue
- **Compressed Export**: Outputs to Parquet format with Zstd compression
- **Performance Tracking**: Measures and reports execution time

## Prerequisites

- Rust 1.70+ (edition 2024)
- Cargo package manager

## Installation

Clone the repository and build the project:

```bash
cargo build --release
```

## Usage

1. Ensure you have an `events.csv` file in the project root with the following columns:
   - `id`: Event identifier
   - `date`: Event date
   - `country`: Country code (e.g., "IN", "US")
   - `channel`: Sales or distribution channel
   - `category`: Product or event category
   - `price`: Unit price
   - `qty`: Quantity

2. Run the application:

```bash
cargo run --release
```

3. The processed data will be saved to `out_in.parquet` in the project directory.

## Data Processing Pipeline

The application performs the following operations:

1. **Load**: Reads `events.csv` with header parsing
2. **Filter**: Selects only events where `country = "IN"`
3. **Transform**: Creates `amount` column as `price × qty`
4. **Aggregate**: Groups by `date`, `channel`, `category` and calculates:
   - `rev`: Sum of amounts (total revenue)
   - `orders`: Count of events (order count)
5. **Sort**: Orders results by revenue descending
6. **Export**: Writes to compressed Parquet file with Zstd compression

## Dependencies

- **polars** (v0.43): High-performance DataFrame library with features:
  - `lazy`: Lazy evaluation and query optimization
  - `parquet`: Parquet file format support
  - `csv`: CSV file reading/writing
  - `dtype-date`: Date datatype support

## Performance

The application tracks and displays execution time for the complete data processing pipeline. Polars' lazy evaluation ensures optimal query execution by:
- Predicate pushdown (early filtering)
- Projection pushdown (selecting only needed columns)
- Common subexpression elimination
- Parallel execution where possible

## Output Format

The resulting Parquet file contains:
- `date`: Event date
- `channel`: Distribution channel
- `category`: Product category
- `rev`: Total revenue (sum of price × qty)
- `orders`: Number of orders/events

Results are sorted by revenue in descending order.

## License

This project is provided as-is for educational and demonstration purposes.

## Contributing

Feel free to submit issues or pull requests for improvements.
