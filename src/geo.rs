/// Parsed geometry from a cell value.
#[derive(Debug, Clone)]
pub enum Geometry {
    /// Single point: (lat, lon).
    Point(f64, f64),
    /// Ordered list of (lat, lon) pairs.
    LineString(Vec<(f64, f64)>),
}

impl Geometry {
    /// Convert to GeoJSON Feature string.
    pub fn to_geojson(&self) -> String {
        let geom = match self {
            Geometry::Point(lat, lon) => {
                format!(r#"{{"type":"Point","coordinates":[{},{}]}}"#, lon, lat)
            }
            Geometry::LineString(coords) => {
                let pairs: Vec<String> = coords
                    .iter()
                    .map(|(lat, lon)| format!("[{},{}]", lon, lat))
                    .collect();
                format!(
                    r#"{{"type":"LineString","coordinates":[{}]}}"#,
                    pairs.join(",")
                )
            }
        };
        format!(
            r#"{{"type":"FeatureCollection","features":[{{"type":"Feature","geometry":{},"properties":{{}}}}]}}"#,
            geom
        )
    }
}

/// Parse a cell value string into a Geometry.
///
/// Supported formats:
/// - WKT: `POINT(lon lat)`, `LINESTRING(lon lat, lon lat, ...)`
/// - Comma-separated: `lat,lon`
///
/// Polars string values may be wrapped in quotes — those are stripped first.
pub fn parse_geometry(value: &str) -> Result<Geometry, String> {
    let trimmed = value.trim().trim_matches('"');

    // Try WKT POINT
    if let Some(pt) = parse_wkt_point(trimmed) {
        return Ok(Geometry::Point(pt.0, pt.1));
    }

    // Try WKT LINESTRING
    if let Some(line) = parse_wkt_linestring(trimmed) {
        return Ok(Geometry::LineString(line));
    }

    // Try comma-separated lat,lon pair
    if let Some(pt) = parse_comma_pair(trimmed) {
        return Ok(Geometry::Point(pt.0, pt.1));
    }

    Err("Could not parse as geometry".to_string())
}

fn parse_wkt_point(s: &str) -> Option<(f64, f64)> {
    let upper = s.to_uppercase();
    let inner = upper
        .strip_prefix("POINT(")
        .or_else(|| upper.strip_prefix("POINT ("))?;
    let inner = inner.strip_suffix(')')?;
    parse_lon_lat_pair(inner)
}

fn parse_wkt_linestring(s: &str) -> Option<Vec<(f64, f64)>> {
    let upper = s.to_uppercase();
    let inner = upper
        .strip_prefix("LINESTRING(")
        .or_else(|| upper.strip_prefix("LINESTRING ("))?;
    let inner = inner.strip_suffix(')')?;

    let coords: Option<Vec<(f64, f64)>> = inner.split(',').map(parse_lon_lat_pair).collect();
    let coords = coords?;
    if coords.len() < 2 {
        return None;
    }
    Some(coords)
}

/// Parse a "lon lat" whitespace-separated pair (WKT order) into (lat, lon).
fn parse_lon_lat_pair(s: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    let lon: f64 = parts[0].parse().ok()?;
    let lat: f64 = parts[1].parse().ok()?;
    validate_lat_lon(lat, lon)
}

fn parse_comma_pair(s: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    let a: f64 = parts[0].trim().parse().ok()?;
    let b: f64 = parts[1].trim().parse().ok()?;

    if (-90.0..=90.0).contains(&a) && (-180.0..=180.0).contains(&b) {
        return Some((a, b));
    }
    if (-90.0..=90.0).contains(&b) && (-180.0..=180.0).contains(&a) {
        return Some((b, a));
    }
    None
}

/// Generate a Leaflet HTML page and open it in the default browser.
pub fn open_in_browser(geom: &Geometry) -> Result<(), String> {
    let geojson = geom.to_geojson();
    let html = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>tblv map</title>
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9/dist/leaflet.css"/>
<script src="https://unpkg.com/leaflet@1.9/dist/leaflet.js"></script>
<style>body{{margin:0}}#map{{height:100vh}}</style>
</head><body>
<div id="map"></div>
<script>
var geojson = {geojson};
var map = L.map('map');
L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
  attribution: '&copy; OpenStreetMap contributors'
}}).addTo(map);
var layer = L.geoJSON(geojson, {{
  pointToLayer: function(f, ll) {{ return L.circleMarker(ll, {{radius:8, color:'#e74c3c', fillOpacity:0.8}}); }}
}}).addTo(map);
map.fitBounds(layer.getBounds().pad(0.3));
</script>
</body></html>"#
    );

    let path = std::env::temp_dir().join("tblv_map.html");
    std::fs::write(&path, html).map_err(|e| format!("Failed to write temp file: {}", e))?;

    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "start";

    std::process::Command::new(cmd)
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(())
}

