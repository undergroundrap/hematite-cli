use serde_json::{json, Value};

pub fn sort_tools_schema() -> Value {
    json!({
        "name": "sort_tools",
        "description": "Sorting algorithm simulator and comparator without external utilities. Actions: sort (sort a list with the chosen algorithm and show step-by-step trace), compare (run multiple algorithms on the same list and compare step counts), analyze (classify input as random/sorted/reverse-sorted/nearly-sorted and recommend best algorithm), search (binary search with step trace on a sorted list).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["sort", "compare", "analyze", "search"],
                    "description": "Action to perform (default: sort)"
                },
                "items": {
                    "type": ["array", "string"],
                    "description": "List of numbers to sort (JSON array or space/comma-separated string)"
                },
                "algorithm": {
                    "type": "string",
                    "enum": ["bubble", "selection", "insertion", "merge", "quick", "heap", "shell", "counting", "radix"],
                    "description": "Sorting algorithm (default: quick)"
                },
                "algorithms": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of algorithms to compare (default: bubble, insertion, merge, quick)"
                },
                "target": {
                    "type": "number",
                    "description": "Value to search for (search action)"
                },
                "show_steps": {
                    "type": "boolean",
                    "description": "Show step-by-step trace (default: true; set false for large lists)"
                },
                "max_steps": {
                    "type": "integer",
                    "description": "Cap shown steps at this count (default: 20)"
                }
            }
        }
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_items(v: &Value) -> Result<Vec<f64>, String> {
    if let Some(arr) = v.as_array() {
        arr.iter()
            .map(|x| x.as_f64().ok_or_else(|| format!("non-numeric item: {x}")))
            .collect()
    } else if let Some(s) = v.as_str() {
        s.split(|c: char| c == ',' || c == ' ')
            .map(|w| w.trim())
            .filter(|w| !w.is_empty())
            .map(|w| w.parse::<f64>().map_err(|_| format!("not a number: '{w}'")))
            .collect()
    } else {
        Err("pass 'items' as a JSON array or space/comma-separated string".to_string())
    }
}

