use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("distance");
    match action {
        "distance" => action_distance(args),
        "bearing" => action_bearing(args),
        "midpoint" => action_midpoint(args),
        "dms" => action_dms(args),
        "bbox" => action_bbox(args),
        "destination" => action_destination(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: distance, bearing, midpoint, dms, bbox, destination"
        )),
    }
}

fn parse_coord(args: &Value, lat_key: &str, lng_key: &str) -> Result<(f64, f64), String> {
    let lat = args
        .get(lat_key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("Missing or invalid '{lat_key}'"))?;
    let lng = args
        .get(lng_key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("Missing or invalid '{lng_key}'"))?;
    if !(-90.0..=90.0).contains(&lat) {
        return Err(format!("Latitude {lat} out of range -90..90"));
    }
    if !(-180.0..=180.0).contains(&lng) {
        return Err(format!("Longitude {lng} out of range -180..180"));
    }
    Ok((lat, lng))
}

fn haversine_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    R * c
}

fn initial_bearing_deg(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let lat1r = lat1.to_radians();
    let lat2r = lat2.to_radians();
    let dlng = (lng2 - lng1).to_radians();
    let y = dlng.sin() * lat2r.cos();
    let x = lat1r.cos() * lat2r.sin() - lat1r.sin() * lat2r.cos() * dlng.cos();
    let bearing = y.atan2(x).to_degrees();
    (bearing + 360.0) % 360.0
}

fn compass_point(bearing: f64) -> &'static str {
    let idx = ((bearing + 22.5) / 45.0) as usize % 8;
    ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][idx]
}

fn action_distance(args: &Value) -> Result<String, String> {
    let (lat1, lng1) = parse_coord(args, "lat1", "lng1")?;
    let (lat2, lng2) = parse_coord(args, "lat2", "lng2")?;

    let km = haversine_km(lat1, lng1, lat2, lng2);
    let miles = km * 0.621371;
    let nm = km * 0.539957;
    let bearing = initial_bearing_deg(lat1, lng1, lat2, lng2);

    let mut out = String::from("geo_tools — distance\n\n");
    out.push_str(&format!("From: {:.6}, {:.6}\n", lat1, lng1));
    out.push_str(&format!("To:   {:.6}, {:.6}\n\n", lat2, lng2));
    out.push_str(&format!("Distance:\n"));
    out.push_str(&format!("  {:>10.3} km\n", km));
    out.push_str(&format!("  {:>10.3} miles\n", miles));
    out.push_str(&format!("  {:>10.3} nautical miles\n\n", nm));
    out.push_str(&format!(
        "Bearing: {:.1}° {}\n",
        bearing,
        compass_point(bearing)
    ));
    Ok(out)
}

fn action_bearing(args: &Value) -> Result<String, String> {
    let (lat1, lng1) = parse_coord(args, "lat1", "lng1")?;
    let (lat2, lng2) = parse_coord(args, "lat2", "lng2")?;

    let fwd = initial_bearing_deg(lat1, lng1, lat2, lng2);
    let back = initial_bearing_deg(lat2, lng2, lat1, lng1);

    let mut out = String::from("geo_tools — bearing\n\n");
    out.push_str(&format!("From: {:.6}, {:.6}\n", lat1, lng1));
    out.push_str(&format!("To:   {:.6}, {:.6}\n\n", lat2, lng2));
    out.push_str(&format!(
        "Initial bearing (forward): {:.2}° {}\n",
        fwd,
        compass_point(fwd)
    ));
    out.push_str(&format!(
        "Final bearing (back):      {:.2}° {}\n",
        back,
        compass_point(back)
    ));
    Ok(out)
}

