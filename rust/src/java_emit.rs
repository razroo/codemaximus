//! Enterprise Java slop (mirrors ``enterprise_java.py``).

use crate::namegen::{
    author_line, block_comment_lines, capitalize_first, class_name, comment, docstring, java_package,
    method_name, new_rng, var_name, JAVA_TYPES,
};
use memchr::memchr_iter;
use rand::seq::SliceRandom;
use rand::Rng;

const SALT: u64 = 0x4A_41_56_41_45_4E_54_45; // "JAVENTE" padding

pub fn generate_java(sanity: f64, file_index: i64) -> (String, usize, String) {
    let mut rng = new_rng(sanity, file_index, SALT);
    let cname = class_name(&mut rng, sanity);
    let pkg = java_package(&mut rng, sanity);

    let mut out = String::with_capacity(128 * 1024);
    out.push_str(&format!("package {pkg};\n\n"));

    let num_imports = rng.gen_range(5..=15);
    for _ in 0..num_imports {
        let imp_pkg = java_package(&mut rng, sanity);
        let imp_class = class_name(&mut rng, sanity);
        out.push_str(&format!("import {imp_pkg}.{imp_class};\n"));
    }
    out.push('\n');

    out.push_str("/**\n");
    out.push_str(&format!(" * {}\n", docstring(&mut rng, sanity, &cname)));
    out.push_str(&format!(" * @author {}\n", author_line(&mut rng, sanity)));
    out.push_str(" * @since 1.0.0\n");
    out.push_str(" * @deprecated Since before it was written\n");
    out.push_str(" */\n");

    let n_ext = rng.gen_range(0..=2);
    let extends: Vec<String> = (0..n_ext).map(|_| class_name(&mut rng, sanity)).collect();
    let n_impl = rng.gen_range(1..=4);
    let implements: Vec<String> = (0..n_impl).map(|_| class_name(&mut rng, sanity)).collect();

    let mut decl = format!("public class {cname}");
    if let Some(e) = extends.first() {
        decl.push_str(&format!(" extends {e}"));
    }
    if !implements.is_empty() {
        decl.push_str(&format!(" implements {}", implements.join(", ")));
    }
    out.push_str(&format!("{decl} {{\n\n"));

    let num_fields = rng.gen_range(4..=12);
    let mut fields: Vec<(String, String)> = Vec::with_capacity(num_fields);
    for _ in 0..num_fields {
        let field_name = var_name(&mut rng, sanity);
        let field_type = JAVA_TYPES.choose(&mut rng).unwrap().to_string();
        out.push_str(&format!("    private {field_type} {field_name};\n"));
        fields.push((field_type, field_name));
    }
    out.push('\n');

    let take = fields.len().min(6);
    out.push_str(&format!("    public {cname}("));
    out.push_str(
        &fields[..take]
            .iter()
            .map(|(t, n)| format!("{t} {n}"))
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push_str(") {\n");
    for (_, n) in &fields[..take] {
        out.push_str(&format!("        this.{n} = {n};\n"));
    }
    out.push_str("    }\n\n");

    for (field_type, field_name) in &fields {
        let cap = capitalize_first(field_name);
        out.push_str("    /**\n");
        out.push_str(&format!("     * Gets the {field_name}.\n"));
        out.push_str(&format!("     * @return the {field_name}\n"));
        out.push_str("     */\n");
        out.push_str(&format!("    public {field_type} get{cap}() {{\n"));
        out.push_str(&format!("        return this.{field_name};\n"));
        out.push_str(&format!("    }}\n\n"));
        out.push_str("    /**\n");
        out.push_str(&format!("     * Sets the {field_name}.\n"));
        out.push_str(&format!("     * @param {field_name} the {field_name} to set\n"));
        out.push_str("     */\n");
        out.push_str(&format!(
            "    public void set{cap}({field_type} {field_name}) {{\n"
        ));
        out.push_str(&format!("        this.{field_name} = {field_name};\n"));
        out.push_str(&format!("    }}\n\n"));
    }

    let num_methods = rng.gen_range(3..=8);
    for _ in 0..num_methods {
        let mname = method_name(&mut rng, sanity);
        let return_type: &str = ["void", "Object", "String", "boolean", "int"]
            .choose(&mut rng)
            .copied()
            .unwrap();
        let nparams = rng.gen_range(0..4);
        let params = (0..nparams)
            .map(|_| {
                format!(
                    "{} {}",
                    JAVA_TYPES.choose(&mut rng).unwrap(),
                    var_name(&mut rng, sanity)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        for line in block_comment_lines(&mut rng, sanity, "    //") {
            out.push_str(&format!("{line}\n"));
        }
        out.push_str(&format!("    public {return_type} {mname}({params}) {{\n"));

        let num_body = rng.gen_range(2..=10);
        for _ in 0..num_body {
            let vname = var_name(&mut rng, sanity);
            out.push_str(&format!(
                "        Object {vname} = null; // {}\n",
                comment(&mut rng, sanity)
            ));
        }
        match return_type {
            "void" => out.push_str(&format!("        // {}\n", comment(&mut rng, sanity))),
            "boolean" => out.push_str(&format!(
                "        return false; // {}\n",
                comment(&mut rng, sanity)
            )),
            "int" => out.push_str(&format!(
                "        return 0; // {}\n",
                comment(&mut rng, sanity)
            )),
            _ => out.push_str(&format!(
                "        return null; // {}\n",
                comment(&mut rng, sanity)
            )),
        }
        out.push_str(&format!("    }}\n\n"));
    }

    let num_inner = rng.gen_range(1..=3);
    for _ in 0..num_inner {
        let inner_name = class_name(&mut rng, sanity);
        out.push_str(&format!("    public static class {inner_name} {{\n"));
        for _ in 0..rng.gen_range(2..=5) {
            out.push_str(&format!(
                "        private Object {};\n",
                var_name(&mut rng, sanity)
            ));
        }
        out.push_str(&format!("    }}\n\n"));
    }

    out.push_str("}\n");

    let lc = memchr_iter(b'\n', out.as_bytes()).count();
    (out, lc, cname)
}