fn fmt_list(items: &[f64]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|x| {
            if x.fract() == 0.0 && x.abs() < 1e15 {
                format!("{}", *x as i64)
            } else {
                format!("{x:.4}")
            }
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

// ── Sorting algorithms with step counting ────────────────────────────────────

struct SortResult {
    sorted: Vec<f64>,
    comparisons: usize,
    swaps: usize,
    steps: Vec<String>,
}

fn bubble_sort(mut arr: Vec<f64>, max_steps: usize) -> SortResult {
    let n = arr.len();
    let mut comparisons = 0usize;
    let mut swaps = 0usize;
    let mut steps = Vec::new();
    for i in 0..n {
        let mut swapped = false;
        for j in 0..n - 1 - i {
            comparisons += 1;
            if arr[j] > arr[j + 1] {
                arr.swap(j, j + 1);
                swaps += 1;
                swapped = true;
                if steps.len() < max_steps {
                    steps.push(format!(
                        "swap {} and {} → {}",
                        arr[j + 1],
                        arr[j],
                        fmt_list(&arr)
                    ));
                }
            }
        }
        if !swapped {
            break;
        }
    }
    SortResult {
        sorted: arr,
        comparisons,
        swaps,
        steps,
    }
}

fn selection_sort(mut arr: Vec<f64>, max_steps: usize) -> SortResult {
    let n = arr.len();
    let mut comparisons = 0usize;
    let mut swaps = 0usize;
    let mut steps = Vec::new();
    for i in 0..n {
        let mut min_idx = i;
        for j in i + 1..n {
            comparisons += 1;
            if arr[j] < arr[min_idx] {
                min_idx = j;
            }
        }
        if min_idx != i {
            arr.swap(i, min_idx);
            swaps += 1;
            if steps.len() < max_steps {
                steps.push(format!(
                    "place {} at position {} → {}",
                    arr[i],
                    i,
                    fmt_list(&arr)
                ));
            }
        }
    }
    SortResult {
        sorted: arr,
        comparisons,
        swaps,
        steps,
    }
}

fn insertion_sort(mut arr: Vec<f64>, max_steps: usize) -> SortResult {
    let n = arr.len();
    let mut comparisons = 0usize;
    let mut swaps = 0usize;
    let mut steps = Vec::new();
    for i in 1..n {
        let key = arr[i];
        let mut j = i;
        while j > 0 {
            comparisons += 1;
            if arr[j - 1] > key {
                arr[j] = arr[j - 1];
                swaps += 1;
                j -= 1;
            } else {
                break;
            }
        }
        arr[j] = key;
        if steps.len() < max_steps {
            steps.push(format!(
                "insert {} at position {} → {}",
                key,
                j,
                fmt_list(&arr)
            ));
        }
    }
    SortResult {
        sorted: arr,
        comparisons,
        swaps,
        steps,
    }
}

fn merge_sort_inner(
    arr: &mut Vec<f64>,
    l: usize,
    r: usize,
    comparisons: &mut usize,
    swaps: &mut usize,
    steps: &mut Vec<String>,
    max_steps: usize,
) {
    if r - l <= 1 {
        return;
    }
    let mid = l + (r - l) / 2;
    merge_sort_inner(arr, l, mid, comparisons, swaps, steps, max_steps);
    merge_sort_inner(arr, mid, r, comparisons, swaps, steps, max_steps);
    // merge
    let left = arr[l..mid].to_vec();
    let right = arr[mid..r].to_vec();
    let (mut i, mut j, mut k) = (0, 0, l);
    while i < left.len() && j < right.len() {
        *comparisons += 1;
        if left[i] <= right[j] {
            arr[k] = left[i];
            i += 1;
        } else {
            arr[k] = right[j];
            j += 1;
            *swaps += 1;
        }
        k += 1;
    }
    while i < left.len() {
        arr[k] = left[i];
        i += 1;
        k += 1;
    }
    while j < right.len() {
        arr[k] = right[j];
        j += 1;
        k += 1;
    }
    if steps.len() < max_steps {
        steps.push(format!(
            "merged [{}..{}] → {}",
            l,
            r - 1,
            fmt_list(&arr[l..r])
        ));
    }
}

fn merge_sort(mut arr: Vec<f64>, max_steps: usize) -> SortResult {
    let n = arr.len();
    let mut comparisons = 0usize;
    let mut swaps = 0usize;
    let mut steps = Vec::new();
    merge_sort_inner(
        &mut arr,
        0,
        n,
        &mut comparisons,
        &mut swaps,
        &mut steps,
        max_steps,
    );
    SortResult {
        sorted: arr,
        comparisons,
        swaps,
        steps,
    }
}

fn quick_sort_inner(
    arr: &mut Vec<f64>,
    l: usize,
    r: usize,
    comparisons: &mut usize,
    swaps: &mut usize,
    steps: &mut Vec<String>,
    max_steps: usize,
) {
    if r <= l + 1 {
        return;
    }
    let pivot = arr[(l + r) / 2];
    let mut i = l;
    let mut j = r - 1;
    loop {
        while arr[i] < pivot {
            *comparisons += 1;
            i += 1;
        }
        while arr[j] > pivot {
            *comparisons += 1;
            if j == 0 {
                break;
            }
            j -= 1;
        }
        if i >= j {
            break;
        }
        arr.swap(i, j);
        *swaps += 1;
        if steps.len() < max_steps {
            steps.push(format!(
                "pivot={} swap pos {} and {} → {}",
                pivot,
                i,
                j,
                fmt_list(&arr[l..r])
            ));
        }
        i += 1;
        if j == 0 {
            break;
        }
        j -= 1;
    }
    if i > l {
        quick_sort_inner(arr, l, i, comparisons, swaps, steps, max_steps);
    }
    if i < r {
        quick_sort_inner(arr, i, r, comparisons, swaps, steps, max_steps);
    }
}

fn quick_sort(mut arr: Vec<f64>, max_steps: usize) -> SortResult {
    let n = arr.len();
    let mut comparisons = 0usize;
    let mut swaps = 0usize;
    let mut steps = Vec::new();
    if n > 1 {
        quick_sort_inner(
            &mut arr,
            0,
            n,
            &mut comparisons,
            &mut swaps,
            &mut steps,
            max_steps,
        );
    }
    SortResult {
        sorted: arr,
        comparisons,
        swaps,
        steps,
    }
}

fn heap_sort(mut arr: Vec<f64>, max_steps: usize) -> SortResult {
    let n = arr.len();
    let mut comparisons = 0usize;
    let mut swaps = 0usize;
    let mut steps = Vec::new();

    fn sift_down(
        arr: &mut Vec<f64>,
        start: usize,
        end: usize,
        comparisons: &mut usize,
        swaps: &mut usize,
    ) {
        let mut root = start;
        loop {
            let mut child = 2 * root + 1;
            if child >= end {
                break;
            }
            *comparisons += 1;
            if child + 1 < end && arr[child] < arr[child + 1] {
                child += 1;
            }
            *comparisons += 1;
            if arr[root] < arr[child] {
                arr.swap(root, child);
                *swaps += 1;
                root = child;
            } else {
                break;
            }
        }
    }

    // heapify
    let mut i = (n / 2) as isize - 1;
    while i >= 0 {
        sift_down(&mut arr, i as usize, n, &mut comparisons, &mut swaps);
        i -= 1;
    }
    // sort
    let mut end = n;
    while end > 1 {
        end -= 1;
        arr.swap(0, end);
        swaps += 1;
        if steps.len() < max_steps {
            steps.push(format!(
                "extract max {} → {}",
                arr[end],
                fmt_list(&arr[..end])
            ));
        }
        sift_down(&mut arr, 0, end, &mut comparisons, &mut swaps);
    }
    SortResult {
        sorted: arr,
        comparisons,
        swaps,
        steps,
    }
}

fn shell_sort(mut arr: Vec<f64>, max_steps: usize) -> SortResult {
    let n = arr.len();
    let mut comparisons = 0usize;
    let mut swaps = 0usize;
    let mut steps = Vec::new();
    let mut gap = n / 2;
    while gap > 0 {
        for i in gap..n {
            let temp = arr[i];
            let mut j = i;
            while j >= gap {
                comparisons += 1;
                if arr[j - gap] > temp {
                    arr[j] = arr[j - gap];
                    swaps += 1;
                    j -= gap;
                } else {
                    break;
                }
            }
            arr[j] = temp;
        }
        if steps.len() < max_steps {
            steps.push(format!("gap={} → {}", gap, fmt_list(&arr)));
        }
        gap /= 2;
    }
    SortResult {
        sorted: arr,
        comparisons,
        swaps,
        steps,
    }
}

fn counting_sort(arr: Vec<f64>, max_steps: usize) -> Result<SortResult, String> {
    // only works for non-negative integers
    if arr.iter().any(|x| x.fract() != 0.0 || *x < 0.0) {
        return Err("counting sort requires non-negative integers".to_string());
    }
    let max_val = arr.iter().cloned().fold(f64::NEG_INFINITY, f64::max) as usize;
    if max_val > 100_000 {
        return Err("counting sort: max value too large (>100000)".to_string());
    }
    let mut count = vec![0usize; max_val + 1];
    for &x in &arr {
        count[x as usize] += 1;
    }
    let mut steps = Vec::new();
    if steps.len() < max_steps {
        let counts: Vec<String> = count
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .map(|(v, c)| format!("{v}×{c}"))
            .collect();
        steps.push(format!("count: {}", counts.join(", ")));
    }
    let mut sorted = Vec::with_capacity(arr.len());
    for (v, &c) in count.iter().enumerate() {
        for _ in 0..c {
            sorted.push(v as f64);
        }
    }
    Ok(SortResult {
        sorted,
        comparisons: arr.len(),
        swaps: 0,
        steps,
    })
}

fn radix_sort(arr: Vec<f64>, max_steps: usize) -> Result<SortResult, String> {
    if arr.iter().any(|x| x.fract() != 0.0 || *x < 0.0) {
        return Err("radix sort requires non-negative integers".to_string());
    }
    let max_val = arr.iter().cloned().fold(0.0f64, f64::max) as u64;
    let mut data: Vec<u64> = arr.iter().map(|&x| x as u64).collect();
    let mut steps = Vec::new();
    let mut exp = 1u64;
    let mut comparisons = 0usize;
    while max_val / exp > 0 {
        let mut output = vec![0u64; data.len()];
        let mut count = [0usize; 10];
        for &x in &data {
            count[(x / exp % 10) as usize] += 1;
            comparisons += 1;
        }
        for i in 1..10 {
            count[i] += count[i - 1];
        }
        for &x in data.iter().rev() {
            let d = (x / exp % 10) as usize;
            count[d] -= 1;
            output[count[d]] = x;
        }
        data = output;
        if steps.len() < max_steps {
            let d: Vec<String> = data.iter().map(|x| x.to_string()).collect();
            steps.push(format!("digit {exp}: [{}]", d.join(", ")));
        }
        exp *= 10;
        if exp > 1_000_000_000_000 {
            break;
        }
    }
    let sorted = data.into_iter().map(|x| x as f64).collect();
    Ok(SortResult {
        sorted,
        comparisons,
        swaps: 0,
        steps,
    })
}

fn run_algorithm(name: &str, arr: Vec<f64>, max_steps: usize) -> Result<SortResult, String> {
    match name {
        "bubble" => Ok(bubble_sort(arr, max_steps)),
        "selection" => Ok(selection_sort(arr, max_steps)),
        "insertion" => Ok(insertion_sort(arr, max_steps)),
        "merge" => Ok(merge_sort(arr, max_steps)),
        "quick" => Ok(quick_sort(arr, max_steps)),
        "heap" => Ok(heap_sort(arr, max_steps)),
        "shell" => Ok(shell_sort(arr, max_steps)),
        "counting" => counting_sort(arr, max_steps),
        "radix" => radix_sort(arr, max_steps),
        other => Err(format!("unknown algorithm '{other}'; valid: bubble, selection, insertion, merge, quick, heap, shell, counting, radix")),
    }
}

fn complexity(name: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    // (best, average, worst, space)
    match name {
        "bubble" => ("O(n)", "O(n²)", "O(n²)", "O(1)"),
        "selection" => ("O(n²)", "O(n²)", "O(n²)", "O(1)"),
        "insertion" => ("O(n)", "O(n²)", "O(n²)", "O(1)"),
        "merge" => ("O(n log n)", "O(n log n)", "O(n log n)", "O(n)"),
        "quick" => ("O(n log n)", "O(n log n)", "O(n²)", "O(log n)"),
        "heap" => ("O(n log n)", "O(n log n)", "O(n log n)", "O(1)"),
        "shell" => ("O(n log n)", "O(n log²n)", "O(n²)", "O(1)"),
        "counting" => ("O(n+k)", "O(n+k)", "O(n+k)", "O(k)"),
        "radix" => ("O(nk)", "O(nk)", "O(nk)", "O(n+k)"),
        _ => ("?", "?", "?", "?"),
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_sort(args: &Value) -> Result<String, String> {
    let items_val = args.get("items").ok_or("pass 'items' list to sort")?;
    let arr = parse_items(items_val)?;
    if arr.is_empty() {
        return Err("'items' list is empty".to_string());
    }
    let algo = args
        .get("algorithm")
        .and_then(|v| v.as_str())
        .unwrap_or("quick");
    let show_steps = args
        .get("show_steps")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let max_steps = args.get("max_steps").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let n = arr.len();
    let original = fmt_list(&arr);
    let result = run_algorithm(algo, arr, if show_steps { max_steps } else { 0 })?;
    let (best, avg, worst, space) = complexity(algo);

    let mut lines = Vec::new();
    lines.push(format!("Algorithm: {} sort  |  n = {}", algo, n));
    lines.push(format!(
        "Complexity — Best: {}  Avg: {}  Worst: {}  Space: {}",
        best, avg, worst, space
    ));
    lines.push(String::new());
    lines.push(format!("Input:  {}", original));
    lines.push(format!("Output: {}", fmt_list(&result.sorted)));
    lines.push(String::new());
    lines.push(format!(
        "Comparisons: {}  |  Swaps/moves: {}",
        result.comparisons, result.swaps
    ));

    if show_steps && !result.steps.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Steps (showing {} of ~{}):",
            result.steps.len().min(max_steps),
            result.steps.len()
        ));
        for (i, step) in result.steps.iter().enumerate() {
            lines.push(format!("  {:>3}. {}", i + 1, step));
        }
        if result.steps.len() >= max_steps {
            lines.push(format!("  ... (use max_steps to see more)"));
        }
    }

    Ok(lines.join("\n"))
}

fn action_compare(args: &Value) -> Result<String, String> {
    let items_val = args.get("items").ok_or("pass 'items' list to compare")?;
    let arr = parse_items(items_val)?;
    if arr.is_empty() {
        return Err("'items' list is empty".to_string());
    }
    let default_algos = ["bubble", "insertion", "merge", "quick"];
    let algo_names: Vec<String> = if let Some(a) = args.get("algorithms").and_then(|v| v.as_array())
    {
        a.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else {
        default_algos.iter().map(|s| s.to_string()).collect()
    };
    let n = arr.len();
    let mut lines = Vec::new();
    lines.push(format!(
        "Comparing sort algorithms on n={} items: {}",
        n,
        fmt_list(&arr)
    ));
    lines.push(String::new());
    lines.push(format!(
        "{:<12}  {:>12}  {:>10}  {:>10}  {:>10}  {:>10}",
        "Algorithm", "Comparisons", "Swaps", "Best", "Average", "Worst"
    ));
    lines.push("-".repeat(72));

    let mut results: Vec<(String, usize, usize)> = Vec::new();
    for name in &algo_names {
        match run_algorithm(name, arr.clone(), 0) {
            Ok(r) => {
                let (best, avg, worst, _) = complexity(name);
                lines.push(format!(
                    "{:<12}  {:>12}  {:>10}  {:>10}  {:>10}  {:>10}",
                    name, r.comparisons, r.swaps, best, avg, worst
                ));
                results.push((name.clone(), r.comparisons, r.swaps));
            }
            Err(e) => {
                lines.push(format!("{:<12}  ERROR: {}", name, e));
            }
        }
    }

    if let Some((best_name, _, _)) = results.iter().min_by_key(|(_, c, _)| c) {
        lines.push(String::new());
        lines.push(format!("Fewest comparisons this run: {}", best_name));
    }

    Ok(lines.join("\n"))
}

fn action_analyze(args: &Value) -> Result<String, String> {
    let items_val = args.get("items").ok_or("pass 'items' list to analyze")?;
    let arr = parse_items(items_val)?;
    if arr.is_empty() {
        return Err("'items' list is empty".to_string());
    }
    let n = arr.len();
    // check sortedness
    let sorted_asc = arr.windows(2).all(|w| w[0] <= w[1]);
    let sorted_desc = arr.windows(2).all(|w| w[0] >= w[1]);
    let inversions: usize = arr
        .iter()
        .enumerate()
        .map(|(i, &a)| arr[i + 1..].iter().filter(|&&b| b < a).count())
        .sum();
    let max_inversions = n * (n - 1) / 2;
    let inv_ratio = if max_inversions == 0 {
        0.0
    } else {
        inversions as f64 / max_inversions as f64
    };
    let has_dupes =
        (1..n).any(|i| arr[i] == arr[i - 1] || arr.iter().filter(|&&x| x == arr[i]).count() > 1);

    let distribution = if sorted_asc {
        "already sorted (ascending)"
    } else if sorted_desc {
        "reverse sorted"
    } else if inv_ratio < 0.05 {
        "nearly sorted"
    } else if inv_ratio > 0.95 {
        "nearly reverse-sorted"
    } else {
        "random / unsorted"
    };

    let recommendation = if sorted_asc {
        "insertion sort (O(n) on sorted data)"
    } else if sorted_desc {
        "insertion sort or merge sort"
    } else if inv_ratio < 0.1 {
        "insertion sort (best for nearly sorted)"
    } else if has_dupes && arr.iter().all(|x| x.fract() == 0.0 && *x >= 0.0) {
        "counting sort or radix sort (integers with duplicates)"
    } else {
        "quick sort or merge sort (general purpose)"
    };

    let mut lines = Vec::new();
    lines.push(format!("Input analysis: n={}", n));
    lines.push(format!("  List:         {}", fmt_list(&arr)));
    lines.push(format!("  Distribution: {}", distribution));
    lines.push(format!(
        "  Inversions:   {} / {} ({:.1}%)",
        inversions,
        max_inversions,
        inv_ratio * 100.0
    ));
    lines.push(format!(
        "  Duplicates:   {}",
        if has_dupes { "yes" } else { "no" }
    ));
    lines.push(format!(
        "  All integers: {}",
        if arr.iter().all(|x| x.fract() == 0.0) {
            "yes"
        } else {
            "no (use merge/quick)"
        }
    ));
    lines.push(String::new());
    lines.push(format!("Recommended: {}", recommendation));

    Ok(lines.join("\n"))
}

fn action_search(args: &Value) -> Result<String, String> {
    let items_val = args
        .get("items")
        .ok_or("pass 'items' (sorted list) to search")?;
    let mut arr = parse_items(items_val)?;
    let target = args
        .get("target")
        .and_then(|v| v.as_f64())
        .ok_or("pass 'target' value to search for")?;
    arr.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut lines = Vec::new();
    lines.push(format!(
        "Binary search for {} in {}",
        target,
        fmt_list(&arr)
    ));
    lines.push(String::new());

    let mut lo = 0usize;
    let mut hi = arr.len();
    let mut step = 0usize;
    let mut found = false;
    while lo < hi {
        step += 1;
        let mid = lo + (hi - lo) / 2;
        let mid_val = arr[mid];
        lines.push(format!(
            "  Step {}: lo={} hi={} mid={} arr[mid]={}",
            step,
            lo,
            hi - 1,
            mid,
            mid_val
        ));
        if (mid_val - target).abs() < f64::EPSILON {
            lines.push(format!("  → Found {} at index {}", target, mid));
            found = true;
            break;
        } else if mid_val < target {
            lines.push(format!("  → {} < target, search right half", mid_val));
            lo = mid + 1;
        } else {
            lines.push(format!("  → {} > target, search left half", mid_val));
            hi = mid;
        }
    }
    if !found {
        lines.push(format!(
            "  → {} not found (would insert at index {})",
            target, lo
        ));
    }
    lines.push(String::new());
    lines.push(format!("Steps: {}  |  Complexity: O(log n)", step));

    Ok(lines.join("\n"))
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("sort");
    match action {
        "sort" => action_sort(args),
        "compare" => action_compare(args),
        "analyze" => action_analyze(args),
        "search" => action_search(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: sort, compare, analyze, search",
            action
        )),
    }
}
