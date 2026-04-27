use serde_json::Value;

/// Performs advanced scientific and computational research tasks.
/// 
/// Modes:
/// - `symbolic`: Solve algebraic equations or simplify expressions using SymPy.
/// - `units`: Perform calculations with dimensional consistency checks.
/// - `complexity`: Empirically audit the Big-O complexity of a code block.
pub async fn scientific_compute(args: &Value) -> Result<String, String> {
    let mode = args["mode"].as_str().ok_or("Missing 'mode' (symbolic, units, complexity)")?;

    match mode {
        "symbolic" => solve_symbolic(args).await,
        "units" => verify_units(args).await,
        "complexity" => audit_complexity(args).await,
        _ => Err(format!("Unknown scientific mode: {}", mode)),
    }
}

async fn solve_symbolic(args: &Value) -> Result<String, String> {
    let expr = args["expr"].as_str().ok_or("Missing 'expr' for symbolic mode")?;
    let target = args["target"].as_str().unwrap_or("solve"); // solve, simplify, integrate, diff

    let python_script = format!(
        "import sympy\n\
         from sympy import symbols, solve, simplify, integrate, diff, Eq\n\
         # Attempt to find symbols automatically\n\
         import re\n\
         raw_expr = r\"{}\"\n\
         # Extract likely symbols (single letters or words starting with letter)\n\
         sym_names = set(re.findall(r'\\b[a-zA-Z][a-zA-Z0-9]*\\b', raw_expr))\n\
         # Remove common functions\n\
         sym_names -= {{'sin', 'cos', 'tan', 'exp', 'log', 'sqrt', 'pi', 'E', 'oo', 'solve', 'simplify', 'integrate', 'diff'}}\n\
         sym_dict = {{name: symbols(name) for name in sym_names}}\n\
         \n\
         try:\n\
             if \"=\" in raw_expr and \"{}\" == \"solve\":\n\
                 lhs, rhs = raw_expr.split(\"=\")\n\
                 result = solve(Eq(eval(lhs, {{'__builtins__': None}}, sym_dict), eval(rhs, {{'__builtins__': None}}, sym_dict)))\n\
             else:\n\
                 expr_obj = eval(raw_expr, {{'__builtins__': None}}, sym_dict)\n\
                 if \"{}\" == \"simplify\": result = simplify(expr_obj)\n\
                 elif \"{}\" == \"integrate\": result = integrate(expr_obj)\n\
                 elif \"{}\" == \"diff\": result = diff(expr_obj)\n\
                 else: result = solve(expr_obj)\n\
             print(f\"RESULT: {{result}}\")\n\
         except Exception as e:\n\
             print(f\"ERROR: {{e}}\")\n",
        expr, target, target, target, target
    );

    execute_in_sandbox(&python_script).await
}

async fn verify_units(args: &Value) -> Result<String, String> {
    let calculation = args["calculation"].as_str().ok_or("Missing 'calculation' for units mode")?;
    
    // Basic dimensional analysis wrapper for Python
    let python_script = format!(
        "try:\n\
         # Simple Unit System (SI focus)\n\
         class UnitValue:\n\
             def __init__(self, val, dims):\n\
                 self.val = val\n\
                 self.dims = dims # {{'m': 1, 's': -1, etc}}\n\
             def __add__(self, other):\n\
                 if self.dims != other.dims: raise ValueError(f\"Dimension mismatch: {{self.dims}} vs {{other.dims}}\")\n\
                 return UnitValue(self.val + other.val, self.dims)\n\
             def __mul__(self, other):\n\
                 new_dims = self.dims.copy()\n\
                 for k, v in other.dims.items(): new_dims[k] = new_dims.get(k, 0) + v\n\
                 return UnitValue(self.val * other.val, new_dims)\n\
             def __truediv__(self, other):\n\
                 new_dims = self.dims.copy()\n\
                 for k, v in other.dims.items(): new_dims[k] = new_dims.get(k, 0) - v\n\
                 return UnitValue(self.val / other.val, new_dims)\n\
             def __repr__(self): return f\"{{self.val}} ({{self.dims}})\"\n\
         \n\
         # Helper to parse strings like '10m'\n\
         def u(s):\n\
             m = __import__('re').match(r'([\\d\\.]+)([a-zA-Z]+)', s)\n\
             val = float(m.group(1))\n\
             unit = m.group(2)\n\
             return UnitValue(val, {{unit: 1}})\n\
         \n\
         # Executing the calculation with unit objects\n\
         # User input is expected to use u('10m') etc.\n\
         raw_calc = r\"{}\"\n\
         # Basic auto-wrap for units in the expression if they look like 10m\n\
         wrapped = __import__('re').sub(r'(\\d+)([a-z]+)', r\"u('\\1\\2')\", raw_calc)\n\
         result = eval(wrapped, {{'u': u}})\n\
         print(f\"RESULT: {{result}}\")\n\
         except Exception as e:\n\
         print(f\"ERROR: {{e}}\")\n",
        calculation
    );

    execute_in_sandbox(&python_script).await
}

async fn audit_complexity(args: &Value) -> Result<String, String> {
    let snippet = args["snippet"].as_str().ok_or("Missing 'snippet' for complexity mode")?;
    
    let python_script = format!(
        "import time\n\
         import math\n\
         def run_target(n):\n\
             {}\n\
         \n\
         samples = [10, 50, 100, 200, 500]\n\
         times = []\n\
         for n in samples:\n\
             start = time.perf_counter()\n\
             run_target(n)\n\
             times.append(time.perf_counter() - start)\n\
         \n\
         # Simplified regression to guess Big-O\n\
         # Compare growth rates: t/n, t/n^2, t/log(n)\n\
         ratios_n = [t/n for t, n in zip(times, samples) if n > 0]\n\
         ratios_n2 = [t/(n**2) for t, n in zip(times, samples) if n > 0]\n\
         \n\
         def variance(data):\n\
             if not data: return 1.0\n\
             avg = sum(data)/len(data)\n\
             return sum((x-avg)**2 for x in data)/len(data)\n\
         \n\
         v_n = variance(ratios_n)\n\
         v_n2 = variance(ratios_n2)\n\
         \n\
         if v_n < v_n2: complexity = \"O(N)\"\n\
         elif v_n2 < v_n: complexity = \"O(N^2)\"\n\
         else: complexity = \"O(Unknown)\"\n\
         \n\
         print(f\"RESULT: Empirically detected {{complexity}}\")\n\
         print(f\"STATS: n={{samples}}, times={{[f'{{t:.6f}}s' for t in times]}}\")\n",
        snippet.replace("\n", "\n    ")
    );

    execute_in_sandbox(&python_script).await
}

async fn execute_in_sandbox(script: &str) -> Result<String, String> {
    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script
    });
    
    // We use the internal tool call to execute
    crate::tools::code_sandbox::execute(&sandbox_args).await
}
