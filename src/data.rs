use polars::prelude::*;
use std::path::Path;

pub struct DataSource {
    lazy: LazyFrame,
    head_n: u32,
}

impl DataSource {
    pub fn open(path: &Path, head_n: u32) -> color_eyre::Result<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let lazy = match ext.as_str() {
            "csv" | "tsv" => {
                let separator = if ext == "tsv" { b'\t' } else { b',' };
                LazyCsvReader::new(path)
                    .with_separator(separator)
                    .with_has_header(true)
                    .with_infer_schema_length(Some(1000))
                    .finish()?
            }
            "parquet" | "pq" => LazyFrame::scan_parquet(path, Default::default())?,
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Unsupported file format: '{}'. Supported: csv, tsv, parquet",
                    ext
                ));
            }
        };

        Ok(Self { lazy, head_n })
    }

    pub fn head(&self) -> color_eyre::Result<DataFrame> {
        let df = self.lazy.clone().limit(self.head_n).collect()?;
        Ok(df)
    }

    pub fn column_names(&mut self) -> color_eyre::Result<Vec<String>> {
        let schema = self.lazy.collect_schema()?;
        Ok(schema.iter_names().map(|n| n.to_string()).collect())
    }

    pub fn column_dtypes(&mut self) -> color_eyre::Result<Vec<String>> {
        let schema = self.lazy.collect_schema()?;
        Ok(schema
            .iter_names_and_dtypes()
            .map(|(_name, dtype)| format!("{}", dtype))
            .collect())
    }

    pub fn describe_column(&self, col_name: &str) -> color_eyre::Result<DataFrame> {
        let c = col(col_name);
        let stats = self
            .lazy
            .clone()
            .select([
                lit("count").alias("statistic"),
                c.clone().count().cast(DataType::String).alias(col_name),
            ])
            .collect()?;

        let null_count = self
            .lazy
            .clone()
            .select([
                lit("null_count").alias("statistic"),
                c.clone()
                    .null_count()
                    .cast(DataType::String)
                    .alias(col_name),
            ])
            .collect()?;

        let mean = self
            .lazy
            .clone()
            .select([
                lit("mean").alias("statistic"),
                c.clone().mean().cast(DataType::String).alias(col_name),
            ])
            .collect()?;

        let std = self
            .lazy
            .clone()
            .select([
                lit("std").alias("statistic"),
                c.clone().std(1).cast(DataType::String).alias(col_name),
            ])
            .collect()?;

        let min = self
            .lazy
            .clone()
            .select([
                lit("min").alias("statistic"),
                c.clone().min().cast(DataType::String).alias(col_name),
            ])
            .collect()?;

        let max = self
            .lazy
            .clone()
            .select([
                lit("max").alias("statistic"),
                c.clone().max().cast(DataType::String).alias(col_name),
            ])
            .collect()?;

        let median = self
            .lazy
            .clone()
            .select([
                lit("median").alias("statistic"),
                c.clone().median().cast(DataType::String).alias(col_name),
            ])
            .collect()?;

        let mut desc = stats;
        for part in [null_count, mean, std, min, max, median] {
            desc = desc.vstack(&part)?;
        }
        Ok(desc)
    }

    pub fn value_counts(&self, col_name: &str, top_n: usize) -> color_eyre::Result<DataFrame> {
        let vc = self
            .lazy
            .clone()
            .group_by([col(col_name)])
            .agg([col(col_name).count().alias("count")])
            .sort(
                ["count"],
                SortMultipleOptions::default().with_order_descending(true),
            )
            .limit(top_n as u32)
            .collect()?;
        Ok(vc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv() -> NamedTempFile {
        let mut f = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(f, "name,age,score").unwrap();
        writeln!(f, "alice,30,95.5").unwrap();
        writeln!(f, "bob,25,87.3").unwrap();
        writeln!(f, "carol,35,91.0").unwrap();
        f
    }

    #[test]
    fn test_open_csv_and_head() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 10).unwrap();
        let df = ds.head().unwrap();
        assert_eq!(df.shape(), (3, 3));
    }

    #[test]
    fn test_column_names() {
        let f = create_test_csv();
        let mut ds = DataSource::open(f.path(), 10).unwrap();
        let names = ds.column_names().unwrap();
        assert_eq!(names, vec!["name", "age", "score"]);
    }

    #[test]
    fn test_head_limits_rows() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 2).unwrap();
        let df = ds.head().unwrap();
        assert_eq!(df.shape().0, 2);
    }

    #[test]
    fn test_unsupported_format() {
        let f = NamedTempFile::with_suffix(".json").unwrap();
        let result = DataSource::open(f.path(), 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_describe_column() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 1000).unwrap();
        let desc = ds.describe_column("age").unwrap();
        assert!(desc.height() > 0);
        // Should have a "statistic" column and the data column
        let col_names: Vec<String> = desc
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(col_names.contains(&"statistic".to_string()));
    }

    #[test]
    fn test_value_counts() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 1000).unwrap();
        let vc = ds.value_counts("name", 100).unwrap();
        assert_eq!(vc.height(), 3); // 3 unique names
    }
}
