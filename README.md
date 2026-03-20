# tblv

Fast terminal viewer for CSV and Parquet files. Built with [Polars](https://pola.rs/) and [Ratatui](https://ratatui.rs/).

- Lazy loading — never reads the entire file into memory
- Vim-style navigation
- Sort and filter columns interactively
- Inspect individual cell values, column statistics, and unique values

## Installation

### Shell (macOS / Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/IlyaKaftanov/tblv/releases/latest/download/tblv-installer.sh | sh
```

### Cargo

```bash
cargo install tblv
```

### From source

```bash
git clone https://github.com/IlyaKaftanov/tblv.git
cd tblv
cargo install --path .
```

## Usage

```bash
tblv data.csv
tblv data.parquet
tblv data.csv -n 10000    # load up to 10000 rows (default: 5000)
```

## Keybindings

| Key                 | Action                                   |
| ------------------- | ---------------------------------------- |
| `h` `j` `k` `l`     | Move left / down / up / right            |
| `Ctrl-d` / `Ctrl-u` | Jump half-page down / up                 |
| `Ctrl-h` / `Ctrl-l` | Jump half-page left / right              |
| `g` / `G`           | Go to first / last row                   |
| `Enter`             | View full cell value (`j`/`k` to scroll) |
| `d`                 | Describe column (stats)                  |
| `u`                 | Show unique values                       |
| `s`                 | Cycle sort: None → Asc → Desc            |
| `f`                 | Filter column (value picker)             |
| `c`                 | Clear all filters                        |
| `?`                 | Help                                     |
| `q` / `Esc`         | Quit / back                              |

**Filter menu:** `Space` toggle, `a` select all, `n` select none, `Enter` apply, `Esc` cancel

## License

MIT
