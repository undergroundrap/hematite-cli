use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("list");
    match action {
        "set" => set_action(args),
        "get" => get_action(args),
        "list" => list_action(args),
        "delete" | "del" | "remove" => delete_action(args),
        "clear" => clear_action(args),
        "keys" => keys_action(args),
        other => Err(format!(
            "keyval_tools: unknown action '{other}'. Valid: set, get, list, delete, clear, keys"
        )),
    }
}

fn kv_path(args: &Value) -> PathBuf {
    // Allow caller to specify a custom path; fall back to .hematite/kv.json
    if let Some(p) = args.get("store").and_then(|v| v.as_str()) {
        return PathBuf::from(p);
    }
    // Look for a .hematite directory walking up from cwd
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        let candidate = dir.join(".hematite");
        if candidate.is_dir() {
            return candidate.join("kv.json");
        }
        if !dir.pop() {
            break;
        }
    }
    // Fall back to ~/.hematite/kv.json
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hematite").join("kv.json")
}

fn load_store(path: &PathBuf) -> Result<BTreeMap<String, Value>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("keyval_tools: cannot read store at {}: {e}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("keyval_tools: corrupted store at {}: {e}", path.display()))
}

fn save_store(path: &PathBuf, store: &BTreeMap<String, Value>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("keyval_tools: cannot create store dir: {e}"))?;
        }
    }
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| format!("keyval_tools: serialization error: {e}"))?;
    std::fs::write(path, json).map_err(|e| {
        format!(
            "keyval_tools: cannot write store at {}: {e}",
            path.display()
        )
    })
}

fn namespace_key(args: &Value, key: &str) -> String {
    if let Some(ns) = args
        .get("namespace")
        .or_else(|| args.get("ns"))
        .and_then(|v| v.as_str())
    {
        format!("{ns}:{key}")
    } else {
        key.to_string()
    }
}

fn set_action(args: &Value) -> Result<String, String> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("keyval_tools set: 'key' required")?;
    let value = args
        .get("value")
        .ok_or("keyval_tools set: 'value' required")?
        .clone();

    let path = kv_path(args);
    let mut store = load_store(&path)?;
    let full_key = namespace_key(args, key);
    let is_new = !store.contains_key(&full_key);
    store.insert(full_key.clone(), value.clone());
    save_store(&path, &store)?;

    let val_display = match &value {
        Value::String(s) => format!("{s:?}"),
        other => other.to_string(),
    };
    Ok(format!(
        "{} '{}' = {val_display}  (store: {})\n",
        if is_new { "Created" } else { "Updated" },
        full_key,
        path.display()
    ))
}

fn get_action(args: &Value) -> Result<String, String> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("keyval_tools get: 'key' required")?;

    let path = kv_path(args);
    let store = load_store(&path)?;
    let full_key = namespace_key(args, key);

    match store.get(&full_key) {
        Some(val) => {
            let display = match val {
                Value::String(s) => s.clone(),
                other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
            };
            Ok(format!("{full_key} = {display}\n"))
        }
        None => Err(format!("keyval_tools: key '{full_key}' not found in store")),
    }
}

fn list_action(args: &Value) -> Result<String, String> {
    let path = kv_path(args);
    let store = load_store(&path)?;

    let prefix = args
        .get("prefix")
        .or_else(|| args.get("namespace").or_else(|| args.get("ns")))
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.ends_with(':') {
                s.to_string()
            } else {
                format!("{s}:")
            }
        });

    let filtered: Vec<(&String, &Value)> = if let Some(ref pre) = prefix {
        store
            .iter()
            .filter(|(k, _)| k.starts_with(pre.as_str()))
            .collect()
    } else {
        store.iter().collect()
    };

    if filtered.is_empty() {
        return Ok(format!(
            "Store is empty{}  ({})\n",
            if prefix.is_some() {
                " for this prefix"
            } else {
                ""
            },
            path.display()
        ));
    }

    let mut out = format!(
        "{} key(s) in store  ({})\n\n",
        filtered.len(),
        path.display()
    );
    for (k, v) in &filtered {
        let val_display = match v {
            Value::String(s) => {
                let snippet: String = s.chars().take(80).collect();
                if s.len() > 80 {
                    format!("{snippet:?}…")
                } else {
                    format!("{snippet:?}")
                }
            }
            other => {
                let s = other.to_string();
                let snippet: String = s.chars().take(80).collect();
                if s.len() > 80 {
                    format!("{snippet}…")
                } else {
                    snippet
                }
            }
        };
        out.push_str(&format!("  {k}  =  {val_display}\n"));
    }
    Ok(out)
}

fn delete_action(args: &Value) -> Result<String, String> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("keyval_tools delete: 'key' required")?;

    let path = kv_path(args);
    let mut store = load_store(&path)?;
    let full_key = namespace_key(args, key);

    if store.remove(&full_key).is_some() {
        save_store(&path, &store)?;
        Ok(format!("Deleted '{full_key}'\n"))
    } else {
        Err(format!("keyval_tools: key '{full_key}' not found"))
    }
}

fn clear_action(args: &Value) -> Result<String, String> {
    let path = kv_path(args);
    let mut store = load_store(&path)?;

    let prefix = args
        .get("prefix")
        .or_else(|| args.get("namespace").or_else(|| args.get("ns")))
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.ends_with(':') {
                s.to_string()
            } else {
                format!("{s}:")
            }
        });

    let count = if let Some(ref pre) = prefix {
        let before = store.len();
        store.retain(|k, _| !k.starts_with(pre.as_str()));
        before - store.len()
    } else {
        let count = store.len();
        store.clear();
        count
    };

    save_store(&path, &store)?;
    Ok(format!(
        "Cleared {count} key(s){}\n",
        if prefix.is_some() {
            " matching prefix"
        } else {
            " (entire store)"
        }
    ))
}

fn keys_action(args: &Value) -> Result<String, String> {
    let path = kv_path(args);
    let store = load_store(&path)?;

    let prefix = args.get("prefix").and_then(|v| v.as_str());

    let keys: Vec<&String> = if let Some(pre) = prefix {
        store.keys().filter(|k| k.starts_with(pre)).collect()
    } else {
        store.keys().collect()
    };

    if keys.is_empty() {
        return Ok("(no keys)\n".to_string());
    }

    let mut out = format!("{} key(s):\n", keys.len());
    for k in keys {
        out.push_str(&format!("  {k}\n"));
    }
    Ok(out)
}
