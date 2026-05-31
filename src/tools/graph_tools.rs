use serde_json::Value;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => action_info(args),
        "bfs" => action_bfs(args),
        "dfs" => action_dfs(args),
        "shortest" | "dijkstra" | "shortest_path" => action_shortest(args),
        "topo" | "topological" | "topo_sort" => action_topo(args),
        "cycles" | "cycle" => action_cycles(args),
        "components" | "connected" => action_components(args),
        other => Err(format!(
            "graph_tools: unknown action '{other}'. Valid: info, bfs, dfs, shortest, topo, cycles, components"
        )),
    }
}

// ── Graph representation ───────────────────────────────────────────────────────

type NodeId = String;

#[derive(Debug)]
struct Graph {
    directed: bool,
    nodes: Vec<NodeId>,
    // adjacency: node -> list of (neighbor, weight)
    adj: HashMap<NodeId, Vec<(NodeId, f64)>>,
}

impl Graph {
    fn parse(args: &Value) -> Result<Self, String> {
        let directed = args
            .get("directed")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // nodes list (optional — inferred from edges if absent)
        let mut node_set: Vec<NodeId> = args
            .get("nodes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| {
                        if let Some(s) = n.as_str() {
                            Some(s.to_string())
                        } else {
                            n.get("id")
                                .or_else(|| n.get("name"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut adj: HashMap<NodeId, Vec<(NodeId, f64)>> = HashMap::new();

        // edges: array of {from, to, weight?} or [from, to] or [from, to, weight]
        let edges = args
            .get("edges")
            .and_then(|v| v.as_array())
            .ok_or("graph_tools: 'edges' array is required")?;

        for (i, edge) in edges.iter().enumerate() {
            let (from, to, weight) = if let Some(arr) = edge.as_array() {
                let from = arr
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Edge {i}: first element must be a string node name"))?
                    .to_string();
                let to = arr
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Edge {i}: second element must be a string node name"))?
                    .to_string();
                let weight = arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0);
                (from, to, weight)
            } else {
                let from = edge
                    .get("from")
                    .or_else(|| edge.get("source"))
                    .or_else(|| edge.get("src"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Edge {i}: missing 'from'"))?
                    .to_string();
                let to = edge
                    .get("to")
                    .or_else(|| edge.get("target"))
                    .or_else(|| edge.get("dst"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Edge {i}: missing 'to'"))?
                    .to_string();
                let weight = edge.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0);
                (from, to, weight)
            };

            // auto-add to node set
            if !node_set.contains(&from) {
                node_set.push(from.clone());
            }
            if !node_set.contains(&to) {
                node_set.push(to.clone());
            }

            adj.entry(from.clone())
                .or_default()
                .push((to.clone(), weight));
            if !directed {
                adj.entry(to.clone())
                    .or_default()
                    .push((from.clone(), weight));
            }
        }

        // ensure all nodes have an adjacency entry
        for n in &node_set {
            adj.entry(n.clone()).or_default();
        }

        Ok(Graph {
            directed,
            nodes: node_set,
            adj,
        })
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn edge_count(&self) -> usize {
        let total: usize = self.adj.values().map(|v| v.len()).sum();
        if self.directed {
            total
        } else {
            total / 2
        }
    }
}

// ── action: info ──────────────────────────────────────────────────────────────

fn action_info(args: &Value) -> Result<String, String> {
    let g = Graph::parse(args)?;
    let mut out = String::from("graph_tools — info\n\n");
    out.push_str(&format!(
        "  Nodes : {}\n  Edges : {}\n  Type  : {}\n\n",
        g.node_count(),
        g.edge_count(),
        if g.directed { "directed" } else { "undirected" }
    ));

    // degree stats
    let mut degree_map: BTreeMap<&str, (usize, usize)> = BTreeMap::new(); // (in, out)
    for node in &g.nodes {
        degree_map.entry(node.as_str()).or_insert((0, 0));
    }
    for (node, neighbors) in &g.adj {
        degree_map.entry(node.as_str()).or_insert((0, 0)).1 += neighbors.len();
        if g.directed {
            for (nb, _) in neighbors {
                degree_map.entry(nb.as_str()).or_insert((0, 0)).0 += 1;
            }
        }
    }

    out.push_str(&format!(
        "  {:<20}  {:>8}  {:>8}\n",
        "Node",
        if g.directed { "In-deg" } else { "Degree" },
        if g.directed { "Out-deg" } else { "" }
    ));
    out.push_str(&format!("  {:-<20}  {:-<8}  {:-<8}\n", "", "", ""));
    for (node, (ind, outd)) in &degree_map {
        if g.directed {
            out.push_str(&format!("  {:<20}  {:>8}  {:>8}\n", node, ind, outd));
        } else {
            out.push_str(&format!("  {:<20}  {:>8}\n", node, outd));
        }
    }
    Ok(out)
}

// ── action: bfs ───────────────────────────────────────────────────────────────

fn action_bfs(args: &Value) -> Result<String, String> {
    let g = Graph::parse(args)?;
    let start = args
        .get("start")
        .or_else(|| args.get("from"))
        .or_else(|| args.get("source"))
        .and_then(|v| v.as_str())
        .ok_or("graph_tools bfs: 'start' node is required")?;

    if !g.nodes.contains(&start.to_string()) {
        return Err(format!(
            "graph_tools bfs: node '{start}' not found in graph"
        ));
    }

    let mut visited: Vec<String> = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();
    let mut level: HashMap<String, usize> = HashMap::new();

    queue.push_back(start.to_string());
    seen.insert(start.to_string());
    level.insert(start.to_string(), 0);

    while let Some(node) = queue.pop_front() {
        visited.push(node.clone());
        let lv = *level.get(&node).unwrap_or(&0);
        if let Some(neighbors) = g.adj.get(&node) {
            let mut sorted_neighbors: Vec<_> = neighbors.iter().collect();
            sorted_neighbors.sort_by(|a, b| a.0.cmp(&b.0));
            for (nb, _) in sorted_neighbors {
                if !seen.contains(nb) {
                    seen.insert(nb.clone());
                    parent.insert(nb.clone(), node.clone());
                    level.insert(nb.clone(), lv + 1);
                    queue.push_back(nb.clone());
                }
            }
        }
    }

    let unreachable: Vec<&String> = g.nodes.iter().filter(|n| !seen.contains(*n)).collect();

    let mut out = format!("graph_tools — BFS from '{start}'\n\n");
    out.push_str(&format!(
        "  {:<20}  {:>6}  {:>20}\n",
        "Node", "Level", "Parent"
    ));
    out.push_str(&format!("  {:-<20}  {:-<6}  {:-<20}\n", "", "", ""));
    for node in &visited {
        let lv = level.get(node).copied().unwrap_or(0);
        let par = parent.get(node).map(|s| s.as_str()).unwrap_or("(root)");
        out.push_str(&format!("  {:<20}  {:>6}  {:<20}\n", node, lv, par));
    }
    out.push_str(&format!("\n  Visit order: {}\n", visited.join(" → ")));
    if !unreachable.is_empty() {
        let names: Vec<&str> = unreachable.iter().map(|s| s.as_str()).collect();
        out.push_str(&format!("  Unreachable: {}\n", names.join(", ")));
    }
    Ok(out)
}

// ── action: dfs ───────────────────────────────────────────────────────────────

fn dfs_visit(
    node: &str,
    g: &Graph,
    visited: &mut Vec<String>,
    seen: &mut HashSet<String>,
    parent: &mut HashMap<String, String>,
) {
    seen.insert(node.to_string());
    visited.push(node.to_string());
    if let Some(neighbors) = g.adj.get(node) {
        let mut sorted_neighbors: Vec<_> = neighbors.iter().collect();
        sorted_neighbors.sort_by(|a, b| a.0.cmp(&b.0));
        for (nb, _) in sorted_neighbors {
            if !seen.contains(nb) {
                parent.insert(nb.clone(), node.to_string());
                dfs_visit(nb, g, visited, seen, parent);
            }
        }
    }
}

fn action_dfs(args: &Value) -> Result<String, String> {
    let g = Graph::parse(args)?;
    let start = args
        .get("start")
        .or_else(|| args.get("from"))
        .or_else(|| args.get("source"))
        .and_then(|v| v.as_str())
        .ok_or("graph_tools dfs: 'start' node is required")?;

    if !g.nodes.contains(&start.to_string()) {
        return Err(format!(
            "graph_tools dfs: node '{start}' not found in graph"
        ));
    }

    let mut visited: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();

    dfs_visit(start, &g, &mut visited, &mut seen, &mut parent);

    let unreachable: Vec<&String> = g.nodes.iter().filter(|n| !seen.contains(*n)).collect();

    let mut out = format!("graph_tools — DFS from '{start}'\n\n");
    out.push_str(&format!("  {:<20}  {:>20}\n", "Node", "Parent"));
    out.push_str(&format!("  {:-<20}  {:-<20}\n", "", ""));
    for node in &visited {
        let par = parent.get(node).map(|s| s.as_str()).unwrap_or("(root)");
        out.push_str(&format!("  {:<20}  {:<20}\n", node, par));
    }
    out.push_str(&format!("\n  Visit order: {}\n", visited.join(" → ")));
    if !unreachable.is_empty() {
        let names: Vec<&str> = unreachable.iter().map(|s| s.as_str()).collect();
        out.push_str(&format!("  Unreachable: {}\n", names.join(", ")));
    }
    Ok(out)
}

// ── action: shortest (Dijkstra) ───────────────────────────────────────────────

fn action_shortest(args: &Value) -> Result<String, String> {
    let g = Graph::parse(args)?;
    let start = args
        .get("start")
        .or_else(|| args.get("from"))
        .or_else(|| args.get("source"))
        .and_then(|v| v.as_str())
        .ok_or("graph_tools shortest: 'start' node is required")?;
    let end = args
        .get("end")
        .or_else(|| args.get("to"))
        .or_else(|| args.get("target"))
        .and_then(|v| v.as_str());

    if !g.nodes.contains(&start.to_string()) {
        return Err(format!("graph_tools shortest: node '{start}' not found"));
    }
    if let Some(e) = end {
        if !g.nodes.contains(&e.to_string()) {
            return Err(format!("graph_tools shortest: node '{e}' not found"));
        }
    }

    // Dijkstra with f64 distances — use ordered_float workaround via u64 bits
    let mut dist: HashMap<String, f64> = HashMap::new();
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut heap: BinaryHeap<(Reverse<u64>, String)> = BinaryHeap::new();

    for n in &g.nodes {
        dist.insert(n.clone(), f64::INFINITY);
    }
    dist.insert(start.to_string(), 0.0);
    heap.push((Reverse(0), start.to_string()));

    while let Some((Reverse(d_bits), u)) = heap.pop() {
        let d = f64::from_bits(d_bits);
        if d > *dist.get(&u).unwrap_or(&f64::INFINITY) {
            continue;
        }
        if let Some(neighbors) = g.adj.get(&u) {
            for (v, w) in neighbors {
                let nd = d + w;
                if nd < *dist.get(v).unwrap_or(&f64::INFINITY) {
                    dist.insert(v.clone(), nd);
                    prev.insert(v.clone(), u.clone());
                    heap.push((Reverse(nd.to_bits()), v.clone()));
                }
            }
        }
    }

    let mut out = format!("graph_tools — Dijkstra from '{start}'\n\n");

    if let Some(target) = end {
        // Single target path
        let d = *dist.get(target).unwrap_or(&f64::INFINITY);
        if d.is_infinite() {
            out.push_str(&format!("  No path from '{start}' to '{target}'\n"));
        } else {
            let mut path: Vec<String> = Vec::new();
            let mut cur = target.to_string();
            loop {
                path.push(cur.clone());
                if cur == start {
                    break;
                }
                match prev.get(&cur) {
                    Some(p) => cur = p.clone(),
                    None => break,
                }
            }
            path.reverse();
            out.push_str(&format!("  Distance: {:.4}\n", d));
            out.push_str(&format!("  Path    : {}\n", path.join(" → ")));
            out.push_str(&format!("  Hops    : {}\n", path.len() - 1));
        }
    } else {
        // All-targets table
        out.push_str(&format!(
            "  {:<20}  {:>12}  {}\n",
            "Target", "Distance", "Path"
        ));
        out.push_str(&format!("  {:-<20}  {:-<12}  {:-<40}\n", "", "", ""));

        let mut sorted_nodes = g.nodes.clone();
        sorted_nodes.sort();
        for target in &sorted_nodes {
            if target == start {
                continue;
            }
            let d = *dist.get(target).unwrap_or(&f64::INFINITY);
            if d.is_infinite() {
                out.push_str(&format!("  {:<20}  {:>12}  unreachable\n", target, "∞"));
            } else {
                let mut path: Vec<String> = Vec::new();
                let mut cur = target.clone();
                loop {
                    path.push(cur.clone());
                    if &cur == start {
                        break;
                    }
                    match prev.get(&cur) {
                        Some(p) => cur = p.clone(),
                        None => break,
                    }
                }
                path.reverse();
                out.push_str(&format!(
                    "  {:<20}  {:>12.4}  {}\n",
                    target,
                    d,
                    path.join(" → ")
                ));
            }
        }
    }
    Ok(out)
}

// ── action: topo ──────────────────────────────────────────────────────────────

fn action_topo(args: &Value) -> Result<String, String> {
    let g = Graph::parse(args)?;

    // Kahn's algorithm
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for n in &g.nodes {
        in_degree.entry(n.clone()).or_insert(0);
    }
    for neighbors in g.adj.values() {
        for (nb, _) in neighbors {
            *in_degree.entry(nb.clone()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    // Sort for deterministic output
    let mut queue_vec: Vec<String> = queue.drain(..).collect();
    queue_vec.sort();
    let mut queue: VecDeque<String> = queue_vec.into();

    let mut order: Vec<String> = Vec::new();

    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        if let Some(neighbors) = g.adj.get(&node) {
            let mut sorted: Vec<_> = neighbors.iter().map(|(n, _)| n.clone()).collect();
            sorted.sort();
            for nb in sorted {
                let d = in_degree.entry(nb.clone()).or_insert(0);
                *d -= 1;
                if *d == 0 {
                    queue.push_back(nb);
                }
            }
        }
    }

    let mut out = String::from("graph_tools — Topological Sort (Kahn's algorithm)\n\n");

    if order.len() < g.nodes.len() {
        out.push_str("  ⚠ Graph contains a cycle — topological sort is not possible.\n");
        out.push_str(&format!(
            "  Processed {} of {} nodes before cycle detected.\n",
            order.len(),
            g.nodes.len()
        ));
        if !order.is_empty() {
            out.push_str(&format!("  Partial order: {}\n", order.join(" → ")));
        }
        let remaining: Vec<&String> = g.nodes.iter().filter(|n| !order.contains(n)).collect();
        let names: Vec<&str> = remaining.iter().map(|s| s.as_str()).collect();
        out.push_str(&format!("  Nodes in cycle: {}\n", names.join(", ")));
    } else {
        out.push_str(&format!("  Order: {}\n\n", order.join(" → ")));
        out.push_str("  Build/installation order (each node's dependencies come first).\n");
        out.push_str("  No cycles detected — valid DAG.\n");
    }
    Ok(out)
}

// ── action: cycles ────────────────────────────────────────────────────────────

fn action_cycles(args: &Value) -> Result<String, String> {
    let g = Graph::parse(args)?;

    // DFS-based cycle detection (directed: back-edge detection)
    enum Color {
        White,
        Gray,
        Black,
    }
    let mut color: HashMap<String, Color> =
        g.nodes.iter().map(|n| (n.clone(), Color::White)).collect();

    let mut cycle_edges: Vec<(String, String)> = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut path_set: HashSet<String> = HashSet::new();

    fn dfs_cycle(
        node: &str,
        adj: &HashMap<NodeId, Vec<(NodeId, f64)>>,
        color: &mut HashMap<String, Color>,
        stack: &mut Vec<String>,
        path_set: &mut HashSet<String>,
        cycle_edges: &mut Vec<(String, String)>,
    ) {
        *color.get_mut(node).unwrap() = Color::Gray;
        stack.push(node.to_string());
        path_set.insert(node.to_string());

        if let Some(neighbors) = adj.get(node) {
            let mut sorted: Vec<_> = neighbors.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            for (nb, _) in sorted {
                match color.get(nb).unwrap_or(&Color::Black) {
                    Color::White => {
                        dfs_cycle(nb, adj, color, stack, path_set, cycle_edges);
                    }
                    Color::Gray => {
                        // back edge — cycle found
                        cycle_edges.push((node.to_string(), nb.to_string()));
                    }
                    Color::Black => {}
                }
            }
        }

        *color.get_mut(node).unwrap() = Color::Black;
        stack.pop();
        path_set.remove(node);
    }

    let mut sorted_nodes = g.nodes.clone();
    sorted_nodes.sort();
    for node in &sorted_nodes {
        if matches!(color.get(node), Some(Color::White)) {
            dfs_cycle(
                node,
                &g.adj,
                &mut color,
                &mut stack,
                &mut path_set,
                &mut cycle_edges,
            );
        }
    }

    let mut out = String::from("graph_tools — Cycle Detection\n\n");
    if cycle_edges.is_empty() {
        out.push_str("  ✓ No cycles detected — graph is a valid DAG.\n");
    } else {
        out.push_str(&format!(
            "  ✗ {} cycle back-edge(s) found:\n\n",
            cycle_edges.len()
        ));
        for (from, to) in &cycle_edges {
            out.push_str(&format!("    {from} → {to}  (back edge)\n"));
        }
        out.push_str("\n  A back edge means the target is an ancestor of the source\n");
        out.push_str("  — removing any listed edge would break that cycle.\n");
    }
    Ok(out)
}

// ── action: components ────────────────────────────────────────────────────────

fn action_components(args: &Value) -> Result<String, String> {
    // For undirected graphs: BFS-based connected components
    // For directed graphs: Kosaraju's SCC (two-pass DFS)
    let g = Graph::parse(args)?;

    let mut out = String::from("graph_tools — Connected Components\n\n");

    if !g.directed {
        // Simple BFS components for undirected
        let mut seen: HashSet<String> = HashSet::new();
        let mut components: Vec<Vec<String>> = Vec::new();

        let mut sorted_nodes = g.nodes.clone();
        sorted_nodes.sort();

        for start in &sorted_nodes {
            if seen.contains(start) {
                continue;
            }
            let mut component: Vec<String> = Vec::new();
            let mut queue: VecDeque<String> = VecDeque::new();
            queue.push_back(start.clone());
            seen.insert(start.clone());
            while let Some(node) = queue.pop_front() {
                component.push(node.clone());
                if let Some(neighbors) = g.adj.get(&node) {
                    let mut sorted: Vec<_> = neighbors.iter().collect();
                    sorted.sort_by(|a, b| a.0.cmp(&b.0));
                    for (nb, _) in sorted {
                        if !seen.contains(nb) {
                            seen.insert(nb.clone());
                            queue.push_back(nb.clone());
                        }
                    }
                }
            }
            component.sort();
            components.push(component);
        }

        out.push_str(&format!(
            "  {} connected component(s) in undirected graph:\n\n",
            components.len()
        ));
        for (i, comp) in components.iter().enumerate() {
            out.push_str(&format!("  Component {}: {}\n", i + 1, comp.join(", ")));
        }
        if components.len() == 1 {
            out.push_str("\n  Graph is fully connected.\n");
        }
    } else {
        // Kosaraju's SCC for directed graphs
        // Pass 1: DFS on original graph, record finish order
        let mut seen: HashSet<String> = HashSet::new();
        let mut finish_stack: Vec<String> = Vec::new();

        fn dfs1(
            node: &str,
            adj: &HashMap<NodeId, Vec<(NodeId, f64)>>,
            seen: &mut HashSet<String>,
            finish_stack: &mut Vec<String>,
        ) {
            seen.insert(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                let mut sorted: Vec<_> = neighbors.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (nb, _) in sorted {
                    if !seen.contains(nb) {
                        dfs1(nb, adj, seen, finish_stack);
                    }
                }
            }
            finish_stack.push(node.to_string());
        }

        let mut sorted_nodes = g.nodes.clone();
        sorted_nodes.sort();
        for node in &sorted_nodes {
            if !seen.contains(node) {
                dfs1(node, &g.adj, &mut seen, &mut finish_stack);
            }
        }

        // Build reverse graph
        let mut rev_adj: HashMap<NodeId, Vec<(NodeId, f64)>> = HashMap::new();
        for node in &g.nodes {
            rev_adj.entry(node.clone()).or_default();
        }
        for (from, neighbors) in &g.adj {
            for (to, w) in neighbors {
                rev_adj
                    .entry(to.clone())
                    .or_default()
                    .push((from.clone(), *w));
            }
        }

        // Pass 2: DFS on reverse graph in reverse finish order
        let mut seen2: HashSet<String> = HashSet::new();
        let mut sccs: Vec<Vec<String>> = Vec::new();

        fn dfs2(
            node: &str,
            rev_adj: &HashMap<NodeId, Vec<(NodeId, f64)>>,
            seen: &mut HashSet<String>,
            component: &mut Vec<String>,
        ) {
            seen.insert(node.to_string());
            component.push(node.to_string());
            if let Some(neighbors) = rev_adj.get(node) {
                let mut sorted: Vec<_> = neighbors.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (nb, _) in sorted {
                    if !seen.contains(nb) {
                        dfs2(nb, rev_adj, seen, component);
                    }
                }
            }
        }

        while let Some(node) = finish_stack.pop() {
            if !seen2.contains(&node) {
                let mut component: Vec<String> = Vec::new();
                dfs2(&node, &rev_adj, &mut seen2, &mut component);
                component.sort();
                sccs.push(component);
            }
        }

        out.push_str(&format!(
            "  {} strongly connected component(s) in directed graph:\n\n",
            sccs.len()
        ));
        for (i, scc) in sccs.iter().enumerate() {
            let label = if scc.len() > 1 { "cycle" } else { "trivial" };
            out.push_str(&format!(
                "  SCC {}: {} nodes [{}]  {}\n",
                i + 1,
                scc.len(),
                scc.join(", "),
                label
            ));
        }
        let cycles_count = sccs.iter().filter(|s| s.len() > 1).count();
        if cycles_count == 0 {
            out.push_str("\n  No multi-node SCCs — graph is a DAG.\n");
        } else {
            out.push_str(&format!("\n  {cycles_count} SCC(s) with cycles.\n"));
        }
    }

    Ok(out)
}
