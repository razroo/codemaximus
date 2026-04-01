//! JavaScript / TypeScript slop (mirrors ``javascript.py``).

use crate::namegen::{
    class_name, comment, method_name, new_rng, var_name, CHAOS_WORDS, TS_PRIMITIVES,
};
use memchr::memchr_iter;
use rand::seq::SliceRandom;
use rand::Rng;

const SALT: u64 = 0x4A_53_53_4C_4F_50_00_00; // "JS"

pub fn generate_javascript(sanity: f64, file_index: i64) -> (String, usize, String, String) {
    let mut rng = new_rng(sanity, file_index, SALT);
    let use_ts = rng.gen_bool(0.4);
    let ext = if use_ts { ".ts" } else { ".js" };
    let module_name = class_name(&mut rng, sanity);

    let mut out = String::with_capacity(96 * 1024);

    out.push_str(&format!("// {}\n", comment(&mut rng, sanity)));
    out.push_str("'use strict';\n\n");

    let num_imports = rng.gen_range(8..=20);
    for _ in 0..num_imports {
        let imp_name = class_name(&mut rng, sanity);
        if sanity < 0.3 && rng.gen_bool(0.5) {
            out.push_str(&format!(
                "const {imp_name} = require('./{}');\n",
                class_name(&mut rng, sanity)
            ));
        } else {
            out.push_str(&format!(
                "import {{ {imp_name} }} from './{}';\n",
                class_name(&mut rng, sanity)
            ));
        }
    }
    out.push('\n');

    if use_ts || sanity < 0.3 {
        let num_types = rng.gen_range(3..=8);
        for _ in 0..num_types {
            let tname = format!("{}Type", class_name(&mut rng, sanity));
            if rng.gen_bool(0.5) {
                out.push_str(&format!("type {tname} = {{\n"));
                for _ in 0..rng.gen_range(3..=8) {
                    out.push_str(&format!(
                        "  {}: {};\n",
                        var_name(&mut rng, sanity),
                        TS_PRIMITIVES.choose(&mut rng).unwrap()
                    ));
                }
                out.push_str("};\n\n");
            } else {
                let inner = format!("{}Type", class_name(&mut rng, sanity));
                out.push_str(&format!(
                    "type {tname} = {{ value: {inner} | null | undefined | {tname} }};\n\n"
                ));
            }
        }
    }

    out.push_str(&format!("// {}\n", comment(&mut rng, sanity)));
    let switch_func = method_name(&mut rng, sanity);
    out.push_str(&format!("function {switch_func}(input) {{\n"));
    out.push_str("  switch (input) {\n");
    let num_cases = rng.gen_range(20..=50);
    for _ in 0..num_cases {
        let case_val = match rng.gen_range(0..3) {
            0 => format!("'{}'", var_name(&mut rng, sanity)),
            1 => format!("{}", rng.gen_range(0..1000)),
            _ => format!("'{}'", CHAOS_WORDS.choose(&mut rng).unwrap()),
        };
        out.push_str(&format!("    case {case_val}:\n"));
        out.push_str(&format!(
            "      console.log('{}'); // {}\n",
            var_name(&mut rng, sanity),
            comment(&mut rng, sanity)
        ));
        out.push_str("      break;\n");
    }
    out.push_str("    default:\n");
    out.push_str(&format!(
        "      return null; // {}\n",
        comment(&mut rng, sanity)
    ));
    out.push_str("  }\n");
    out.push_str("}\n\n");

    out.push_str(&format!("// {}\n", comment(&mut rng, sanity)));
    let callback_func = method_name(&mut rng, sanity);
    out.push_str(&format!("function {callback_func}(callback) {{\n"));
    let depth = rng.gen_range(5..=10);
    for d in 0..depth {
        let indent = "  ".repeat(d + 1);
        let vname = var_name(&mut rng, sanity);
        out.push_str(&format!("{indent}setTimeout(function() {{\n"));
        out.push_str(&format!(
            "{indent}  var {vname} = null; // {}\n",
            comment(&mut rng, sanity)
        ));
        out.push_str(&format!(
            "{indent}  console.log('{}');\n",
            var_name(&mut rng, sanity)
        ));
    }
    for d in (1..=depth).rev() {
        let indent = "  ".repeat(d);
        out.push_str(&format!(
            "{indent}}}, {});\n",
            rng.gen_range(0..5000)
        ));
    }
    out.push_str("}\n\n");

    out.push_str(&format!("// {}\n", comment(&mut rng, sanity)));
    let promise_func = method_name(&mut rng, sanity);
    out.push_str(&format!("function {promise_func}() {{\n"));
    out.push_str("  return new Promise((resolve, reject) => {\n");
    out.push_str("    resolve(undefined);\n");
    out.push_str("  })\n");
    let chain_len = rng.gen_range(5..=15);
    for _ in 0..chain_len {
        let vname = var_name(&mut rng, sanity);
        out.push_str(&format!("    .then(({vname}) => {{\n"));
        out.push_str(&format!("      // {}\n", comment(&mut rng, sanity)));
        out.push_str(&format!("      return {vname};\n"));
        out.push_str("    })\n");
    }
    out.push_str("    .catch((err) => {\n");
    out.push_str(&format!("      // {}\n", comment(&mut rng, sanity)));
    out.push_str("      return null;\n");
    out.push_str("    });\n");
    out.push_str("}\n\n");

    out.push_str(&format!("class {module_name} {{\n"));
    out.push_str("  constructor() {\n");
    for _ in 0..rng.gen_range(5..=12) {
        out.push_str(&format!(
            "    this.{} = null;\n",
            var_name(&mut rng, sanity)
        ));
    }
    out.push_str("  }\n\n");

    for _ in 0..rng.gen_range(4..=10) {
        let mname = method_name(&mut rng, sanity);
        let params = (0..rng.gen_range(0..4))
            .map(|_| var_name(&mut rng, sanity))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  // {}\n", comment(&mut rng, sanity)));
        out.push_str(&format!("  {mname}({params}) {{\n"));
        for _ in 0..rng.gen_range(2..=6) {
            out.push_str(&format!(
                "    const {} = null; // {}\n",
                var_name(&mut rng, sanity),
                comment(&mut rng, sanity)
            ));
        }
        out.push_str("    return undefined;\n");
        out.push_str("  }\n\n");
    }
    out.push_str("}\n\n");

    out.push_str(&format!(
        "module.exports = {{ {module_name}, {switch_func}, {callback_func}, {promise_func} }};\n"
    ));

    let lc = memchr_iter(b'\n', out.as_bytes()).count();
    (out, lc, module_name, ext.to_string())
}
