import random

from codemaximus.generators.base import SlopGenerator, GeneratedFile
from codemaximus import naming, comments
from codemaximus.native import line_count, native_generate_javascript

_TS_PRIMITIVES = (
    "string",
    "number",
    "boolean",
    "any",
    "unknown",
    "never",
    "void",
    "null | undefined",
)


class JavaScriptGenerator(SlopGenerator):
    name = "javascript"
    language = "javascript"
    extension = ".js"

    def generate(self, sanity: float, file_index: int) -> GeneratedFile:
        if native_generate_javascript is not None:
            content, lc, module_name, ext = native_generate_javascript(sanity, file_index)
            filename = f"js/{module_name}_{file_index}{ext}"
            return GeneratedFile(filename=filename, content=content, line_count=lc)

        parts: list[str] = []
        use_ts = random.random() < 0.4
        ext = ".ts" if use_ts else ".js"
        module_name = naming.class_name(sanity)

        parts.append(f"// {comments.comment(sanity)}\n")
        parts.append("'use strict';\n\n")

        num_imports = random.randint(8, 20)
        for _ in range(num_imports):
            imp_name = naming.class_name(sanity)
            if sanity < 0.3 and random.random() < 0.5:
                parts.append(
                    f"const {imp_name} = require('./{naming.class_name(sanity)}');\n"
                )
            else:
                parts.append(
                    f"import {{ {imp_name} }} from './{naming.class_name(sanity)}';\n"
                )
        parts.append("\n")

        if use_ts or sanity < 0.3:
            num_types = random.randint(3, 8)
            for _ in range(num_types):
                tname = naming.class_name(sanity) + "Type"
                if random.random() < 0.5:
                    parts.append(f"type {tname} = {{\n")
                    for _ in range(random.randint(3, 8)):
                        parts.append(
                            f"  {naming.var_name(sanity)}: {random.choice(_TS_PRIMITIVES)};\n"
                        )
                    parts.append("};\n\n")
                else:
                    inner = naming.class_name(sanity) + "Type"
                    parts.append(
                        f"type {tname} = {{ value: {inner} | null | undefined | {tname} }};\n\n"
                    )

        parts.append(f"// {comments.comment(sanity)}\n")
        switch_func = naming.method_name(sanity)
        parts.append(f"function {switch_func}(input) {{\n")
        parts.append("  switch (input) {\n")
        num_cases = random.randint(20, 50)
        for _ in range(num_cases):
            case_val = random.choice(
                [
                    f"'{naming.var_name(sanity)}'",
                    str(random.randint(0, 999)),
                    f"'{random.choice(naming.CHAOS_WORDS)}'",
                ]
            )
            parts.append(f"    case {case_val}:\n")
            parts.append(
                f"      console.log('{naming.var_name(sanity)}'); // {comments.comment(sanity)}\n"
            )
            parts.append("      break;\n")
        parts.append("    default:\n")
        parts.append(f"      return null; // {comments.comment(sanity)}\n")
        parts.append("  }\n")
        parts.append("}\n\n")

        parts.append(f"// {comments.comment(sanity)}\n")
        callback_func = naming.method_name(sanity)
        parts.append(f"function {callback_func}(callback) {{\n")
        depth = random.randint(5, 10)
        for d in range(depth):
            indent = "  " * (d + 1)
            vname = naming.var_name(sanity)
            parts.append(f"{indent}setTimeout(function() {{\n")
            parts.append(f"{indent}  var {vname} = null; // {comments.comment(sanity)}\n")
            parts.append(f"{indent}  console.log('{naming.var_name(sanity)}');\n")
        for d in range(depth, 0, -1):
            indent = "  " * d
            parts.append(f"{indent}}}, {random.randint(0, 5000)});\n")
        parts.append("}\n\n")

        parts.append(f"// {comments.comment(sanity)}\n")
        promise_func = naming.method_name(sanity)
        parts.append(f"function {promise_func}() {{\n")
        parts.append("  return new Promise((resolve, reject) => {\n")
        parts.append("    resolve(undefined);\n")
        parts.append("  })\n")
        chain_len = random.randint(5, 15)
        for _ in range(chain_len):
            vname = naming.var_name(sanity)
            parts.append(f"    .then(({vname}) => {{\n")
            parts.append(f"      // {comments.comment(sanity)}\n")
            parts.append(f"      return {vname};\n")
            parts.append("    })\n")
        parts.append("    .catch((err) => {\n")
        parts.append(f"      // {comments.comment(sanity)}\n")
        parts.append("      return null;\n")
        parts.append("    });\n")
        parts.append("}\n\n")

        parts.append(f"class {module_name} {{\n")
        parts.append("  constructor() {\n")
        num_props = random.randint(5, 12)
        for _ in range(num_props):
            parts.append(f"    this.{naming.var_name(sanity)} = null;\n")
        parts.append("  }\n\n")

        num_methods = random.randint(4, 10)
        for _ in range(num_methods):
            mname = naming.method_name(sanity)
            params = ", ".join(naming.var_name(sanity) for _ in range(random.randint(0, 4)))
            parts.append(f"  // {comments.comment(sanity)}\n")
            parts.append(f"  {mname}({params}) {{\n")
            for _ in range(random.randint(2, 6)):
                parts.append(
                    f"    const {naming.var_name(sanity)} = null; // {comments.comment(sanity)}\n"
                )
            parts.append("    return undefined;\n")
            parts.append("  }\n\n")
        parts.append("}\n\n")

        parts.append(
            f"module.exports = {{ {module_name}, {switch_func}, {callback_func}, {promise_func} }};\n"
        )

        content = "".join(parts)
        filename = f"js/{module_name}_{file_index}{ext}"
        return GeneratedFile(
            filename=filename, content=content, line_count=line_count(content)
        )