fn validate_lat_lon(lat: f64, lon: f64) -> Option<(f64, f64)> {
    if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
        Some((lat, lon))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wkt_point() {
        let geo = parse_geometry("POINT(30.5 50.2)").unwrap();
        match geo {
            Geometry::Point(lat, lon) => {
                assert!((lat - 50.2).abs() < 1e-9);
                assert!((lon - 30.5).abs() < 1e-9);
            }
            _ => panic!("expected Point"),
        }
    }

    #[test]
    fn test_wkt_point_with_space() {
        let geo = parse_geometry("POINT (30.5 50.2)").unwrap();
        match geo {
            Geometry::Point(lat, lon) => {
                assert!((lat - 50.2).abs() < 1e-9);
                assert!((lon - 30.5).abs() < 1e-9);
            }
            _ => panic!("expected Point"),
        }
    }

    #[test]
    fn test_wkt_quoted() {
        let geo = parse_geometry("\"POINT(30.5 50.2)\"").unwrap();
        match geo {
            Geometry::Point(lat, lon) => {
                assert!((lat - 50.2).abs() < 1e-9);
                assert!((lon - 30.5).abs() < 1e-9);
            }
            _ => panic!("expected Point"),
        }
    }

    #[test]
    fn test_comma_lat_lon() {
        let geo = parse_geometry("51.5074, -0.1278").unwrap();
        match geo {
            Geometry::Point(lat, lon) => {
                assert!((lat - 51.5074).abs() < 1e-9);
                assert!((lon - (-0.1278)).abs() < 1e-9);
            }
            _ => panic!("expected Point"),
        }
    }

    #[test]
    fn test_wkt_linestring() {
        let geo =
            parse_geometry("LINESTRING(-34.856 -7.982, -34.857 -7.981, -34.858 -7.980)").unwrap();
        match geo {
            Geometry::LineString(coords) => {
                assert_eq!(coords.len(), 3);
                // WKT is lon,lat — we return (lat, lon)
                assert!((coords[0].0 - (-7.982)).abs() < 1e-9);
                assert!((coords[0].1 - (-34.856)).abs() < 1e-9);
            }
            _ => panic!("expected LineString"),
        }
    }

    #[test]
    fn test_wkt_linestring_with_space() {
        let geo = parse_geometry("LINESTRING (-34.856 -7.982, -34.857 -7.981)").unwrap();
        match &geo {
            Geometry::LineString(coords) => assert_eq!(coords.len(), 2),
            _ => panic!("expected LineString"),
        }
    }

    #[test]
    fn test_linestring_single_point_rejected() {
        assert!(parse_geometry("LINESTRING(-34.856 -7.982)").is_err());
    }

    #[test]
    fn test_linestring_geojson() {
        let geo = parse_geometry("LINESTRING(-34.856 -7.982, -34.860 -7.970)").unwrap();
        let json = geo.to_geojson();
        assert!(json.contains("LineString"));
        assert!(json.contains("-34.856"));
        assert!(json.contains("-7.982"));
    }

    #[test]
    fn test_invalid_value() {
        assert!(parse_geometry("hello world").is_err());
        assert!(parse_geometry("42").is_err());
        assert!(parse_geometry("").is_err());
    }

    #[test]
    fn test_out_of_range() {
        assert!(parse_geometry("POINT(200 100)").is_err());
    }
}
