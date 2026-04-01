//! Fast generic-Python-shaped slop (mirrors ``generators/generic.py``).

use crate::namegen::{
    random_condition, ASSERTIONS, CHAOS_WORDS, CORP_NOUNS, CORP_PREFIXES, WRAPPER_LAYERS,
};
use crate::namegen::{class_name, comment, docstring, method_name, mix_seed, var_name};
use memchr::memchr_iter;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn nested_if_else(out: &mut String, rng: &mut ChaCha8Rng, sanity: f64, _cname: &str) {
    let c = comment(rng, sanity);
    out.push_str(&format!("# {c}\n\n\n"));
    let func_name = method_name(rng, sanity);
    let v0 = var_name(rng, sanity);
    let v1 = var_name(rng, sanity);
    let v2 = var_name(rng, sanity);
    out.push_str(&format!("def {func_name}({v0}, {v1}, {v2}):\n"));
    let ds = docstring(rng, sanity, &func_name);
    out.push_str(&format!("    \"\"\"{ds}\"\"\"\n"));

    let depth = rng.gen_range(10..=20);
    for d in 0..depth {
        let indent = "    ".repeat(d + 1);
        let var = var_name(rng, sanity);
        let condition = random_condition(rng, &var);
        out.push_str(&format!(
            "{indent}if {condition}:  # {}\n",
            comment(rng, sanity)
        ));
        out.push_str(&format!(
            "{indent}    {} = None\n",
            var_name(rng, sanity)
        ));
    }
    for d in (1..=depth).rev() {
        let indent = "    ".repeat(d);
        out.push_str(&format!("{indent}else:\n"));
        out.push_str(&format!(
            "{indent}    pass  # {}\n",
            comment(rng, sanity)
        ));
    }
    out.push_str("    return None\n\n\n");
}

fn wrapper_hell(out: &mut String, rng: &mut ChaCha8Rng, sanity: f64, _cname: &str) {
    out.push_str(&format!("# {}\n\n", comment(rng, sanity)));
    let base_name = method_name(rng, sanity);
    let num_layers = rng.gen_range(4..=WRAPPER_LAYERS.len());
    for i in 0..num_layers {
        let suffix = WRAPPER_LAYERS[i];
        let next_suffix = (i + 1 < num_layers).then(|| WRAPPER_LAYERS[i + 1]);
        let fname = format!("{base_name}{suffix}");
        let nparams = rng.gen_range(1..4);
        let params: Vec<String> = (0..nparams).map(|_| var_name(rng, sanity)).collect();
        let params_s = params.join(", ");
        out.push_str(&format!("def {fname}({params_s}):\n"));
        let wds = docstring(rng, sanity, &fname);
        out.push_str(&format!("    \"\"\"{wds}\"\"\"\n"));
        out.push_str(&format!("    # {}\n", comment(rng, sanity)));
        for _ in 0..rng.gen_range(1..3) {
            out.push_str(&format!("    {} = None\n", var_name(rng, sanity)));
        }
        if let Some(ns) = next_suffix {
            let next_name = format!("{base_name}{ns}");
            out.push_str(&format!("    return {next_name}({params_s})\n\n\n"));
        } else {
            out.push_str(&format!(
                "    return None  # {}\n\n\n",
                comment(rng, sanity)
            ));
        }
    }
}

fn massive_enum(out: &mut String, rng: &mut ChaCha8Rng, sanity: f64, cname: &str) {
    out.push_str(&format!("# {}\n", comment(rng, sanity)));
    out.push_str("from enum import Enum, auto\n\n\n");
    let enum_name = format!("{cname}Type");
    out.push_str(&format!("class {enum_name}(Enum):\n"));
    let eds = docstring(rng, sanity, &enum_name);
    out.push_str(&format!("    \"\"\"{eds}\"\"\"\n\n"));

    let num_values = rng.gen_range(50..=100);
    let mut used = std::collections::HashSet::new();
    for i in 0..num_values {
        let mut val_name = if sanity > 0.5 {
            format!(
                "{}_{}_{}",
                CORP_PREFIXES.choose(rng).unwrap().to_uppercase(),
                CORP_NOUNS.choose(rng).unwrap().to_uppercase(),
                i
            )
        } else {
            format!(
                "{}_{}",
                CHAOS_WORDS.choose(rng).unwrap().to_uppercase(),
                i
            )
        };
        if !used.insert(val_name.clone()) {
            val_name = format!("{val_name}_EXTRA_{i}");
            used.insert(val_name.clone());
        }
        out.push_str(&format!(
            "    {val_name} = auto()  # {}\n",
            comment(rng, sanity)
        ));
    }
    out.push_str("\n\n");
}

fn fake_tests(out: &mut String, rng: &mut ChaCha8Rng, sanity: f64, cname: &str) {
    out.push_str(&format!("# {}\n", comment(rng, sanity)));
    out.push_str("import unittest\n\n\n");
    let test_class = format!("Test{cname}");
    out.push_str(&format!("class {test_class}(unittest.TestCase):\n"));
    let tds = docstring(rng, sanity, &test_class);
    out.push_str(&format!("    \"\"\"{tds}\"\"\"\n\n"));

    let num_tests = rng.gen_range(10..=30);
    for i in 0..num_tests {
        let test_name = method_name(rng, sanity);
        out.push_str(&format!("    def test_{test_name}_{i}(self):\n"));
        out.push_str(&format!("        # {}\n", comment(rng, sanity)));
        let num_asserts = rng.gen_range(1..=5);
        for _ in 0..num_asserts {
            let assertion = match rng.gen_range(0..10) {
                0 => format!("self.assertTrue(True)  # {}", comment(rng, sanity)),
                j => ASSERTIONS[(j as usize - 1).min(ASSERTIONS.len() - 1)].to_string(),
            };
            out.push_str(&format!("        {assertion}\n"));
        }
        out.push('\n');
    }
    out.push_str("\nif __name__ == '__main__':\n");
    out.push_str("    unittest.main()\n\n");
}

/// Returns `(content, newline_count, class_name_for_filename)`.
pub fn generate_generic(sanity: f64, file_index: i64) -> (String, usize, String) {
    let mut rng = ChaCha8Rng::seed_from_u64(mix_seed(sanity, file_index));
    let cname = class_name(&mut rng, sanity);
    let slop = rng.gen_range(0..5);

    let mut out = String::with_capacity(48 * 1024);
    match slop {
        0 => nested_if_else(&mut out, &mut rng, sanity, &cname),
        1 => wrapper_hell(&mut out, &mut rng, sanity, &cname),
        2 => massive_enum(&mut out, &mut rng, sanity, &cname),
        3 => fake_tests(&mut out, &mut rng, sanity, &cname),
        _ => {
            nested_if_else(&mut out, &mut rng, sanity, &cname);
            wrapper_hell(&mut out, &mut rng, sanity, &cname);
            massive_enum(&mut out, &mut rng, sanity, &cname);
            fake_tests(&mut out, &mut rng, sanity, &cname);
        }
    }

    let lc = memchr_iter(b'\n', out.as_bytes()).count();
    (out, lc, cname)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_seed() {
        let (a, la, ca) = generate_generic(0.5, 42);
        let (b, lb, cb) = generate_generic(0.5, 42);
        assert_eq!(a, b);
        assert_eq!(la, lb);
        assert_eq!(ca, cb);
    }
}