fn action_midpoint(args: &Value) -> Result<String, String> {
    let points_val = args.get("points");
    let mut points: Vec<(f64, f64)> = Vec::new();

    if let Some(arr) = points_val.and_then(|v| v.as_array()) {
        for item in arr {
            let lat = item
                .get(0)
                .or_else(|| item.get("lat"))
                .and_then(|v| v.as_f64())
                .ok_or("Each point needs lat (index 0 or 'lat')")?;
            let lng = item
                .get(1)
                .or_else(|| item.get("lng"))
                .and_then(|v| v.as_f64())
                .ok_or("Each point needs lng (index 1 or 'lng')")?;
            points.push((lat, lng));
        }
    } else {
        let (lat1, lng1) = parse_coord(args, "lat1", "lng1")?;
        let (lat2, lng2) = parse_coord(args, "lat2", "lng2")?;
        points.push((lat1, lng1));
        points.push((lat2, lng2));
    }

    if points.len() < 2 {
        return Err("Need at least 2 points".to_string());
    }

    // Cartesian midpoint method (handles antimeridian correctly)
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut z = 0.0f64;
    for &(lat, lng) in &points {
        let latr = lat.to_radians();
        let lngr = lng.to_radians();
        x += latr.cos() * lngr.cos();
        y += latr.cos() * lngr.sin();
        z += latr.sin();
    }
    let n = points.len() as f64;
    x /= n;
    y /= n;
    z /= n;
    let mid_lng = y.atan2(x).to_degrees();
    let hyp = (x * x + y * y).sqrt();
    let mid_lat = z.atan2(hyp).to_degrees();

    let mut out = String::from("geo_tools — midpoint\n\n");
    out.push_str(&format!("Points ({}):\n", points.len()));
    for (i, &(lat, lng)) in points.iter().enumerate() {
        out.push_str(&format!("  {}: {:.6}, {:.6}\n", i + 1, lat, lng));
    }
    out.push_str(&format!("\nMidpoint: {:.6}, {:.6}\n", mid_lat, mid_lng));
    Ok(out)
}

fn decimal_to_dms(deg: f64) -> (i32, u32, f64) {
    let abs = deg.abs();
    let d = abs as i32;
    let min_f = (abs - d as f64) * 60.0;
    let m = min_f as u32;
    let s = (min_f - m as f64) * 60.0;
    (d, m, s)
}

fn dms_to_decimal(d: f64, m: f64, s: f64, neg: bool) -> f64 {
    let dec = d.abs() + m / 60.0 + s / 3600.0;
    if neg {
        -dec
    } else {
        dec
    }
}

fn action_dms(args: &Value) -> Result<String, String> {
    // Mode A: decimal → DMS  (lat + lng as decimals)
    // Mode B: DMS → decimal  (lat_d/lat_m/lat_s/lat_dir + lng_d/lng_m/lng_s/lng_dir)
    let mut out = String::from("geo_tools — dms\n\n");

    if let (Some(lat), Some(lng)) = (
        args.get("lat").and_then(|v| v.as_f64()),
        args.get("lng").and_then(|v| v.as_f64()),
    ) {
        // Decimal → DMS
        let (ld, lm, ls) = decimal_to_dms(lat);
        let (nd, nm, ns) = decimal_to_dms(lng);
        let lat_dir = if lat >= 0.0 { "N" } else { "S" };
        let lng_dir = if lng >= 0.0 { "E" } else { "W" };
        out.push_str(&format!("Decimal: {:.6}, {:.6}\n\n", lat, lng));
        out.push_str(&format!(
            "DMS: {}°{}'{:.2}\"{}  {}°{}'{:.2}\"{}\n",
            ld, lm, ls, lat_dir, nd, nm, ns, lng_dir
        ));
        out.push_str(&format!(
            "     {}° {} {} {}  {}° {} {} {}\n",
            ld,
            lm,
            format!("{:.2}", ls),
            lat_dir,
            nd,
            nm,
            format!("{:.2}", ns),
            lng_dir
        ));
    } else {
        // DMS → decimal
        let lat_d = args
            .get("lat_d")
            .and_then(|v| v.as_f64())
            .ok_or("Missing lat_d")?;
        let lat_m = args.get("lat_m").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lat_s = args.get("lat_s").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lat_dir = args.get("lat_dir").and_then(|v| v.as_str()).unwrap_or("N");
        let lng_d = args
            .get("lng_d")
            .and_then(|v| v.as_f64())
            .ok_or("Missing lng_d")?;
        let lng_m = args.get("lng_m").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lng_s = args.get("lng_s").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lng_dir = args.get("lng_dir").and_then(|v| v.as_str()).unwrap_or("E");

        let lat = dms_to_decimal(lat_d, lat_m, lat_s, lat_dir.eq_ignore_ascii_case("S"));
        let lng = dms_to_decimal(lng_d, lng_m, lng_s, lng_dir.eq_ignore_ascii_case("W"));

        out.push_str(&format!(
            "DMS: {}°{}'{:.2}\"{}  {}°{}'{:.2}\"{}\n\n",
            lat_d as i32, lat_m as u32, lat_s, lat_dir, lng_d as i32, lng_m as u32, lng_s, lng_dir
        ));
        out.push_str(&format!("Decimal: {:.6}, {:.6}\n", lat, lng));
    }
    Ok(out)
}

