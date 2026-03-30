#!/usr/bin/env python3
"""
Process Natural Earth Shapefiles and convert to Rust static arrays.
Generates coastline_data.rs with detailed coastlines and lakes.
HIGH QUALITY VERSION - preserves detail for accurate maps.
"""

import struct
from pathlib import Path


def read_shp_header(f):
    """Read shapefile header"""
    f.seek(0)
    header = f.read(100)
    file_code = struct.unpack(">i", header[0:4])[0]
    if file_code != 9994:
        raise ValueError(f"Invalid shapefile: file code {file_code}")
    file_length = struct.unpack(">i", header[24:28])[0] * 2
    shape_type = struct.unpack("<i", header[32:36])[0]
    return {"file_length": file_length, "shape_type": shape_type}


def read_record_header(f):
    """Read a record header"""
    header = f.read(8)
    if len(header) < 8:
        return None
    content_length = struct.unpack(">i", header[4:8])[0] * 2
    return content_length


def read_polyline(f, content_length):
    """Read a polyline/polygon record"""
    content = f.read(content_length)
    shape_type = struct.unpack("<i", content[0:4])[0]
    if shape_type == 0:
        return None
    if shape_type not in [3, 5]:
        return None

    num_parts = struct.unpack("<i", content[36:40])[0]
    num_points = struct.unpack("<i", content[40:44])[0]
    parts = struct.unpack(f"<{num_parts}i", content[44 : 44 + num_parts * 4])

    points_start = 44 + num_parts * 4
    points_data = content[points_start : points_start + num_points * 16]
    points = []
    for i in range(num_points):
        x = struct.unpack("<d", points_data[i * 16 : i * 16 + 8])[0]
        y = struct.unpack("<d", points_data[i * 16 + 8 : i * 16 + 16])[0]
        points.append((x, y))

    segments = []
    for i, start_idx in enumerate(parts):
        end_idx = parts[i + 1] if i + 1 < len(parts) else num_points
        segment = points[start_idx:end_idx]
        if len(segment) >= 2:
            segments.append(segment)
    return segments


def parse_shapefile(filepath):
    """Parse a shapefile and return list of segments"""
    all_segments = []
    with open(filepath, "rb") as f:
        header = read_shp_header(f)
        print(f"Processing {filepath.name} (type {header['shape_type']})...")
        record_count = 0
        while True:
            content_len = read_record_header(f)
            if content_len is None:
                break
            segments = read_polyline(f, content_len)
            if segments:
                all_segments.extend(segments)
                record_count += 1
        print(f"  Read {record_count} records -> {len(all_segments)} segments")
    return all_segments


def simplify_line(points, scale_factor=3.0):
    """Simplify line allocating points proportionally to segment length.

    No minimum or maximum caps - purely proportional allocation.
    Longer segments get more points, shorter segments get fewer.

    Args:
        points: List of (lon, lat) tuples
        scale_factor: Points allocated per degree of path length (default 3.0)
    """
    if len(points) <= 2:
        return points

    # Calculate segment length (sum of distances between consecutive points)
    length = 0.0
    for i in range(1, len(points)):
        dx = points[i][0] - points[i - 1][0]
        dy = points[i][1] - points[i - 1][1]
        length += (dx * dx + dy * dy) ** 0.5

    # Allocate points proportionally to length
    allocated_points = int(length * scale_factor)

    # If we have more points than allocated, downsample
    if len(points) > allocated_points and allocated_points >= 2:
        step = len(points) / allocated_points
        result = []
        for i in range(allocated_points):
            idx = int(i * step)
            if idx < len(points):
                result.append(points[idx])
        # Always include last point
        if result[-1] != points[-1]:
            result.append(points[-1])
        return result

    # Otherwise keep all original points (don't upsample)
    return points


def filter_major_features(segments, min_points=20):
    """Filter to segments with enough points to be meaningful"""
    return [seg for seg in segments if len(seg) >= min_points]


