//! Go slop (mirrors ``go_slop.py``).

use crate::namegen::{
    capitalize_first, class_name, comment, go_package, method_name, new_rng, var_name, GO_IMPORTS,
    GO_TYPES,
};
use memchr::memchr_iter;
use rand::seq::{index::sample, SliceRandom};
use rand::Rng;

const SALT: u64 = 0x47_4F_4C_41_4E_47_00_00; // "GOLANG"

pub fn generate_go(sanity: f64, file_index: i64) -> (String, usize, String) {
    let mut rng = new_rng(sanity, file_index, SALT);
    let pkg = go_package(&mut rng, sanity);
    let struct_name = class_name(&mut rng, sanity);

    let mut out = String::with_capacity(96 * 1024);
    out.push_str(&format!("package {pkg}\n\n"));

    let k = rng.gen_range(4..=10);
    let idx: Vec<usize> = sample(&mut rng, GO_IMPORTS.len(), k).into_iter().collect();
    out.push_str("import (\n");
    for &i in &idx {
        out.push_str(&format!("\t{}\n", GO_IMPORTS[i]));
    }
    out.push_str(")\n\n");

    out.push_str("// suppress unused imports\n");
    out.push_str("var (\n");
    for &i in &idx {
        let imp = GO_IMPORTS[i];
        let name = imp.trim_matches('"').split('/').last().unwrap();
        if name == "io" {
            out.push_str(&format!("\t_ = {name}.ErrClosedPipe\n"));
        }
    }
    out.push_str("\t_ = fmt.Sprintf\n");
    out.push_str("\t_ = errors.New\n");
    out.push_str(")\n\n");

    out.push_str(&format!("// {}\n", comment(&mut rng, sanity)));
    out.push_str(&format!("type {struct_name} struct {{\n"));

    let ptr_star = format!("*{}", class_name(&mut rng, sanity));
    let num_fields = rng.gen_range(10..=20);
    for _ in 0..num_fields {
        let mut fname = var_name(&mut rng, sanity);
        fname = capitalize_first(&fname);
        let ftype = if rng.gen_bool(0.15) {
            ptr_star.clone()
        } else {
            GO_TYPES.choose(&mut rng).unwrap().to_string()
        };
        let tag = format!(
            "`json:\"{}\" yaml:\"{}\" xml:\"{}\"`",
            fname.to_lowercase(),
            fname.to_lowercase(),
            fname.to_lowercase()
        );
        out.push_str(&format!("\t{fname} {ftype} {tag}\n"));
    }
    out.push_str("}\n\n");

    out.push_str(&format!("// New{struct_name} creates a new {struct_name}.\n"));
    out.push_str(&format!("// {}\n", comment(&mut rng, sanity)));
    out.push_str(&format!(
        "func New{struct_name}(ctx context.Context) (*{struct_name}, error) {{\n"
    ));
    out.push_str("\tif ctx == nil {\n");
    out.push_str(&format!(
        "\t\treturn nil, errors.New(\"{}: context cannot be nil\")\n",
        var_name(&mut rng, sanity)
    ));
    out.push_str("\t}\n");
    out.push_str("\treturn &");
    out.push_str(&struct_name);
    out.push_str("{}, nil\n");
    out.push_str("}\n\n");

    let num_methods = rng.gen_range(5..=12);
    for _ in 0..num_methods {
        let mut mname = method_name(&mut rng, sanity);
        mname = capitalize_first(&mname);
        let receiver = struct_name.chars().next().unwrap().to_lowercase().collect::<String>();
        let return_type: &str = [
            "error",
            "(interface{}, error)",
            "(string, error)",
            "(bool, error)",
            "(int, error)",
        ]
        .choose(&mut rng)
        .copied()
        .unwrap();

        out.push_str(&format!("// {mname} {}\n", comment(&mut rng, sanity)));
        out.push_str(&format!(
            "func ({receiver} *{struct_name}) {mname}(ctx context.Context) {return_type} {{\n"
        ));

        let num_checks = rng.gen_range(2..=6);
        for i in 0..num_checks {
            let err_var = if sanity < 0.5 && i > 0 {
                format!("err{i}")
            } else {
                "err".to_string()
            };
            let vname = var_name(&mut rng, sanity);
            out.push_str(&format!(
                "\t{vname}, {err_var} := func() (interface{{}}, error) {{\n"
            ));
            out.push_str(&format!("\t\t// {}\n", comment(&mut rng, sanity)));
            out.push_str("\t\treturn nil, nil\n");
            out.push_str("\t}()\n");
            out.push_str(&format!("\tif {err_var} != nil {{\n"));
            if return_type.contains("interface") || return_type.contains("string") {
                out.push_str(&format!("\t\treturn nil, {err_var}\n"));
            } else if return_type.contains("bool") {
                out.push_str(&format!("\t\treturn false, {err_var}\n"));
            } else if return_type.contains("int") {
                out.push_str(&format!("\t\treturn 0, {err_var}\n"));
            } else {
                out.push_str(&format!("\t\treturn {err_var}\n"));
            }
            out.push_str("\t}\n");
            out.push_str(&format!("\t_ = {vname} // {}\n\n", comment(&mut rng, sanity)));
        }

        match return_type {
            "error" => out.push_str("\treturn nil\n"),
            rt if rt.contains("bool") => out.push_str("\treturn false, nil\n"),
            rt if rt.contains("int") => out.push_str("\treturn 0, nil\n"),
            _ => out.push_str("\treturn nil, nil\n"),
        }
        out.push_str("}\n\n");
    }

    let num_interfaces = rng.gen_range(2..=4);
    for _ in 0..num_interfaces {
        let iname = class_name(&mut rng, sanity);
        out.push_str(&format!("// {iname} {}\n", comment(&mut rng, sanity)));
        out.push_str(&format!("type {iname} interface {{\n"));
        for _ in 0..rng.gen_range(3..=8) {
            let mut imname = method_name(&mut rng, sanity);
            imname = capitalize_first(&imname);
            out.push_str(&format!("\t{imname}(ctx context.Context) error\n"));
        }
        out.push_str("}\n\n");
    }

    let recv = struct_name.chars().next().unwrap().to_lowercase().collect::<String>();
    out.push_str(&format!("// {}\n", comment(&mut rng, sanity)));
    out.push_str(&format!(
        "func ({recv} *{struct_name}) startWorkers(ctx context.Context) {{\n"
    ));
    out.push_str("\tch := make(chan interface{}, 100)\n");
    out.push_str("\tvar wg sync.WaitGroup\n");
    let num_workers = rng.gen_range(3..=7);
    for _ in 0..num_workers {
        out.push_str("\twg.Add(1)\n");
        out.push_str("\tgo func() {\n");
        out.push_str("\t\tdefer wg.Done()\n");
        out.push_str("\t\tfor {\n");
        out.push_str("\t\t\tselect {\n");
        out.push_str("\t\t\tcase <-ctx.Done():\n");
        out.push_str("\t\t\t\treturn\n");
        out.push_str(&format!(
            "\t\t\tcase ch <- nil: // {}\n",
            comment(&mut rng, sanity)
        ));
        out.push_str("\t\t\t\ttime.Sleep(time.Millisecond)\n");
        out.push_str("\t\t\t}\n");
        out.push_str("\t\t}\n");
        out.push_str("\t}()\n\n");
    }
    out.push_str("\t_ = ch\n");
    out.push_str("\twg.Wait()\n");
    out.push_str("}\n");

    let lc = memchr_iter(b'\n', out.as_bytes()).count();
    (out, lc, struct_name)
}