fn action_bbox(args: &Value) -> Result<String, String> {
    let arr = args
        .get("points")
        .and_then(|v| v.as_array())
        .ok_or("Missing 'points' array")?;

    if arr.is_empty() {
        return Err("'points' array is empty".to_string());
    }

    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lng = f64::MAX;
    let mut max_lng = f64::MIN;

    for item in arr {
        let lat = item
            .get(0)
            .or_else(|| item.get("lat"))
            .and_then(|v| v.as_f64())
            .ok_or("Each point needs lat")?;
        let lng = item
            .get(1)
            .or_else(|| item.get("lng"))
            .and_then(|v| v.as_f64())
            .ok_or("Each point needs lng")?;
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
        min_lng = min_lng.min(lng);
        max_lng = max_lng.max(lng);
    }

    let center_lat = (min_lat + max_lat) / 2.0;
    let center_lng = (min_lng + max_lng) / 2.0;
    let height_km = haversine_km(min_lat, center_lng, max_lat, center_lng);
    let width_km = haversine_km(center_lat, min_lng, center_lat, max_lng);

    let mut out = String::from("geo_tools — bbox\n\n");
    out.push_str(&format!("Points: {}\n\n", arr.len()));
    out.push_str(&format!("Bounding box:\n"));
    out.push_str(&format!("  North: {:.6}\n", max_lat));
    out.push_str(&format!("  South: {:.6}\n", min_lat));
    out.push_str(&format!("  East:  {:.6}\n", max_lng));
    out.push_str(&format!("  West:  {:.6}\n\n", min_lng));
    out.push_str(&format!("Center: {:.6}, {:.6}\n", center_lat, center_lng));
    out.push_str(&format!("Width:  {:.3} km\n", width_km));
    out.push_str(&format!("Height: {:.3} km\n", height_km));
    Ok(out)
}

fn action_destination(args: &Value) -> Result<String, String> {
    let (lat, lng) = parse_coord(args, "lat", "lng")?;
    let distance_km = args
        .get("distance")
        .and_then(|v| v.as_f64())
        .ok_or("Missing 'distance' (km)")?;
    let bearing_deg = args
        .get("bearing")
        .and_then(|v| v.as_f64())
        .ok_or("Missing 'bearing' (degrees)")?;

    const R: f64 = 6371.0;
    let d = distance_km / R;
    let br = bearing_deg.to_radians();
    let latr = lat.to_radians();
    let lngr = lng.to_radians();

    let dest_lat = (latr.sin() * d.cos() + latr.cos() * d.sin() * br.cos()).asin();
    let dest_lng =
        lngr + (br.sin() * d.sin() * latr.cos()).atan2(d.cos() - latr.sin() * dest_lat.sin());

    let dest_lat_deg = dest_lat.to_degrees();
    let dest_lng_deg = dest_lng.to_degrees();
    let dest_lng_norm = ((dest_lng_deg + 540.0) % 360.0) - 180.0;

    let mut out = String::from("geo_tools — destination\n\n");
    out.push_str(&format!("Origin:   {:.6}, {:.6}\n", lat, lng));
    out.push_str(&format!("Distance: {:.3} km\n", distance_km));
    out.push_str(&format!(
        "Bearing:  {:.1}° {}\n\n",
        bearing_deg,
        compass_point(bearing_deg)
    ));
    out.push_str(&format!(
        "Destination: {:.6}, {:.6}\n",
        dest_lat_deg, dest_lng_norm
    ));
    Ok(out)
}
