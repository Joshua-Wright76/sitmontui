#!/usr/bin/env python3
"""
Process Natural Earth Shapefiles and convert to Rust static arrays.
Generates coastline_data.rs with low/medium/high detail map datasets.
"""

import math
import struct
from pathlib import Path


LOD_LEVELS = {
    "LOW": {"coast": 0.8, "lake": 0.6, "border": 0.7},
    "MEDIUM": {"coast": 3.0, "lake": 2.0, "border": 2.5},
    "HIGH": {"coast": 15.0, "lake": 10.0, "border": 12.5},
}


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

    # Allocate points proportionally to length, without exceeding original detail.
    allocated_points = max(2, min(len(points), int(length * scale_factor)))

    # If we have more points than allocated, downsample
    if len(points) > allocated_points:
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


def dedupe_points(points):
    """Remove consecutive duplicate points after simplification."""
    if not points:
        return points

    deduped = [points[0]]
    for point in points[1:]:
        if point != deduped[-1]:
            deduped.append(point)
    return deduped


def adaptive_min_points(scale_factor):
    """Keep fewer tiny segments in low LOD, preserve more in higher LODs."""
    return max(4, int(math.ceil(scale_factor * 3)))


def build_lod_segments(segments, scale_factor, min_points):
    """Simplify and filter segments for a specific LOD."""
    lod_segments = []
    total_points = 0

    for seg in segments:
        if len(seg) < min_points:
            continue
        simplified = dedupe_points(simplify_line(seg, scale_factor=scale_factor))
        if len(simplified) >= 2:
            lod_segments.append(simplified)
            total_points += len(simplified)

    return lod_segments, total_points


def write_segment_constant(lines, doc_comment, const_name, segments):
    """Append a Rust segment constant to the output."""
    lines.append(f"/// {doc_comment}")
    lines.append(f"pub const {const_name}: &[GeoSegment] = &[")
    for seg in segments:
        lines.append("    &[")
        for x, y in seg:
            lines.append(f"        ({x:.5}, {y:.5}),")
        lines.append("    ],")
    lines.append("];")
    lines.append("")


def filter_major_features(segments, min_points=20):
    """Filter to segments with enough points to be meaningful"""
    return [seg for seg in segments if len(seg) >= min_points]


def generate_rust_code(coastlines, lakes, borders, output_path):
    """Generate the Rust source file with fixed LOD map data."""

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
        "// Contains low, medium, and high detail datasets for zoom-based rendering",
        "",
        "/// A geographic segment as (longitude, latitude) points",
        "pub type GeoSegment = &'static [(f64, f64)];",
        "",
    ]

    total_points = 0

    for lod_name, config in LOD_LEVELS.items():
        coast_segments, coast_points = build_lod_segments(
            major_coastlines,
            scale_factor=config["coast"],
            min_points=adaptive_min_points(config["coast"]),
        )
        lake_segments, lake_points = build_lod_segments(
            major_lakes,
            scale_factor=config["lake"],
            min_points=adaptive_min_points(config["lake"]),
        )
        border_segments, border_points = build_lod_segments(
            major_borders,
            scale_factor=config["border"],
            min_points=adaptive_min_points(config["border"]),
        )

        write_segment_constant(
            lines,
            f"Coastline segments ({lod_name.lower()} detail)",
            f"COASTLINE_SEGMENTS_{lod_name}",
            coast_segments,
        )
        write_segment_constant(
            lines,
            f"Major lake segments ({lod_name.lower()} detail)",
            f"LAKE_SEGMENTS_{lod_name}",
            lake_segments,
        )
        write_segment_constant(
            lines,
            f"Country border segments ({lod_name.lower()} detail)",
            f"BORDER_SEGMENTS_{lod_name}",
            border_segments,
        )

        lod_total = coast_points + lake_points + border_points
        total_points += lod_total
        print(
            f"{lod_name}: {len(coast_segments)} coastlines/{coast_points} pts, "
            f"{len(lake_segments)} lakes/{lake_points} pts, "
            f"{len(border_segments)} borders/{border_points} pts"
        )

    with open(output_path, "w") as f:
        f.write("\n".join(lines))

    print(f"\nGenerated: {output_path}")
    print(f"Total emitted points across all LODs: {total_points}")


if __name__ == "__main__":
    data_dir = Path("/Users/joshuawright/Projects/sitmontui/data/ne_raw")
    output_path = Path("/Users/joshuawright/Projects/sitmontui/src/coastline_data.rs")

    print("Natural Earth Data Processor - FIXED LOD")
    print("=" * 50)

    coastlines = parse_shapefile(data_dir / "ne_10m_coastline.shp")
    lakes = parse_shapefile(data_dir / "ne_10m_lakes.shp")
    borders = parse_shapefile(data_dir / "ne_10m_admin_0_countries.shp")

    print()
    generate_rust_code(coastlines, lakes, borders, output_path)