def generate_rust_code(coastlines, lakes, borders, output_path):
    """Generate the Rust source file with HIGH QUALITY data"""

    # Keep all substantial coastlines
    print("\nProcessing coastlines...")
    major_coastlines = filter_major_features(coastlines, min_points=10)
    print(f"  Keeping {len(major_coastlines)} of {len(coastlines)} coastline segments")

    # Filter lakes to major ones only
    print("Processing lakes...")
    major_lakes = []
    for seg in lakes:
        if len(seg) < 10:
            continue
        # Calculate approximate area
        lons = [p[0] for p in seg]
        lats = [p[1] for p in seg]
        width = max(lons) - min(lons)
        height = max(lats) - min(lats)
        # Keep lakes larger than ~2000 km²
        if width * 111 * height * 111 > 2000:
            major_lakes.append(seg)
    print(f"  Keeping {len(major_lakes)} major lakes")

    # Keep all borders
    print("Processing borders...")
    major_borders = filter_major_features(borders, min_points=10)
    print(f"  Keeping {len(major_borders)} border segments")

    lines = [
        "// Auto-generated from Natural Earth 1:10m data",
        "// Sources:",
        "//   - Coastlines: ne_10m_coastline",
        "//   - Lakes: ne_10m_lakes (major lakes only)",
        "//   - Borders: ne_10m_admin_0_countries",
        "// License: Public Domain (Natural Earth)",
        "//",
        "// HIGH QUALITY: Preserved detail for accurate geographic rendering",
        "",
        "/// A geographic segment as (longitude, latitude) points",
        "pub type GeoSegment = &'static [(f64, f64)];",
        "",
    ]

    total_points = 0

    # Coastlines - keep all, simplify gently
    lines.append("/// Coastline segments (detailed)")
    lines.append("pub const COASTLINE_SEGMENTS: &[GeoSegment] = &[")
    coast_count = 0
    for seg in major_coastlines:
        simplified = simplify_line(seg, scale_factor=3.0)
        if len(simplified) >= 2:
            lines.append("    &[")
            for x, y in simplified:
                lines.append(f"        ({x:.5}, {y:.5}),")
                total_points += 1
            lines.append("    ],")
            coast_count += 1
    lines.append("];")
    lines.append("")
    print(f"Coastlines: {coast_count} segments")

    # Lakes
    lines.append("/// Major lake segments (Great Lakes, Caspian Sea, etc.)")
    lines.append("pub const LAKE_SEGMENTS: &[GeoSegment] = &[")
    lake_pts = 0
    for seg in major_lakes:
        simplified = simplify_line(seg, scale_factor=2.0)
        if len(simplified) >= 2:
            lines.append("    &[")
            for x, y in simplified:
                lines.append(f"        ({x:.5}, {y:.5}),")
                lake_pts += 1
            lines.append("    ],")
    lines.append("];")
    lines.append("")
    print(f"Lakes: {len(major_lakes)} segments, {lake_pts} points")

    total_points += lake_pts

    # Borders - subtle, single line
    lines.append("/// Country border segments")
    lines.append("pub const BORDER_SEGMENTS: &[GeoSegment] = &[")
    border_pts = 0
    for seg in major_borders:
        simplified = simplify_line(seg, scale_factor=2.5)
        if len(simplified) >= 2:
            lines.append("    &[")
            for x, y in simplified:
                lines.append(f"        ({x:.5}, {y:.5}),")
                border_pts += 1
            lines.append("    ],")
    lines.append("];")
    lines.append("")
    print(f"Borders: {len(major_borders)} segments, {border_pts} points")

    total_points += border_pts

    # Helper function
    lines.append("/// Get all geographic segments for drawing")
    lines.append("pub fn all_segments() -> Vec<GeoSegment> {")
    lines.append("    let mut all = Vec::new();")
    lines.append("    all.extend(COASTLINE_SEGMENTS);")
    lines.append("    all.extend(LAKE_SEGMENTS);")
    lines.append("    all.extend(BORDER_SEGMENTS);")
    lines.append("    all")
    lines.append("}")

    with open(output_path, "w") as f:
        f.write("\n".join(lines))

    print(f"\nGenerated: {output_path}")
    print(
        f"Total: {total_points} points ({len(major_coastlines)} coastlines, {len(major_lakes)} lakes, {len(major_borders)} borders)"
    )


if __name__ == "__main__":
    data_dir = Path("/Users/joshuawright/Projects/sitmontui/data/ne_raw")
    output_path = Path("/Users/joshuawright/Projects/sitmontui/src/coastline_data.rs")

    print("Natural Earth Data Processor - HIGH QUALITY")
    print("=" * 50)

    coastlines = parse_shapefile(data_dir / "ne_10m_coastline.shp")
    lakes = parse_shapefile(data_dir / "ne_10m_lakes.shp")
    borders = parse_shapefile(data_dir / "ne_10m_admin_0_countries.shp")

    print()
    generate_rust_code(coastlines, lakes, borders, output_path)
