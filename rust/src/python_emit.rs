//! Enterprise Python slop (mirrors ``enterprise_python.py``).

use crate::namegen::{
    class_name, comment, docstring, method_name, new_rng, var_name, ENUM_STATUS_VALUES,
    STDLIB_IMPORTS,
};
use memchr::memchr_iter;
use rand::seq::index::sample;
use rand::Rng;

const SALT: u64 = 0x50_59_54_48_4F_4E_45_4E; // "PYTHONEN"

pub fn generate_enterprise_python(sanity: f64, file_index: i64) -> (String, usize, String) {
    let mut rng = new_rng(sanity, file_index, SALT);
    let cname = class_name(&mut rng, sanity);

    let mut out = String::with_capacity(96 * 1024);

    out.push_str(&format!("\"\"\"\n{}\n\n", docstring(&mut rng, sanity, &cname)));
    out.push_str(&format!("This module provides the {cname} implementation\n"));
    out.push_str("for enterprise-grade workflow orchestration.\n");
    out.push_str("\"\"\"\n\n");

    let k = rng.gen_range(4..=STDLIB_IMPORTS.len());
    let idx: Vec<usize> = sample(&mut rng, STDLIB_IMPORTS.len(), k).into_iter().collect();
    for i in idx {
        out.push_str(&format!("{}\n", STDLIB_IMPORTS[i]));
    }
    out.push('\n');

    out.push_str("T = TypeVar('T')\n");
    out.push_str("U = TypeVar('U')\n");
    for _ in 0..rng.gen_range(2..=5) {
        let alias_name = format!("{}Type", class_name(&mut rng, sanity));
        out.push_str(&format!(
            "{alias_name} = Union[dict[str, Any], list[Any], None]\n"
        ));
    }
    out.push('\n');
    out.push_str("logger = logging.getLogger(__name__)\n\n");

    let meta_name = format!("{}Meta", class_name(&mut rng, sanity));
    out.push_str(&format!("\nclass {meta_name}(type):\n"));
    out.push_str(&format!("    \"\"\"{}\"\"\"\n\n", docstring(&mut rng, sanity, &meta_name)));
    out.push_str("    _instances: dict[type, Any] = {}\n\n");
    out.push_str("    def __call__(cls, *args: Any, **kwargs: Any) -> Any:\n");
    out.push_str("        if cls not in cls._instances:\n");
    out.push_str("            cls._instances[cls] = super().__call__(*args, **kwargs)\n");
    out.push_str("        return cls._instances[cls]\n\n");

    let abc_name = format!("Abstract{}", class_name(&mut rng, sanity));
    out.push_str(&format!("\nclass {abc_name}(ABC):\n"));
    out.push_str(&format!("    \"\"\"{}\"\"\"\n\n", docstring(&mut rng, sanity, &abc_name)));

    let num_abstract = rng.gen_range(3..=7);
    for _ in 0..num_abstract {
        let mname = method_name(&mut rng, sanity);
        let nparams = rng.gen_range(1..4);
        let params = (0..nparams)
            .map(|_| format!("{}: Any", var_name(&mut rng, sanity)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str("    @abstractmethod\n");
        out.push_str(&format!("    def {mname}(self, {params}) -> Any:\n"));
        out.push_str(&format!("        # {}\n", comment(&mut rng, sanity)));
        out.push_str("        ...\n\n");
    }

    let enum_name = format!("{}Status", class_name(&mut rng, sanity));
    out.push_str(&format!("\nclass {enum_name}(Enum):\n"));
    out.push_str(&format!("    \"\"\"{}\"\"\"\n\n", docstring(&mut rng, sanity, &enum_name)));

    let nv = rng.gen_range(6..=15.min(ENUM_STATUS_VALUES.len()));
    let idx: Vec<usize> = sample(&mut rng, ENUM_STATUS_VALUES.len(), nv)
        .into_iter()
        .collect();
    for i in idx {
        out.push_str(&format!("    {} = auto()\n", ENUM_STATUS_VALUES[i]));
    }
    out.push('\n');

    out.push_str(&format!("\nclass {cname}({abc_name}, metaclass={meta_name}):\n"));
    out.push_str(&format!("    \"\"\"\n    {}\n\n", docstring(&mut rng, sanity, &cname)));
    for line in crate::namegen::block_comment_lines(&mut rng, sanity, "   ") {
        out.push_str(&format!("    {line}\n"));
    }
    out.push_str("    \"\"\"\n\n");

    let num_params = rng.gen_range(8..=15);
    let params_list: Vec<String> = (0..num_params)
        .map(|_| format!("{}: Any = None", var_name(&mut rng, sanity)))
        .collect();

    out.push_str("    def __init__(\n");
    out.push_str("        self,\n");
    for p in &params_list {
        out.push_str(&format!("        {p},\n"));
    }
    out.push_str("    ) -> None:\n");
    out.push_str(&format!(
        "        \"\"\"{}\"\"\"\n",
        docstring(&mut rng, sanity, "__init__")
    ));
    for p in &params_list {
        let pname = p.split(':').next().unwrap().trim();
        out.push_str(&format!("        self._{pname} = {pname}\n"));
    }
    out.push_str("        self._initialized = True\n");
    out.push_str(&format!("        self._state = {enum_name}.PENDING\n"));
    out.push_str(&format!("        logger.info(f'Initialized {cname}')\n\n"));

    for p in params_list.iter().take(5) {
        let pname = p.split(':').next().unwrap().trim();
        out.push_str("    @property\n");
        out.push_str(&format!("    def {pname}(self) -> Any:\n"));
        out.push_str(&format!("        # {}\n", comment(&mut rng, sanity)));
        out.push_str(&format!("        return self._{pname}\n\n"));
        out.push_str(&format!("    @{pname}.setter\n"));
        out.push_str(&format!("    def {pname}(self, value: Any) -> None:\n"));
        out.push_str(&format!("        self._{pname} = value\n\n"));
    }

    for _ in 0..num_abstract {
        let mname = method_name(&mut rng, sanity);
        let nparams = rng.gen_range(1..3);
        let params = (0..nparams)
            .map(|_| format!("{}: Any", var_name(&mut rng, sanity)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    def {mname}(self, {params}) -> Any:\n"));
        out.push_str(&format!("        \"\"\"{}\"\"\"\n", docstring(&mut rng, sanity, &mname)));
        let num_body = rng.gen_range(3..=8);
        for _ in 0..num_body {
            let vname = var_name(&mut rng, sanity);
            out.push_str(&format!("        {vname} = None  # {}\n", comment(&mut rng, sanity)));
        }
        out.push_str("        return None\n\n");
    }

    out.push_str("    @classmethod\n");
    out.push_str(&format!("    def create(cls, **kwargs: Any) -> '{cname}':\n"));
    out.push_str(&format!(
        "        \"\"\"{}\"\"\"\n",
        docstring(&mut rng, sanity, "create")
    ));
    out.push_str("        return cls(**kwargs)\n\n");

    out.push_str(&format!("    def __enter__(self) -> '{cname}':\n"));
    out.push_str(&format!("        self._state = {enum_name}.ACTIVE\n"));
    out.push_str("        return self\n\n");
    out.push_str("    def __exit__(self, *args: Any) -> None:\n");
    out.push_str(&format!("        self._state = {enum_name}.COMPLETED\n\n"));

    out.push_str("    def __repr__(self) -> str:\n");
    out.push_str(&format!("        return f'{cname}(state={{self._state}})'\n"));

    let lc = memchr_iter(b'\n', out.as_bytes()).count();
    (out, lc, cname)
}
